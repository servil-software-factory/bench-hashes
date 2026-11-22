# b3sum on 2× Xeon Gold 6330 (a) (all 10 workloads)

Host `x86-icelake` (Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.03 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 807 μs | 818 μs (same) | 865 μs (same) | 3.40 ms (4.2× slower) |
| 64 KiB | 808 μs | 765 μs (**5% faster**) | 856 μs (6% slower) | 3.14 ms (3.9× slower) |
| 1 MiB | 1.15 ms | 1.12 ms (same) | 1.26 ms (same) | 3.32 ms (2.9× slower) |
| 16 MiB | 1.96 ms | 1.96 ms (same) | 2.04 ms (same) | 3.92 ms (2.0× slower) |
| 256 MiB | 6.23 ms | 6.22 ms (same) | 6.30 ms (same) | 11.9 ms (1.9× slower) |
| 1 GiB | 18.6 ms | 18.6 ms (same) | 18.6 ms (same) | 37.5 ms (2.0× slower) |
| 1000 x 16 KiB | 5.20 ms | 5.14 ms (same) | 4.09 ms (**21% faster**) | 20.3 ms (3.9× slower) |
| 1000 x 16 KiB, a process each | 595 ms | 568 ms (**4% faster**) | 641 ms (8% slower) | 2.91 s (4.9× slower) |
| mixed tree, 1000 files, 74 MiB | 6.84 ms | 6.92 ms (same) | 6.28 ms (**8% faster**) | 37.2 ms (5.4× slower) |
| 16 x 64 MiB | 21.0 ms | 21.7 ms (same) | 18.4 ms (**12% faster**) | 45.0 ms (2.1× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 870 μs | 884 μs (same) | 946 μs (same) | 3.51 ms (4.1× slower) |
| 64 KiB | 908 μs | 905 μs (same) | 973 μs (same) | 3.31 ms (3.7× slower) |
| 1 MiB | 1.66 ms | 1.69 ms (same) | 1.76 ms (6% slower) | 3.61 ms (2.2× slower) |
| 16 MiB | 4.45 ms | 4.40 ms (same) | 5.71 ms (28% slower) | 7.27 ms (1.6× slower) |
| 256 MiB | 48.1 ms | 48.3 ms (same) | 35.4 ms (**26% faster**) | 51.2 ms (6% slower) |
| 1 GiB | 185 ms | 184 ms (same) | 124 ms (**33% faster**) | 189 ms (same) |
| 1000 x 16 KiB | 7.12 ms | 7.00 ms (same) | 6.93 ms (same) | 65.3 ms (9.2× slower) |
| 1000 x 16 KiB, a process each | 649 ms | 621 ms (**4% faster**) | 694 ms (7% slower) | 2.95 s (4.5× slower) |
| mixed tree, 1000 files, 74 MiB | 16.1 ms | 16.4 ms (same) | 18.6 ms (15% slower) | 125 ms (7.8× slower) |
| 16 x 64 MiB | 186 ms | 186 ms (same) | 141 ms (**24% faster**) | 211 ms (13% slower) |

---

# Quick suite on 2× Xeon Gold 6330 (a) (5 rounds, includes 100 x 16 KiB)

Host `x86-icelake` (Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.00 in the busiest window. Runs: 5 rounds, each contender once on each input in each (--quick: a check, not a record).

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 810 μs | 810 μs (same) | 839 μs (same) | 3.29 ms (4.1× slower) |
| 1 MiB | 1.20 ms | 1.15 ms (same) | 1.34 ms (same) | 3.36 ms (2.8× slower) |
| 16 MiB | 2.07 ms | 1.97 ms (same) | 2.15 ms (same) | 4.05 ms (2.0× slower) |
| 100 x 16 KiB | 1.65 ms | 1.53 ms (same) | 1.80 ms (same) | 4.67 ms (2.8× slower) |
| 100 x 16 KiB, a process each | 63.3 ms | 60.3 ms (same) | 68.0 ms (7% slower) | 292 ms (4.6× slower) |
| mixed tree, 100 files, 5 MiB | 2.23 ms | 2.11 ms (same) | 2.27 ms (same) | 6.34 ms (2.8× slower) |
| 4 x 16 MiB | 3.97 ms | 3.68 ms (same) | 3.09 ms (same) | 6.96 ms (1.8× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 871 μs | 836 μs (same) | 896 μs (same) | 3.26 ms (3.8× slower) |
| 1 MiB | 1.65 ms | 1.68 ms (same) | 1.84 ms (same) | 3.66 ms (2.2× slower) |
| 16 MiB | 4.40 ms | 4.24 ms (same) | 5.56 ms (26% slower) | 7.07 ms (1.6× slower) |
| 100 x 16 KiB | 1.86 ms | 1.82 ms (same) | 2.47 ms (32% slower) | 9.35 ms (5.0× slower) |
| 100 x 16 KiB, a process each | 68.4 ms | 65.3 ms (same) | 73.6 ms (8% slower) | 297 ms (4.3× slower) |
| mixed tree, 100 files, 5 MiB | 3.27 ms | 3.45 ms (same) | 3.81 ms (17% slower) | 14.2 ms (4.4× slower) |
| 4 x 16 MiB | 13.7 ms | 13.4 ms (same) | 12.4 ms (**9% faster**) | 20.0 ms (46% slower) |

---

# Official b3sum 1.8.7 vs BLAKE3-Servil 0.10.0 on 2× Xeon Gold 6330 (a)

Host `x86-icelake` (Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.01 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.10.0.*

| Workload | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|
| 4 KiB | 903 μs | 3.44 ms (3.8× slower) |
| 64 KiB | 853 μs | 3.12 ms (3.7× slower) |
| 1 MiB | 1.33 ms | 3.33 ms (2.6× slower) |
| 16 MiB | 1.99 ms | 3.96 ms (2.0× slower) |
| 256 MiB | 6.39 ms | 12.0 ms (1.8× slower) |
| 1 GiB | 18.6 ms | 37.8 ms (2.0× slower) |
| 1000 x 16 KiB | 4.23 ms | 20.6 ms (4.9× slower) |
| 1000 x 16 KiB, a process each | 642 ms | 2.91 s (4.5× slower) |
| mixed tree, 1000 files, 74 MiB | 6.42 ms | 37.5 ms (5.8× slower) |
| 16 x 64 MiB | 18.4 ms | 46.2 ms (2.5× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.10.0.*

| Workload | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|
| 4 KiB | 1.01 ms | 3.56 ms (3.5× slower) |
| 64 KiB | 1.05 ms | 3.27 ms (3.1× slower) |
| 1 MiB | 1.86 ms | 3.64 ms (2.0× slower) |
| 16 MiB | 5.68 ms | 7.14 ms (26% slower) |
| 256 MiB | 35.3 ms | 50.7 ms (44% slower) |
| 1 GiB | 124 ms | 187 ms (1.5× slower) |
| 1000 x 16 KiB | 6.92 ms | 65.0 ms (9.4× slower) |
| 1000 x 16 KiB, a process each | 695 ms | 2.95 s (4.2× slower) |
| mixed tree, 1000 files, 74 MiB | 18.8 ms | 124 ms (6.6× slower) |
| 16 x 64 MiB | 142 ms | 211 ms (49% slower) |
