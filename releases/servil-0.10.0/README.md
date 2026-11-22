# BLAKE3-Servil 0.10.0: measured performance

**`b3sum` from BLAKE3-Servil 0.10.0 beats official `b3sum` 1.8.7 on every workload tested on Linux** (from **1.1×** to **26× faster** across six x86-64 machines), and wins **14 of 20** workloads on an Apple M6, with 1 slower (1 MiB in memory, 7% slower).

Compared with BLAKE3-Servil 0.9.1 on Linux, `b3sum` 0.10.0 hashes a cold 1 GiB file **16%–46% faster** and hashes 1,000 in-memory 16 KiB files **7%–23% faster**.

---

## `b3sum`: BLAKE3-Servil 0.10.0 vs. official `b3sum` 1.8.7

Whole-process wall time from start to exit across six x86-64 Linux machines and one Apple M6.
**In memory** hashes files already in the OS page cache; **from disk** evicts them first so they are read from storage.

| Workload | Linux in memory (6 hosts) | Linux from disk (6 hosts) | Apple M6 in memory | Apple M6 from disk |
|---|---:|---:|---:|---:|
| 1 small file (4 KiB) | **1.9×–4.6× faster** | **1.7×–3.7× faster** | same | same |
| 1 medium file (1 MiB) | **1.5×–3.8× faster** | **1.2×–2.2× faster** | 7% slower | **1.2× faster** |
| 1 large file (1 GiB) | **1.2×–2.7× faster** | **1.2×–2.0× faster** | **1.2× faster** | **2.1× faster** |
| 1,000 small files (16 KiB each) | **3.5×–7.2× faster** | **9.4×–26× faster** | **7.4× faster** | **5.8× faster** |
| Mixed directory (1,000 files, 74 MiB) | **2.8×–7.3× faster** | **6.6×–12× faster** | **4.8× faster** | **4.8× faster** |
| 16 large files (64 MiB each) | **1.1×–4.3× faster** | **1.3×–2.0× faster** | **1.2× faster** | **1.8× faster** |

*[Full 10-workload tables and per-machine wall times (`b3sum/`)](b3sum/)*

---

## `b3sum`: what changed from 0.9.1 to 0.10.0

| Workload | Linux in memory (6 hosts) | Linux from disk (6 hosts) |
|---|---|---|
| 1 small file (4 KiB) | same | same |
| 1 medium file (1 MiB) | 7%–18% faster (4 of 6) | 6% slower (2 of 6) |
| 1 large file (1 GiB) | same | **16%–46% faster** (all 6) |
| 1,000 small files (16 KiB each) | **7%–23% faster** (all 6) | up to 45% faster (2 of 6) |
| Mixed directory (1,000 files, 74 MiB) | 8%–18% faster (4 of 6) | up to 31% faster (4 of 6); 15% slower on 2 |
| 16 large files (64 MiB each) | 12%–34% faster (5 of 6) | **13%–31% faster** (all 6) |

Outside these six rows: on the three two-socket machines, one cold 16 MiB file is 16%–28% slower than with 0.9.1, and it is 17%–31% faster on the three one-socket machines ([known costs](known-costs.md)).

*[All 10 workloads, quick suite, and 0.8.0 comparison (`b3sum/`)](b3sum/)*

---

## The Rust & C library (`blake3-servil`)

Across 7 machines (6 x86-64 Linux, one Apple M6; one benchmark binary on both sides, 5% margin), BLAKE3-Servil 0.10.0 improved on **6–80 workloads per machine** relative to 0.9.1.

- **Known cost (> 5%):** 9 multithreaded workloads were **6%–19% slower** across 3 machines (**2× EPYC 7443**, **EPYC 9255**, **EPYC 9275F**), while 4 had none beyond 5%. See every cell by name in [library/](library/) and the tracked causes in [known-costs.md](known-costs.md).

*The x86-64 library checks ran on `d3ed18d1`, the candidate stamped earlier the same day; the release commit is `2fd4ff6f`. The two differ in documents, release tools and one test, and in no shipped code.*

- **Interactive charts for every machine and workload:** [servil-software-factory.github.io/bench-hashes](https://servil-software-factory.github.io/bench-hashes/)
- **Full per-machine library breakdown (`library/`):** [library/](library/)

---

## Try it yourself

```sh
cargo install b3sum --version 1.8.7 --root official
RUSTFLAGS="-C target-feature=+crt-static" cargo install b3sums --version 0.10.0 \
    --target x86_64-unknown-linux-gnu --root servil-0.10
```

Compare them on any file with [hyperfine](https://github.com/sharkdp/hyperfine):

```sh
hyperfine -N --warmup 3 'official/bin/b3sum FILE' 'servil-0.10/bin/b3sum FILE'
```

---

## Details, methodology & raw evidence

- **Per-machine `b3sum` tables (all 10 workloads + exact ms times):** [Apple M6](b3sum/apple-m6.md), [Ryzen 9700X](b3sum/x86-9700x.md), [2× EPYC 7443](b3sum/x86-avx2.md), [EPYC 9255](b3sum/x86-epyc-9255.md), [EPYC 9275F](b3sum/x86-epyc-9275f.md), [2× Xeon Gold 6330 (b)](b3sum/x86-icelake-2.md), [2× Xeon Gold 6330 (a)](b3sum/x86-icelake.md)
- **Per-machine library tables (all cells + 3%–5% band):** [Apple M6](library/apple-m6.md), [Ryzen 9700X](library/x86-9700x.md), [2× EPYC 7443](library/x86-avx2.md), [EPYC 9255](library/x86-epyc-9255.md), [EPYC 9275F](library/x86-epyc-9275f.md), [2× Xeon Gold 6330 (b)](library/x86-icelake-2.md), [2× Xeon Gold 6330 (a)](library/x86-icelake.md)
- **Known costs and owners:** [known-costs.md](known-costs.md)
- **Machines, loads & exact commits:** [machines.md](machines.md)
- **Raw samples, scripts & checksums:** [`evidence/`](https://github.com/devonjonte/bench-hashes/tree/data/2026-11-20/servil-010/evidence) (two-socket Linux machines bound to memory node 0; 0.10.0 commit `2fd4ff6f643ef04f98f7a2e5f8ca0f096298827a`)
