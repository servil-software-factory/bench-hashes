# Machines, storage, and loads (`2fd4ff6f643ef04f98f7a2e5f8ca0f096298827a`)

Two-socket Linux machines ran bound to memory node 0 (`numactl --cpunodebind=0 --membind=0`).

- **EPYC 9275F** (`x86-epyc-9275f`): AMD EPYC 9275F 24-Core Processor, 48 CPUs, linux7.0.0-43-generic; files on ext4 (/dev/md0 on /); quiet: other programs kept 0.00 CPUs busy on average, 0.02 in the busiest window; 15 rounds, each contender once on each input in each
- **EPYC 9255** (`x86-epyc-9255`): AMD EPYC 9255 24-Core Processor, 48 CPUs, linux7.0.0-43-generic; files on ext4 (/dev/md0 on /); quiet: other programs kept 0.00 CPUs busy on average, 0.03 in the busiest window; 15 rounds, each contender once on each input in each
- **Ryzen 9700X** (`x86-9700x`): AMD Ryzen 7 9700X 8-Core Processor, 16 CPUs, linux7.0.0-43-generic; files on ext4 (/dev/md0 on /); quiet: other programs kept 0.00 CPUs busy on average, 0.01 in the busiest window; 15 rounds, each contender once on each input in each
- **2× Xeon Gold 6330 (a)** (`x86-icelake`): Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic; files on ext4 (/dev/md0 on /); quiet: other programs kept 0.00 CPUs busy on average, 0.03 in the busiest window; 15 rounds, each contender once on each input in each
- **2× Xeon Gold 6330 (b)** (`x86-icelake-2`): Intel(R) Xeon(R) Gold 6330 CPU @ 2.00GHz, 112 CPUs, linux7.0.0-43-generic; files on ext4 (/dev/md0 on /); quiet: other programs kept 0.00 CPUs busy on average, 0.03 in the busiest window; 15 rounds, each contender once on each input in each
- **2× EPYC 7443** (`x86-avx2`): AMD EPYC 7443 24-Core Processor, 96 CPUs, linux7.0.0-42-generic; files on ext4 (/dev/md0 on /); quiet: other programs kept 0.00 CPUs busy on average, 0.06 in the busiest window; 15 rounds, each contender once on each input in each
