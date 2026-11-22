# b3sum on 2× Xeon Gold 6330 (b) (all 10 workloads)

Host `x86-icelake-2` (Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.03 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 810 μs | 778 μs (same) | 867 μs (same) | 3.35 ms (4.2× slower) |
| 64 KiB | 782 μs | 759 μs (same) | 859 μs (10% slower) | 3.11 ms (4.0× slower) |
| 1 MiB | 1.16 ms | 1.11 ms (same) | 1.37 ms (same) | 3.28 ms (2.8× slower) |
| 16 MiB | 1.89 ms | 1.90 ms (same) | 2.04 ms (8% slower) | 3.91 ms (2.1× slower) |
| 256 MiB | 6.25 ms | 6.16 ms (same) | 6.38 ms (same) | 11.8 ms (1.9× slower) |
| 1 GiB | 18.5 ms | 18.2 ms (same) | 18.4 ms (same) | 37.7 ms (2.0× slower) |
| 1000 x 16 KiB | 5.35 ms | 5.01 ms (**6% faster**) | 4.11 ms (**23% faster**) | 20.3 ms (3.8× slower) |
| 1000 x 16 KiB, a process each | 593 ms | 565 ms (**5% faster**) | 639 ms (8% slower) | 2.90 s (4.9× slower) |
| mixed tree, 1000 files, 74 MiB | 7.02 ms | 6.84 ms (same) | 6.35 ms (**9% faster**) | 36.8 ms (5.2× slower) |
| 16 x 64 MiB | 21.6 ms | 21.0 ms (same) | 18.6 ms (**14% faster**) | 44.9 ms (2.1× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 883 μs | 870 μs (same) | 949 μs (same) | 3.60 ms (4.1× slower) |
| 64 KiB | 924 μs | 883 μs (**4% faster**) | 970 μs (5% slower) | 3.27 ms (3.5× slower) |
| 1 MiB | 1.63 ms | 1.58 ms (**3% faster**) | 1.73 ms (6% slower) | 3.64 ms (2.2× slower) |
| 16 MiB | 4.98 ms | 4.90 ms (same) | 5.90 ms (19% slower) | 7.59 ms (1.5× slower) |
| 256 MiB | 53.6 ms | 53.5 ms (same) | 46.1 ms (**14% faster**) | 56.7 ms (6% slower) |
| 1 GiB | 206 ms | 206 ms (same) | 172 ms (**16% faster**) | 210 ms (same) |
| 1000 x 16 KiB | 7.47 ms | 7.39 ms (same) | 7.18 ms (**4% faster**) | 107 ms (14× slower) |
| 1000 x 16 KiB, a process each | 731 ms | 702 ms (**4% faster**) | 776 ms (6% slower) | 3.01 s (4.1× slower) |
| mixed tree, 1000 files, 74 MiB | 18.0 ms | 17.9 ms (same) | 19.4 ms (8% slower) | 130 ms (7.2× slower) |
| 16 x 64 MiB | 206 ms | 206 ms (same) | 179 ms (**13% faster**) | 230 ms (11% slower) |

---

# Quick suite on 2× Xeon Gold 6330 (b) (5 rounds, includes 100 x 16 KiB)

Host `x86-icelake-2` (Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.00 in the busiest window. Runs: 5 rounds, each contender once on each input in each (--quick: a check, not a record).

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 806 μs | 758 μs (same) | 849 μs (same) | 3.30 ms (4.1× slower) |
| 1 MiB | 1.18 ms | 1.14 ms (same) | 1.26 ms (same) | 3.32 ms (2.8× slower) |
| 16 MiB | 2.03 ms | 1.88 ms (same) | 2.06 ms (same) | 3.93 ms (1.9× slower) |
| 100 x 16 KiB | 1.66 ms | 1.53 ms (same) | 1.83 ms (same) | 4.68 ms (2.8× slower) |
| 100 x 16 KiB, a process each | 63.3 ms | 60.0 ms (**5% faster**) | 67.5 ms (7% slower) | 292 ms (4.6× slower) |
| mixed tree, 100 files, 5 MiB | 2.10 ms | 2.06 ms (same) | 2.15 ms (same) | 6.31 ms (3.0× slower) |
| 4 x 16 MiB | 3.67 ms | 3.45 ms (same) | 2.95 ms (same) | 6.52 ms (1.8× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 875 μs | 822 μs (same) | 920 μs (same) | 3.27 ms (3.8× slower) |
| 1 MiB | 1.62 ms | 1.62 ms (same) | 1.71 ms (5% slower) | 3.61 ms (2.2× slower) |
| 16 MiB | 4.89 ms | 4.87 ms (same) | 5.91 ms (21% slower) | 7.59 ms (1.6× slower) |
| 100 x 16 KiB | 1.91 ms | 1.85 ms (same) | 2.44 ms (27% slower) | 13.9 ms (7.3× slower) |
| 100 x 16 KiB, a process each | 76.9 ms | 73.3 ms (**5% faster**) | 81.6 ms (6% slower) | 304 ms (4.0× slower) |
| mixed tree, 100 files, 5 MiB | 3.59 ms | 3.30 ms (same) | 3.85 ms (same) | 16.2 ms (4.5× slower) |
| 4 x 16 MiB | 16.5 ms | 16.1 ms (same) | 14.0 ms (**15% faster**) | 21.8 ms (32% slower) |

---

# Official b3sum 1.8.7 vs BLAKE3-Servil 0.10.0 on 2× Xeon Gold 6330 (b)

Host `x86-icelake-2` (Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.02 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.10.0.*

| Workload | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|
| 4 KiB | 896 μs | 3.46 ms (3.9× slower) |
| 64 KiB | 875 μs | 3.12 ms (3.6× slower) |
| 1 MiB | 1.35 ms | 3.36 ms (2.5× slower) |
| 16 MiB | 2.00 ms | 3.93 ms (2.0× slower) |
| 256 MiB | 6.37 ms | 11.8 ms (1.8× slower) |
| 1 GiB | 18.7 ms | 37.0 ms (1.9× slower) |
| 1000 x 16 KiB | 4.22 ms | 20.4 ms (4.8× slower) |
| 1000 x 16 KiB, a process each | 641 ms | 2.90 s (4.5× slower) |
| mixed tree, 1000 files, 74 MiB | 6.44 ms | 36.9 ms (5.7× slower) |
| 16 x 64 MiB | 18.3 ms | 45.9 ms (2.4× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.10.0.*

| Workload | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|
| 4 KiB | 1.02 ms | 3.53 ms (3.5× slower) |
| 64 KiB | 1.01 ms | 3.20 ms (3.2× slower) |
| 1 MiB | 1.73 ms | 3.64 ms (2.1× slower) |
| 16 MiB | 5.87 ms | 7.66 ms (30% slower) |
| 256 MiB | 46.0 ms | 56.9 ms (24% slower) |
| 1 GiB | 173 ms | 210 ms (21% slower) |
| 1000 x 16 KiB | 7.19 ms | 107 ms (15× slower) |
| 1000 x 16 KiB, a process each | 779 ms | 3.00 s (3.9× slower) |
| mixed tree, 1000 files, 74 MiB | 18.8 ms | 131 ms (7.0× slower) |
| 16 x 64 MiB | 179 ms | 230 ms (29% slower) |
