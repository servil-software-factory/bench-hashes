//! PROBE (methodology audit, November 13, 2026; never merged): does a one-message hash's speed depend on where its
//! input sits in a cache line? The benchmark's one-message cells give every contender an input at a 64-byte
//! boundary; a program's `Vec<u8>` from glibc sits 16 bytes into a line when large. Each engine hashes the same
//! bytes at offsets 0, 16 and 1 from a 64-byte boundary, call after call, in turns (0, 16, 1, 1, 16, 0, ...), timed
//! by `clocks::measure`. Prints each cell's ns a call and the offset's ratio to offset 0 over the turns.
use std::hint::black_box;

type Engine = fn(&[u8]) -> [u8; 32];

fn servil(m: &[u8]) -> [u8; 32] {
    *blake3_servil::convenience::hash(m).as_bytes()
}
fn servil_hasher(m: &[u8]) -> [u8; 32] {
    let mut h = blake3_servil::convenience::Hasher::new(blake3_servil::Mode::Plain, None);
    h.update(m);
    *h.finalize().as_bytes()
}
fn official(m: &[u8]) -> [u8; 32] {
    *blake3::hash(m).as_bytes()
}
fn sha256_ring(m: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, m).as_ref().try_into().unwrap()
}

fn mean_ns(batches: &[clocks::Batch]) -> f64 {
    batches.iter().map(|b| b.wall_ns).sum::<u64>() as f64 / batches.iter().map(|b| b.calls).sum::<u64>() as f64
}

fn main() {
    let engines: [(&str, Engine); 4] = [
        ("servil convenience::hash", servil),
        ("servil Hasher update+finalize", servil_hasher),
        ("official blake3::hash", official),
        ("SHA-256 ring", sha256_ring),
    ];
    let lengths = [64usize, 1024, 4096, 16384, 65536, 1 << 20, 16 << 20];
    let offsets = [0usize, 16, 1];
    let turns = 6;
    blake3_servil::initialize();
    println!("# align-probe v1");
    println!("engine\tbytes\toffset\tns_per_call\tratio_to_offset_0\tmin\tmax");
    for (name, engine) in engines {
        for len in lengths {
            // One kept buffer, written once; its first 64-byte boundary is offset 0.
            let mut buffer = vec![0u8; len + 256];
            for (i, b) in buffer.iter_mut().enumerate() {
                *b = (i as u64).wrapping_mul(0x9e3779b97f4a7c15).to_le_bytes()[7];
            }
            let base = buffer.as_ptr().align_offset(64);
            // The same bytes at every offset: copy the message there before its turn (untimed).
            let message: Vec<u8> = buffer[base..base + len].to_vec();
            let mut ns = vec![Vec::new(); offsets.len()];
            for turn in 0..turns {
                let order: Vec<usize> = if turn % 2 == 0 { vec![0, 1, 2] } else { vec![2, 1, 0] };
                for o in order {
                    let at = base + offsets[o];
                    buffer[at..at + len].copy_from_slice(&message);
                    let input = &buffer[at..at + len];
                    assert_eq!(input.as_ptr() as usize % 64, offsets[o]);
                    let batches = clocks::measure(6, 5_000_000, || {
                        black_box(engine(black_box(input)));
                    });
                    ns[o].push(mean_ns(&batches));
                }
            }
            for (o, offset) in offsets.iter().enumerate() {
                let ratios: Vec<f64> = (0..turns).map(|t| ns[o][t] / ns[0][t]).collect();
                let geo = (ratios.iter().map(|r| r.ln()).sum::<f64>() / turns as f64).exp();
                let mean = ns[o].iter().sum::<f64>() / turns as f64;
                println!(
                    "{name}\t{len}\t{offset}\t{mean:.2}\t{geo:.4}\t{:.4}\t{:.4}",
                    ratios.iter().cloned().fold(f64::INFINITY, f64::min),
                    ratios.iter().cloned().fold(0.0, f64::max)
                );
            }
        }
    }
    let windows = clocks::load::windows();
    let busiest = windows.iter().map(|w| w.other_milli_cpus.max(w.steal_milli_cpus)).max().unwrap_or(0);
    println!("# load: {} windows, busiest {} milli-CPUs of other programs", windows.len(), busiest);
}
