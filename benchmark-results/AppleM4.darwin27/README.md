# Apple M4 measurements for bench-hashes 0.24

These runs are of a rented Mac mini that runs timing jobs and nothing else: an
Apple M4 with 10 CPUs (4 performance, 6 efficiency) and 16 GB. Its provider's
remote-desktop agent stays. Each run's load read quiet; the map's section
introductions and the overview show each run's own load.

| run | measured | other programs' load |
|---|---|---|
| library, job 3560 | November 21, 2026 | quiet; mean 0.08 CPUs; peak 0.54 |
| b3sum, job 3561 | November 21, 2026 | quiet; mean 0.18 CPUs; peak 0.57 |
| crowd, job 3562 | November 21, 2026 | the programs themselves |

All three ran on mains power with low-power mode off. The file paths name
the runner's home as `<runner-home>`; every measured value is as recorded.
