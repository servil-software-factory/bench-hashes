# Library cells, 0.9.1 → 0.10.0: 2× Xeon Gold 6330 (b) (`x86-icelake-2`)

One bench-hashes (0.22) built both libraries and measured them in alternating pairs. Verdicts come from `bench-hashes compare` over the check's kept runs (3% bench mark; 5% project margin).

`old side: bench 97c0fbcbedb8 clean; servil 2c60420c2bcc clean`; `new side: bench 97c0fbcbedb8 clean; servil d3ed18d19a49 clean`

556 cells over 4 pairs: **8 faster, 4 slower**, the rest level.

| form | scenario | use case | cells | faster | slower | median time ratio | range |
|---|---|---|---:|---:|---:|---:|---|
| one thread | solo | OneMessage | 28 | 0 | 0 | ×1.000 | ×0.932–×1.079 |
| one thread | solo | ManyMessages | 24 | 0 | 1 | ×1.001 | ×0.977–×1.060 |
| one thread | solo | ManyLongMessages | 4 | 0 | 0 | ×1.002 | ×0.998–×1.010 |
| one thread | solo | IdleOneMessage | 28 | 0 | 0 | ×1.002 | ×0.947–×1.047 |
| one thread | solo | IdleManyMessages | 24 | 0 | 0 | ×0.998 | ×0.955–×1.034 |
| one thread | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×1.000 | ×0.974–×1.014 |
| one thread | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×0.996 | ×0.943–×1.020 |
| one thread | solo | ContinuousMessages | 13 | 0 | 0 | ×0.991 | ×0.969–×1.004 |
| one thread | solo | ContinuousBatches | 7 | 0 | 0 | ×1.002 | ×0.983–×1.015 |
| one thread | solo | LentMessages | 13 | 0 | 0 | ×1.000 | ×0.988–×1.004 |
| one thread | solo | Interleaved | 1 | 0 | 0 | ×1.001 | ×1.001–×1.001 |
| one thread | solo | Collection | 2 | 0 | 0 | ×1.002 | ×1.000–×1.005 |
| one thread | solo | LentBatches | 7 | 0 | 0 | ×1.002 | ×0.983–×1.015 |
| one thread | solo | ContinuousCollection | 2 | 0 | 0 | ×1.002 | ×1.002–×1.002 |
| one thread | solo | MerkleTree | 8 | 0 | 0 | ×0.998 | ×0.997–×1.002 |
| one thread | solo | InterleavedTurn | 2 | 0 | 0 | ×1.026 | ×1.010–×1.041 |
| one thread | solo | IdleInterleavedTurn | 2 | 0 | 0 | ×1.017 | ×1.006–×1.028 |
| one thread | solo | BriefIdleInterleavedTurn | 2 | 0 | 0 | ×1.030 | ×1.013–×1.046 |
| one thread | solo | ReceivingMessage | 3 | 2 | 0 | ×0.807 | ×0.805–×1.000 |
| every core | solo | OneMessage | 28 | 0 | 0 | ×1.003 | ×0.907–×1.054 |
| every core | solo | ManyMessages | 24 | 0 | 1 | ×1.000 | ×0.960–×1.050 |
| every core | solo | ManyLongMessages | 4 | 0 | 0 | ×1.006 | ×0.993–×1.007 |
| every core | solo | IdleOneMessage | 28 | 0 | 0 | ×1.003 | ×0.937–×1.105 |
| every core | solo | IdleManyMessages | 24 | 0 | 0 | ×0.990 | ×0.938–×1.039 |
| every core | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×1.000 | ×0.793–×1.091 |
| every core | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×0.997 | ×0.963–×1.032 |
| every core | solo | ContinuousMessages | 13 | 0 | 0 | ×1.000 | ×0.986–×1.003 |
| every core | solo | ContinuousBatches | 7 | 0 | 0 | ×0.995 | ×0.985–×1.014 |
| every core | solo | LentMessages | 13 | 0 | 0 | ×1.000 | ×0.994–×1.008 |
| every core | solo | Interleaved | 1 | 0 | 0 | ×1.006 | ×1.006–×1.006 |
| every core | solo | Collection | 2 | 0 | 0 | ×1.000 | ×0.975–×1.024 |
| every core | solo | LentBatches | 7 | 0 | 0 | ×0.995 | ×0.985–×1.014 |
| every core | solo | ContinuousCollection | 2 | 0 | 0 | ×1.004 | ×1.002–×1.006 |
| every core | solo | MerkleTree | 8 | 0 | 0 | ×0.998 | ×0.977–×1.004 |
| every core | solo | InterleavedTurn | 2 | 0 | 0 | ×1.016 | ×1.007–×1.026 |
| every core | solo | IdleInterleavedTurn | 2 | 0 | 1 | ×1.017 | ×1.000–×1.034 |
| every core | solo | BriefIdleInterleavedTurn | 2 | 0 | 1 | ×1.028 | ×1.017–×1.040 |
| every core | solo | ReceivingMessage | 3 | 2 | 0 | ×0.858 | ×0.857–×1.000 |
| every core | shared-one-cache | ContinuousMessages | 13 | 0 | 0 | ×1.003 | ×0.987–×1.015 |
| every core | shared-one-cache | ContinuousBatches | 7 | 0 | 0 | ×0.991 | ×0.980–×1.003 |
| every core | shared-one-cache | LentMessages | 13 | 0 | 0 | ×1.002 | ×0.994–×1.015 |
| every core | shared-one-cache | Interleaved | 1 | 0 | 0 | ×0.995 | ×0.995–×0.995 |
| every core | shared-one-cache | Collection | 2 | 0 | 0 | ×1.015 | ×1.007–×1.023 |
| every core | shared-one-cache | LentBatches | 7 | 0 | 0 | ×0.991 | ×0.980–×1.003 |
| every core | shared-one-cache | ContinuousCollection | 2 | 0 | 0 | ×1.002 | ×1.002–×1.003 |
| every core | shared-one-cache | MerkleTree | 8 | 0 | 0 | ×0.994 | ×0.954–×1.000 |
| every core | shared-one-cache | ReceivingMessage | 3 | 2 | 0 | ×0.858 | ×0.858–×1.000 |
| every core | shared-two-caches | ContinuousMessages | 13 | 0 | 0 | ×1.002 | ×0.982–×1.015 |
| every core | shared-two-caches | ContinuousBatches | 7 | 0 | 0 | ×1.002 | ×0.977–×1.006 |
| every core | shared-two-caches | LentMessages | 13 | 0 | 0 | ×1.001 | ×0.982–×1.028 |
| every core | shared-two-caches | Interleaved | 1 | 0 | 0 | ×1.002 | ×1.002–×1.002 |
| every core | shared-two-caches | Collection | 2 | 0 | 0 | ×0.984 | ×0.949–×1.018 |
| every core | shared-two-caches | LentBatches | 7 | 0 | 0 | ×1.002 | ×0.977–×1.006 |
| every core | shared-two-caches | ContinuousCollection | 2 | 0 | 0 | ×1.002 | ×1.001–×1.002 |
| every core | shared-two-caches | MerkleTree | 8 | 0 | 0 | ×1.002 | ×0.983–×1.012 |
| every core | shared-two-caches | ReceivingMessage | 3 | 2 | 0 | ×0.858 | ×0.858–×1.000 |

Slower cells (beyond 5% in bold):

- `blake3-servil-st|solo|ManyMessages|3`: **+6% slower** (4 pairs)
- `blake3-servil-mt|solo|ManyMessages|4`: +5% slower (4 pairs)
- `blake3-servil-mt|solo|BriefIdleInterleavedTurn|1024 x 1 KiB`: +4% slower (4 pairs)
- `blake3-servil-mt|solo|IdleInterleavedTurn|1024 x 1 KiB`: +3% slower (4 pairs)

Ten largest gains:

- `blake3-servil-st|solo|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-st|solo|ReceivingMessage|2 μs`: 1.2× faster
- `blake3-servil-mt|solo|ReceivingMessage|2 μs`: 1.2× faster
- `blake3-servil-mt|solo|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-mt|shared-one-cache|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-mt|shared-one-cache|ReceivingMessage|2 μs`: 1.2× faster
- `blake3-servil-mt|shared-two-caches|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-mt|shared-two-caches|ReceivingMessage|2 μs`: 1.2× faster


Repeated by name with 16 or more link layouts a side, after the check above marked them from four pairs:

- `blake3-servil-st|solo|ManyMessages|3`: level over 16 pairs (6.0% slower from four)
- `blake3-servil-mt|solo|ManyMessages|4`: level over 64 pairs (5.0% slower from four)
