# Library cells, 0.9.1 → 0.10.0: 2× EPYC 7443 (`x86-avx2`)

One bench-hashes (0.22) built both libraries and measured them in alternating pairs. Verdicts come from `bench-hashes compare` over the check's kept runs (3% bench mark; 5% project margin).

`old side: bench 97c0fbcbedb8 clean; servil 2c60420c2bcc clean`; `new side: bench 97c0fbcbedb8 clean; servil d3ed18d19a49 clean`

556 cells over 4 pairs: **45 faster, 4 slower**, the rest level.

| form | scenario | use case | cells | faster | slower | median time ratio | range |
|---|---|---|---:|---:|---:|---:|---|
| one thread | solo | OneMessage | 28 | 1 | 0 | ×0.996 | ×0.947–×1.060 |
| one thread | solo | ManyMessages | 24 | 0 | 0 | ×1.018 | ×0.963–×1.125 |
| one thread | solo | ManyLongMessages | 4 | 0 | 0 | ×0.977 | ×0.964–×1.005 |
| one thread | solo | IdleOneMessage | 28 | 0 | 0 | ×0.996 | ×0.920–×1.100 |
| one thread | solo | IdleManyMessages | 24 | 0 | 0 | ×1.004 | ×0.957–×1.073 |
| one thread | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×0.990 | ×0.948–×1.012 |
| one thread | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×0.999 | ×0.974–×1.019 |
| one thread | solo | ContinuousMessages | 13 | 0 | 0 | ×0.992 | ×0.975–×1.002 |
| one thread | solo | ContinuousBatches | 7 | 0 | 0 | ×0.994 | ×0.987–×1.036 |
| one thread | solo | LentMessages | 13 | 0 | 0 | ×0.995 | ×0.982–×1.002 |
| one thread | solo | Interleaved | 1 | 0 | 0 | ×1.008 | ×1.008–×1.008 |
| one thread | solo | Collection | 2 | 0 | 0 | ×1.000 | ×0.999–×1.001 |
| one thread | solo | LentBatches | 7 | 0 | 0 | ×0.994 | ×0.987–×1.036 |
| one thread | solo | ContinuousCollection | 2 | 0 | 0 | ×0.986 | ×0.984–×0.989 |
| one thread | solo | MerkleTree | 8 | 0 | 0 | ×0.994 | ×0.988–×1.007 |
| one thread | solo | InterleavedTurn | 2 | 0 | 0 | ×0.994 | ×0.988–×1.001 |
| one thread | solo | IdleInterleavedTurn | 2 | 0 | 0 | ×0.984 | ×0.961–×1.006 |
| one thread | solo | BriefIdleInterleavedTurn | 2 | 0 | 0 | ×1.014 | ×1.014–×1.014 |
| one thread | solo | ReceivingMessage | 3 | 2 | 0 | ×0.843 | ×0.842–×1.000 |
| every core | solo | OneMessage | 28 | 2 | 2 | ×0.990 | ×0.694–×1.174 |
| every core | solo | ManyMessages | 24 | 5 | 0 | ×1.007 | ×0.649–×1.099 |
| every core | solo | ManyLongMessages | 4 | 1 | 0 | ×0.988 | ×0.701–×1.005 |
| every core | solo | IdleOneMessage | 28 | 0 | 0 | ×0.984 | ×0.825–×1.054 |
| every core | solo | IdleManyMessages | 24 | 2 | 0 | ×0.984 | ×0.713–×1.068 |
| every core | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×0.992 | ×0.693–×1.028 |
| every core | solo | BriefIdleManyMessages | 24 | 2 | 0 | ×0.996 | ×0.568–×1.008 |
| every core | solo | ContinuousMessages | 13 | 0 | 0 | ×0.994 | ×0.972–×1.002 |
| every core | solo | ContinuousBatches | 7 | 2 | 0 | ×0.977 | ×0.561–×0.997 |
| every core | solo | LentMessages | 13 | 3 | 1 | ×0.997 | ×0.613–×1.185 |
| every core | solo | Interleaved | 1 | 0 | 0 | ×0.998 | ×0.998–×0.998 |
| every core | solo | Collection | 2 | 0 | 0 | ×0.934 | ×0.925–×0.943 |
| every core | solo | LentBatches | 7 | 2 | 0 | ×0.977 | ×0.561–×0.997 |
| every core | solo | ContinuousCollection | 2 | 0 | 0 | ×0.970 | ×0.955–×0.984 |
| every core | solo | MerkleTree | 8 | 1 | 0 | ×0.951 | ×0.609–×1.038 |
| every core | solo | InterleavedTurn | 2 | 0 | 0 | ×0.998 | ×0.993–×1.004 |
| every core | solo | IdleInterleavedTurn | 2 | 0 | 0 | ×0.995 | ×0.983–×1.007 |
| every core | solo | BriefIdleInterleavedTurn | 2 | 0 | 0 | ×1.006 | ×1.005–×1.008 |
| every core | solo | ReceivingMessage | 3 | 2 | 0 | ×0.882 | ×0.880–×1.000 |
| every core | shared-one-cache | ContinuousMessages | 13 | 0 | 0 | ×0.997 | ×0.973–×1.005 |
| every core | shared-one-cache | ContinuousBatches | 7 | 1 | 0 | ×0.980 | ×0.539–×1.031 |
| every core | shared-one-cache | LentMessages | 13 | 1 | 0 | ×0.997 | ×0.686–×1.061 |
| every core | shared-one-cache | Interleaved | 1 | 0 | 0 | ×1.004 | ×1.004–×1.004 |
| every core | shared-one-cache | Collection | 2 | 0 | 0 | ×1.015 | ×1.001–×1.030 |
| every core | shared-one-cache | LentBatches | 7 | 1 | 0 | ×0.980 | ×0.539–×1.031 |
| every core | shared-one-cache | ContinuousCollection | 2 | 0 | 0 | ×0.974 | ×0.970–×0.978 |
| every core | shared-one-cache | MerkleTree | 8 | 3 | 1 | ×0.939 | ×0.681–×1.071 |
| every core | shared-one-cache | ReceivingMessage | 3 | 2 | 0 | ×0.888 | ×0.887–×1.000 |
| every core | shared-two-caches | ContinuousMessages | 13 | 0 | 0 | ×0.996 | ×0.974–×1.004 |
| every core | shared-two-caches | ContinuousBatches | 7 | 2 | 0 | ×0.980 | ×0.654–×0.997 |
| every core | shared-two-caches | LentMessages | 13 | 3 | 0 | ×0.996 | ×0.680–×1.004 |
| every core | shared-two-caches | Interleaved | 1 | 0 | 0 | ×1.006 | ×1.006–×1.006 |
| every core | shared-two-caches | Collection | 2 | 0 | 0 | ×0.984 | ×0.982–×0.986 |
| every core | shared-two-caches | LentBatches | 7 | 2 | 0 | ×0.980 | ×0.654–×0.997 |
| every core | shared-two-caches | ContinuousCollection | 2 | 0 | 0 | ×0.954 | ×0.940–×0.968 |
| every core | shared-two-caches | MerkleTree | 8 | 3 | 0 | ×0.914 | ×0.615–×1.026 |
| every core | shared-two-caches | ReceivingMessage | 3 | 2 | 0 | ×0.881 | ×0.880–×1.000 |

Slower cells (beyond 5% in bold):

- `blake3-servil-mt|solo|LentMessages|64 MiB`: **+19% slower** (4 pairs)
- `blake3-servil-mt|shared-one-cache|MerkleTree|1048576 x 16 B`: **+7% slower** (4 pairs)
- `blake3-servil-mt|solo|OneMessage|32 MiB`: **+7% slower** (4 pairs)
- `blake3-servil-mt|solo|OneMessage|128 MiB`: **+7% slower** (4 pairs)

Ten largest gains:

- `blake3-servil-mt|shared-one-cache|ContinuousBatches|16384`: 1.9× faster
- `blake3-servil-mt|shared-one-cache|LentBatches|16384`: 1.9× faster
- `blake3-servil-mt|solo|ContinuousBatches|16384`: 1.8× faster
- `blake3-servil-mt|solo|LentBatches|16384`: 1.8× faster
- `blake3-servil-mt|solo|BriefIdleManyMessages|8192`: 1.8× faster
- `blake3-servil-mt|solo|ContinuousBatches|65536`: 1.7× faster
- `blake3-servil-mt|solo|LentBatches|65536`: 1.7× faster
- `blake3-servil-mt|solo|BriefIdleManyMessages|16384`: 1.6× faster
- `blake3-servil-mt|solo|MerkleTree|4096`: 1.6× faster
- `blake3-servil-mt|solo|LentMessages|4 MiB`: 1.6× faster


Repeated by name with 16 or more link layouts a side, after the check above marked them from four pairs:

- `blake3-servil-mt|shared-one-cache|MerkleTree|1048576 x 16 B`: 3.6% slower over 16 pairs (7.1% from four)
