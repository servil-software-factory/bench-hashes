# b3sum on 2× EPYC 7443 (all 10 workloads)

Host `x86-avx2` (AMD EPYC 7443 24-Core Processor, 96 CPUs, linux7.0.0-42-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.06 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 979 μs | 980 μs (same) | 1.05 ms (same) | 4.04 ms (4.2× slower) |
| 64 KiB | 840 μs | 790 μs (same) | 935 μs (same) | 3.73 ms (4.5× slower) |
| 1 MiB | 1.34 ms | 1.36 ms (same) | 1.16 ms (**13% faster**) | 3.82 ms (2.9× slower) |
| 16 MiB | 2.12 ms | 2.12 ms (same) | 2.25 ms (5% slower) | 4.60 ms (2.2× slower) |
| 256 MiB | 6.41 ms | 6.73 ms (same) | 6.64 ms (same) | 13.1 ms (2.0× slower) |
| 1 GiB | 17.4 ms | 17.7 ms (same) | 18.0 ms (same) | 37.3 ms (2.1× slower) |
| 1000 x 16 KiB | 5.07 ms | 4.99 ms (same) | 4.73 ms (**7% faster**) | 35.1 ms (6.9× slower) |
| 1000 x 16 KiB, a process each | 587 ms | 588 ms (same) | 631 ms (7% slower) | 3.08 s (5.2× slower) |
| mixed tree, 1000 files, 74 MiB | 7.30 ms | 6.87 ms (same) | 7.44 ms (same) | 48.2 ms (6.6× slower) |
| 16 x 64 MiB | 21.0 ms | 20.9 ms (same) | 17.9 ms (**15% faster**) | 48.2 ms (2.2× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 1.04 ms | 1.04 ms (same) | 1.17 ms (same) | 4.09 ms (4.0× slower) |
| 64 KiB | 1.12 ms | 1.13 ms (same) | 1.20 ms (same) | 4.10 ms (3.6× slower) |
| 1 MiB | 2.91 ms | 2.92 ms (same) | 3.10 ms (same) | 4.51 ms (1.6× slower) |
| 16 MiB | 5.82 ms | 5.95 ms (same) | 6.76 ms (16% slower) | 8.93 ms (1.5× slower) |
| 256 MiB | 60.8 ms | 61.6 ms (same) | 39.6 ms (**1.5× faster**) | 64.9 ms (7% slower) |
| 1 GiB | 231 ms | 232 ms (same) | 130 ms (**1.8× faster**) | 237 ms (same) |
| 1000 x 16 KiB | 9.66 ms | 9.60 ms (same) | 9.69 ms (same) | 219 ms (23× slower) |
| 1000 x 16 KiB, a process each | 788 ms | 790 ms (same) | 834 ms (6% slower) | 3.28 s (4.2× slower) |
| mixed tree, 1000 files, 74 MiB | 22.9 ms | 23.0 ms (same) | 20.7 ms (**10% faster**) | 214 ms (9.3× slower) |
| 16 x 64 MiB | 229 ms | 230 ms (same) | 159 ms (**31% faster**) | 276 ms (20% slower) |

---

# Quick suite on 2× EPYC 7443 (5 rounds, includes 100 x 16 KiB)

Host `x86-avx2` (AMD EPYC 7443 24-Core Processor, 96 CPUs, linux7.0.0-42-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.00 in the busiest window. Runs: 5 rounds, each contender once on each input in each (--quick: a check, not a record).

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 756 μs | 761 μs (same) | 814 μs (same) | 3.32 ms (4.4× slower) |
| 1 MiB | 1.18 ms | 1.16 ms (same) | 1.08 ms (same) | 3.47 ms (2.9× slower) |
| 16 MiB | 1.88 ms | 2.03 ms (8% slower) | 2.12 ms (13% slower) | 4.26 ms (2.3× slower) |
| 100 x 16 KiB | 1.66 ms | 1.58 ms (same) | 1.90 ms (same) | 6.76 ms (4.1× slower) |
| 100 x 16 KiB, a process each | 59.4 ms | 59.3 ms (same) | 63.6 ms (7% slower) | 311 ms (5.2× slower) |
| mixed tree, 100 files, 5 MiB | 2.30 ms | 2.20 ms (same) | 2.28 ms (same) | 8.27 ms (3.6× slower) |
| 4 x 16 MiB | 3.79 ms | 3.87 ms (same) | 3.07 ms (**19% faster**) | 7.60 ms (2.0× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.9.1.*

| Workload | BLAKE3-Servil 0.9.1 | BLAKE3-Servil 0.8.0 | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|---:|---:|
| 4 KiB | 798 μs | 781 μs (same) | 825 μs (same) | 3.36 ms (4.2× slower) |
| 1 MiB | 2.75 ms | 2.78 ms (same) | 3.00 ms (same) | 4.45 ms (1.6× slower) |
| 16 MiB | 5.68 ms | 5.60 ms (same) | 6.42 ms (same) | 8.65 ms (1.5× slower) |
| 100 x 16 KiB | 2.32 ms | 2.20 ms (same) | 2.62 ms (same) | 25.0 ms (11× slower) |
| 100 x 16 KiB, a process each | 79.3 ms | 79.0 ms (same) | 84.2 ms (6% slower) | 330 ms (4.2× slower) |
| mixed tree, 100 files, 5 MiB | 4.83 ms | 4.81 ms (same) | 5.15 ms (same) | 23.8 ms (4.9× slower) |
| 4 x 16 MiB | 23.6 ms | 24.1 ms (same) | 14.9 ms (**1.6× faster**) | 31.3 ms (33% slower) |

---

# Official b3sum 1.8.7 vs BLAKE3-Servil 0.10.0 on 2× EPYC 7443

Host `x86-avx2` (AMD EPYC 7443 24-Core Processor, 96 CPUs, linux7.0.0-42-generic); files on ext4 (/dev/md0 on /). Load: quiet: other programs kept 0.00 CPUs busy on average, 0.03 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.10.0.*

| Workload | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|
| 4 KiB | 1.11 ms | 4.18 ms (3.8× slower) |
| 64 KiB | 909 μs | 3.81 ms (4.2× slower) |
| 1 MiB | 1.26 ms | 3.99 ms (3.2× slower) |
| 16 MiB | 2.46 ms | 4.65 ms (1.9× slower) |
| 256 MiB | 6.76 ms | 12.3 ms (1.8× slower) |
| 1 GiB | 17.6 ms | 35.1 ms (1.9× slower) |
| 1000 x 16 KiB | 4.70 ms | 34.0 ms (7.2× slower) |
| 1000 x 16 KiB, a process each | 634 ms | 3.08 s (4.9× slower) |
| mixed tree, 1000 files, 74 MiB | 7.52 ms | 49.0 ms (6.5× slower) |
| 16 x 64 MiB | 17.5 ms | 48.1 ms (2.6× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against BLAKE3-Servil 0.10.0.*

| Workload | BLAKE3-Servil 0.10.0 | Official b3sum 1.8.7 |
|---|---:|---:|
| 4 KiB | 1.19 ms | 4.03 ms (3.4× slower) |
| 64 KiB | 1.26 ms | 3.94 ms (3.1× slower) |
| 1 MiB | 3.14 ms | 4.68 ms (49% slower) |
| 16 MiB | 6.70 ms | 8.95 ms (33% slower) |
| 256 MiB | 39.0 ms | 64.0 ms (1.6× slower) |
| 1 GiB | 131 ms | 236 ms (1.8× slower) |
| 1000 x 16 KiB | 9.74 ms | 220 ms (23× slower) |
| 1000 x 16 KiB, a process each | 837 ms | 3.29 s (3.9× slower) |
| mixed tree, 1000 files, 74 MiB | 20.5 ms | 212 ms (10× slower) |
| 16 x 64 MiB | 158 ms | 276 ms (1.7× slower) |
