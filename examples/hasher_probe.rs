//! PROBE (methodology audit, November 13, 2026; never merged): what does an incremental hasher's whole life cost
//! for a short message (make it, one update, finalize), beside the one-shot call? No cell of the benchmark times
//! that alone: its incremental cells are long messages, or a mix whose bytes are nearly all long messages'.
//! Call after call on one core, `clocks::measure`, three turns each.
use std::hint::black_box;

type Engine = fn(&[u8]) -> [u8; 32];

fn servil_hash(m: &[u8]) -> [u8; 32] {
    *blake3_servil::convenience::hash(m).as_bytes()
}
fn servil_hasher(m: &[u8]) -> [u8; 32] {
    let mut h = blake3_servil::convenience::Hasher::new(blake3_servil::Mode::Plain, None);
    h.update(m);
    *h.finalize().as_bytes()
}
fn official_hash(m: &[u8]) -> [u8; 32] {
    *blake3::hash(m).as_bytes()
}
fn official_hasher(m: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(m);
    *h.finalize().as_bytes()
}
fn ring_digest(m: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, m).as_ref().try_into().unwrap()
}
fn ring_context(m: &[u8]) -> [u8; 32] {
    let mut c = ring::digest::Context::new(&ring::digest::SHA256);
    c.update(m);
    c.finish().as_ref().try_into().unwrap()
}

fn main() {
    blake3_servil::initialize();
    let engines: [(&str, Engine); 6] = [
        ("servil convenience::hash", servil_hash),
        ("servil Hasher new+update+finalize", servil_hasher),
        ("official blake3::hash", official_hash),
        ("official Hasher new+update+finalize", official_hasher),
        ("ring SHA-256 digest", ring_digest),
        ("ring SHA-256 Context new+update+finish", ring_context),
    ];
    println!("# hasher-probe v1");
    println!("engine\tbytes\tns_per_call\tmin_turn\tmax_turn");
    for len in [1usize, 64, 256, 1024, 4096, 65536] {
        let message: Vec<u8> = (0..len).map(|i| (i as u64).wrapping_mul(0x9e3779b97f4a7c15).to_le_bytes()[7]).collect();
        let mut ns = vec![Vec::new(); engines.len()];
        for turn in 0..3 {
            for e in 0..engines.len() {
                let e = if turn % 2 == 0 { e } else { engines.len() - 1 - e };
                let engine = engines[e].1;
                let batches = clocks::measure(6, 5_000_000, || {
                    black_box(engine(black_box(&message)));
                });
                ns[e].push(batches.iter().map(|b| b.wall_ns).sum::<u64>() as f64 / batches.iter().map(|b| b.calls).sum::<u64>() as f64);
            }
        }
        for (e, (name, _)) in engines.iter().enumerate() {
            let mean = ns[e].iter().sum::<f64>() / ns[e].len() as f64;
            println!("{name}\t{len}\t{mean:.1}\t{:.1}\t{:.1}", ns[e].iter().cloned().fold(f64::INFINITY, f64::min), ns[e].iter().cloned().fold(0.0, f64::max));
        }
    }
    let windows = clocks::load::windows();
    println!("# load: {} windows, busiest {} milli-CPUs of other programs", windows.len(), windows.iter().map(|w| w.other_milli_cpus.max(w.steal_milli_cpus)).max().unwrap_or(0));
}
