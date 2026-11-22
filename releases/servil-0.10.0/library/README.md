# Per-machine library results

- [Apple M6](apple-m6.md)
- [Ryzen 9700X](x86-9700x.md)
- [2× EPYC 7443](x86-avx2.md)
- [EPYC 9255](x86-epyc-9255.md)
- [EPYC 9275F](x86-epyc-9275f.md)
- [2× Xeon Gold 6330 (b)](x86-icelake-2.md)
- [2× Xeon Gold 6330 (a)](x86-icelake.md)

## Workloads slower beyond 5%

- **EPYC 9275F** — `solo · OneMessage · 128 MiB` (every core): 13% slower
- **EPYC 9275F** — `solo · OneMessage · 32 MiB` (every core): 12% slower
- **EPYC 9275F** — `shared-one-cache · Collection · Nix files` (every core): 10% slower
- **EPYC 9255** — `shared-one-cache · LentMessages · 64 MiB` (every core): 16% slower
- **EPYC 9255** — `solo · OneMessage · 128 MiB` (every core): 7% slower
- **EPYC 9255** — `solo · OneMessage · 32 MiB` (every core): 6% slower
- **2× EPYC 7443** — `solo · LentMessages · 64 MiB` (every core): 19% slower
- **2× EPYC 7443** — `solo · OneMessage · 32 MiB` (every core): 7% slower
- **2× EPYC 7443** — `solo · OneMessage · 128 MiB` (every core): 7% slower
