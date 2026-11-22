# BLAKE3-Servil 0.10.0: measured performance

@@HEADLINE@@

---

## `b3sum`: BLAKE3-Servil 0.10.0 vs. official `b3sum` 1.8.7

Whole-process wall time from start to exit across six x86-64 Linux machines and one Apple M6.
**In memory** hashes files already in the OS page cache; **from disk** evicts them first so they are read from storage.

@@TABLE-OFFICIAL@@

*[Full 10-workload tables and per-machine wall times (`b3sum/`)](b3sum/)*

---

## `b3sum`: what changed from @@BASE@@ to 0.10.0

@@TABLE-PREV@@

Outside these six rows: on the three two-socket machines, one cold 16 MiB file is 16%–28% slower than with 0.9.1, and it is 17%–31% faster on the three one-socket machines ([known costs](known-costs.md)).

*[All 10 workloads, quick suite, and 0.8.0 comparison (`b3sum/`)](b3sum/)*

---

## The Rust & C library (`blake3-servil`)

@@LIBRARY-SUMMARY@@

- **Interactive charts for every machine and workload:** [servil-software-factory.github.io/bench-hashes](https://servil-software-factory.github.io/bench-hashes/)
- **Full per-machine library breakdown (`library/`):** [library/](library/)

---

## Try it yourself

```sh
cargo install b3sum --version 1.8.7 --root official
RUSTFLAGS="-C target-feature=+crt-static" cargo install b3sums \
    --git https://github.com/servil-software-factory/BLAKE3 \
    --rev @@COMMIT@@ \
    --target x86_64-unknown-linux-gnu --root servil-0.10
```

Compare them on any file with [hyperfine](https://github.com/sharkdp/hyperfine):

```sh
hyperfine -N --warmup 3 'official/bin/b3sum FILE' 'servil-0.10/bin/b3sum FILE'
```

---

## Details, methodology & raw evidence

- **Per-machine `b3sum` tables (all 10 workloads + exact ms times):** @@B3SUM-PAGES@@
- **Per-machine library tables (all cells + 3%–5% band):** @@LIBRARY-PAGES@@
- **Known costs and owners:** [known-costs.md](known-costs.md)
- **Machines, loads & exact commits:** [machines.md](machines.md)
- **Raw samples, scripts & checksums:** [`evidence/`](evidence/) (two-socket Linux machines bound to memory node 0; 0.10.0 commit `@@COMMIT@@`)
