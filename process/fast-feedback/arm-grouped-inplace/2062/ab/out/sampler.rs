use blake3_servil::{convenience::Hasher, throughput, Mode};
#[cfg(any(all(feature = "main-api", feature = "next-api"), not(any(feature = "main-api", feature = "next-api"))))]
compile_error!("select exactly one finalization contract: main-api or next-api");
#[cfg(feature = "main-api")]
const FINALIZATION_CONTRACT: &str = "main: finalize_each reads; explicit reset is inside cycle/finish work";
#[cfg(feature = "next-api")]
const FINALIZATION_CONTRACT: &str = "next: finalize_each resets; no additional cycle/finish reset";
#[inline(always)]
fn finalize_and_reset(hs: &mut [Hasher], which: &[usize], out: &mut [[u8; 32]]) {
    throughput::finalize_each(hs, which, out);
    #[cfg(feature = "main-api")]
    for &i in which { hs[i].reset(None); }
}
fn core_counts(shape: &str, items: usize, algorithm: &str, calls: u64, counts: Option<clocks::Counts>) {
    if let Some(c) = counts {
        eprintln!("core_counts shape={shape} items={items} algorithm={algorithm} calls={calls} p_time_ns={} p_cycles={} p_instructions={} e_time_ns={} e_cycles={} e_instructions={}", c.p.time_ns, c.p.cycles, c.p.instructions, c.e.time_ns, c.e.cycles, c.e.instructions);
    } else {
        eprintln!("core_counts shape={shape} items={items} algorithm={algorithm} calls={calls} p_time_ns=NA p_cycles=NA p_instructions=NA e_time_ns=NA e_cycles=NA e_instructions=NA");
    }
}
fn mode<'a>(i: usize, flavor: usize, keys: &'a [[u8; 32]]) -> Mode<'a> {
    match flavor {
        0 => Mode::Plain,
        1 => Mode::Keyed(&keys[0]),
        2 => Mode::Keyed(&keys[i]),
        _ => match i % 3 {
            0 => Mode::Plain,
            1 => Mode::Keyed(&keys[i]),
            _ => Mode::DeriveKey("BLAKE3-Servil mixed held-stage probe"),
        },
    }
}
fn expected(i: usize, flavor: usize, keys: &[[u8; 32]], bytes: &[u8]) -> [u8; 32] {
    match mode(i, flavor, keys) {
        Mode::Plain => *blake3::hash(bytes).as_bytes(),
        Mode::Keyed(key) => *blake3::keyed_hash(key, bytes).as_bytes(),
        _ => blake3::derive_key("BLAKE3-Servil mixed held-stage probe", bytes),
    }
}
fn feed<'a>(hs: &mut [Hasher], data: &'a [Vec<u8>], which: &[usize], pieces: &mut Vec<(usize, &'a [u8])>, width: usize) {
    for from in (0..data[0].len()).step_by(width) {
        pieces.clear();
        for (i, bytes) in data.iter().enumerate() {
            pieces.push((which[i], &bytes[from..(from + width).min(bytes.len())]));
        }
        throughput::update_each(hs, std::hint::black_box(pieces.as_slice()));
    }
}
fn run(open: usize, active: usize, len: usize, width: usize, flavor: usize, reverse: bool) {
    let keys: Vec<[u8; 32]> = (0..open).map(|i| {
        let mut key = core::array::from_fn(|j| (j * 11 + 73) as u8);
        key[..8].copy_from_slice(&(i as u64).to_le_bytes());
        key
    }).collect();
    assert_eq!(keys.iter().collect::<std::collections::BTreeSet<_>>().len(), open);
    let which: Vec<_> = (0..active).map(|i| (i * 61) % open).collect();
    assert_eq!(which.iter().copied().collect::<std::collections::BTreeSet<_>>().len(), active);
    let mut hs: Vec<_> = (0..open).map(|i| Hasher::new(mode(i, flavor, &keys), None)).collect();
    let data: Vec<Vec<u8>> = (0..active).map(|i| (0..len).map(|j| (i * 31 + j * 17) as u8).collect()).collect();
    let wanted: Vec<_> = data.iter().enumerate().map(|(i, d)| expected(which[i], flavor, &keys, d)).collect();
    let mut out = vec![[0u8; 32]; active];
    let mut pieces = Vec::with_capacity(active);
    let shape = format!("{}_open{open}_len{len}_piece{width}", ["plain", "shared_key", "distinct_keys", "mixed_modes"][flavor]);
    eprintln!("ownership: one caller owns states/descriptors/outputs; open={open} active={active}; hasher_size={} align={} native_stride={} digest_stride=32; no concurrent sharing", std::mem::size_of::<Hasher>(), std::mem::align_of::<Hasher>(), if open > 1 { &hs[1] as *const Hasher as usize - &hs[0] as *const Hasher as usize } else { std::mem::size_of::<Hasher>() });
    eprintln!("constructors/keys/data/caller capacity outside counters; cycle includes descriptor refill, library allocations, updates, digest/reset; update includes selected reset+refill+updates, digest outside; finish prepares a cohort outside counters and consumes/resets it ONCE,64 samples,clock bookkeeping included,energy NA");
    let preflight = std::env::var_os("PINNED_RUN_PREFLIGHT").is_some_and(|v| v == "1");
    let mut algorithms = ["update", "cycle", "finish"];
    if reverse { algorithms.reverse(); }
    for algorithm in algorithms {
        let (calls, wall, cpu, cycles, instructions, energy, counts);
        if algorithm != "finish" {
            let mut work = || {
                if algorithm == "update" { for &i in &which { hs[i].reset(None); } }
                feed(&mut hs, &data, &which, &mut pieces, width);
                if algorithm == "cycle" {
                    finalize_and_reset(std::hint::black_box(&mut hs), std::hint::black_box(&which), &mut out);
                    std::hint::black_box(&out);
                } else { std::hint::black_box(&hs); }
            };
            if preflight {
                work(); calls = 1; wall = 0; cpu = 0; cycles = 0; instructions = 0; energy = "NA".into(); counts = None;
            } else {
                let trial = clocks::measure(1, 10_000_000, &mut work); calls = trial[0].calls * 10;
                let e = clocks::energy::package_nj(); let p = clocks::process_cpu_ns(); let c = clocks::Counts::read(); let t = clocks::now();
                for _ in 0..calls { work(); }
                wall = clocks::since_ns(t); let c = c.zip(clocks::Counts::read()).map(|(a, b)| b.since(a)); cpu = clocks::process_cpu_ns() - p;
                cycles = c.map_or(0, |c| c.p.cycles + c.e.cycles); instructions = c.map_or(0, |c| c.p.instructions + c.e.instructions);
                energy = e.zip(clocks::energy::package_nj()).map_or_else(|| "NA".into(), |(a, b)| (b - a).to_string());
                counts = c;
            }
            if algorithm == "update" { finalize_and_reset(&mut hs, &which, &mut out); }
        } else {
            calls = if preflight { 1 } else { 64 }; let (mut w, mut p, mut cy, mut ins) = (0, 0, 0, 0);
            let mut total_counts = Some(clocks::Counts::default());
            for _ in 0..calls {
                feed(&mut hs, &data, &which, &mut pieces, width);
                if preflight {
                    finalize_and_reset(std::hint::black_box(&mut hs), std::hint::black_box(&which), &mut out);
                } else {
                    let p0 = clocks::process_cpu_ns(); let c = clocks::Counts::read(); let t = clocks::now();
                    finalize_and_reset(std::hint::black_box(&mut hs), std::hint::black_box(&which), &mut out);
                    w += clocks::since_ns(t); let c = c.zip(clocks::Counts::read()).map(|(a, b)| b.since(a)); p += clocks::process_cpu_ns() - p0;
                    cy += c.map_or(0, |c| c.p.cycles + c.e.cycles); ins += c.map_or(0, |c| c.p.instructions + c.e.instructions);
                    total_counts = total_counts.zip(c).map(|(a, b)| a.plus(b));
                }
                assert_eq!(out, wanted, "consume-once finish canonical digest");
            }
            wall = w; cpu = p; cycles = cy; instructions = ins; energy = "NA".into();
            counts = if preflight { None } else { total_counts };
        }
        core_counts(&shape, active, algorithm, calls, counts);
        if !preflight {
            clocks::load::tick();
            println!("{shape}\t{active}\t{}\t{algorithm}\t{calls}\t{wall}\t{cpu}\t{cycles}\t{instructions}\t{energy}", data.iter().map(Vec::len).sum::<usize>());
        }
        assert_eq!(out, wanted, "{algorithm} canonical digest");
        assert!(hs.iter().all(|h| h.count() == 0), "selected states reset and idle states untouched");
    }
}
fn main() {
    blake3_servil::initialize(); let reverse = std::env::args().nth(1).as_deref() == Some("reverse");
    let preflight = std::env::var_os("PINNED_RUN_PREFLIGHT").is_some_and(|v| v == "1");
    eprintln!("finalization_contract={FINALIZATION_CONTRACT}");
    if preflight {
        eprintln!("preflight: quiet wait and timing skipped");
    } else {
        clocks::load::wait_until_quiet();
        clocks::load::tick();
    }
    println!("shape\titems\tbytes\talgorithm\tcalls\twall_ns\tcpu_ns\tcycles\tinstructions\tpackage_nj");
    for n in [2, 8, 16, 17, 24, 128, 512] { for len in [16384, 32768] { for flavor in 0..4 { run(n, n, len, 4096, flavor, reverse); } } }
    for active in [7, 8, 15, 16] { for flavor in [0, 3] { run(1024, active, 16384, 4096, flavor, reverse); } }
    if !preflight {
        let windows = clocks::load::windows();
        eprintln!("load: {}", clocks::load::describe(&windows));
        if windows.iter().any(|w| w.busy()) {
            eprintln!("busy load window: abstaining from a speed verdict");
            std::process::exit(2);
        }
    }
}

