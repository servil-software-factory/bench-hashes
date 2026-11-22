# b3sum on Apple M6 (job 3544)

Host `apple-m6` (Apple M6, 12 CPUs, darwin27); files on apfs (on /System/Volumes/Data). Load: quiet: other programs kept 0.19 CPUs busy on average, 0.71 in the busiest window. Runs: 15 rounds, each contender once on each input in each.

## In memory (page cache)

*Wall time per run; parenthesized comparison is against Official b3sum 1.8.7.*

| Workload | Official b3sum 1.8.7 | BLAKE3-Servil 0.10.0 | Official b3sum (1 thread) | BLAKE3-Servil 0.10.0 (1 thread) | sha256sum | sha1sum | md5sum |
|---|---:|---:|---:|---:|---:|---:|---:|
| 4 KiB | 1.24 ms | 1.21 ms (same) | 1.09 ms (**12% faster**) | 1.22 ms (same) | 845 μs (**32% faster**) | 850 μs (**31% faster**) | 861 μs (**31% faster**) |
| 64 KiB | 1.15 ms | 1.12 ms (same) | 1.02 ms (**11% faster**) | 1.12 ms (same) | 801 μs (**30% faster**) | 798 μs (**31% faster**) | 851 μs (**26% faster**) |
| 1 MiB | 1.17 ms | 1.25 ms (7% slower) | 1.38 ms (18% slower) | 1.25 ms (7% slower) | 1.14 ms (same) | 1.15 ms (same) | 1.93 ms (1.7× slower) |
| 16 MiB | 2.02 ms | 2.03 ms (same) | 7.55 ms (3.7× slower) | 4.17 ms (2.1× slower) | 7.14 ms (3.5× slower) | 7.31 ms (3.6× slower) | 20.2 ms (10× slower) |
| 256 MiB | 15.7 ms | 13.3 ms (**15% faster**) | 110 ms (7.0× slower) | 54.7 ms (3.5× slower) | 104 ms (6.6× slower) | 107 ms (6.8× slower) | 309 ms (20× slower) |
| 1 GiB | 57.9 ms | 47.9 ms (**17% faster**) | 433 ms (7.5× slower) | 215 ms (3.7× slower) | 416 ms (7.2× slower) | 426 ms (7.4× slower) | 1.23 s (21× slower) |
| 1000 x 16 KiB | 40.7 ms | 5.49 ms (**7.4× faster**) | 16.5 ms (**2.5× faster**) | 13.2 ms (**3.1× faster**) | 14.5 ms (**2.8× faster**) | 14.8 ms (**2.7× faster**) | 27.2 ms (**33% faster**) |
| 1000 x 16 KiB, a process each | 1.07 s | 1.04 s (same) | 948 ms (**11% faster**) | 1.04 s (same) | 732 ms (**31% faster**) | 733 ms (**31% faster**) | 744 ms (**30% faster**) |
| mixed tree, 1000 files, 74 MiB | 35.5 ms | 7.37 ms (**4.8× faster**) | 41.0 ms (15% slower) | 25.1 ms (**29% faster**) | 38.2 ms (8% slower) | 39.1 ms (10% slower) | 97.6 ms (2.7× slower) |
| 16 x 64 MiB | 59.2 ms | 50.3 ms (**15% faster**) | 431 ms (7.3× slower) | 215 ms (3.6× slower) | 413 ms (7.0× slower) | 424 ms (7.2× slower) | 1.23 s (21× slower) |

## From disk (evicted before each run)

*Wall time per run; parenthesized comparison is against Official b3sum 1.8.7.*

| Workload | Official b3sum 1.8.7 | BLAKE3-Servil 0.10.0 | Official b3sum (1 thread) | BLAKE3-Servil 0.10.0 (1 thread) | sha256sum | sha1sum | md5sum |
|---|---:|---:|---:|---:|---:|---:|---:|
| 4 KiB | 1.66 ms | 1.55 ms (same) | 1.42 ms (same) | 1.57 ms (same) | 1.21 ms (same) | 1.19 ms (same) | 1.22 ms (same) |
| 64 KiB | 1.35 ms | 1.29 ms (**5% faster**) | 1.42 ms (5% slower) | 1.29 ms (**5% faster**) | 1.18 ms (**13% faster**) | 1.18 ms (**13% faster**) | 1.29 ms (**5% faster**) |
| 1 MiB | 2.11 ms | 1.74 ms (**18% faster**) | 2.77 ms (31% slower) | 1.75 ms (**17% faster**) | 2.35 ms (11% slower) | 2.37 ms (12% slower) | 3.09 ms (46% slower) |
| 16 MiB | 7.97 ms | 5.83 ms (**26% faster**) | 15.5 ms (2.0× slower) | 7.07 ms (same) | 9.05 ms (17% slower) | 9.20 ms (18% slower) | 21.4 ms (2.8× slower) |
| 256 MiB | 158 ms | 79.3 ms (**2.0× faster**) | 180 ms (14% slower) | 80.3 ms (**2.0× faster**) | 110 ms (**30% faster**) | 112 ms (**29% faster**) | 312 ms (2.0× slower) |
| 1 GiB | 661 ms | 311 ms (**2.1× faster**) | 605 ms (**8% faster**) | 312 ms (**2.1× faster**) | 431 ms (**1.5× faster**) | 440 ms (**1.5× faster**) | 1.24 s (1.9× slower) |
| 1000 x 16 KiB | 86.9 ms | 15.0 ms (**5.8× faster**) | 48.2 ms (**1.8× faster**) | 45.3 ms (**1.9× faster**) | 46.3 ms (**1.9× faster**) | 45.9 ms (**1.9× faster**) | 60.6 ms (**30% faster**) |
| 1000 x 16 KiB, a process each | 1.14 s | 1.11 s (**3% faster**) | 1.02 s (**11% faster**) | 1.11 s (**3% faster**) | 800 ms (**30% faster**) | 802 ms (**30% faster**) | 814 ms (**29% faster**) |
| mixed tree, 1000 files, 74 MiB | 166 ms | 34.5 ms (**4.8× faster**) | 214 ms (29% slower) | 108 ms (**1.5× faster**) | 175 ms (5% slower) | 176 ms (6% slower) | 235 ms (42% slower) |
| 16 x 64 MiB | 622 ms | 347 ms (**1.8× faster**) | 605 ms (same) | 337 ms (**1.8× faster**) | 452 ms (**27% faster**) | 461 ms (**26% faster**) | 1.26 s (2.0× slower) |
