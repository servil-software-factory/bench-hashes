# Library cells, 0.9.1 → 0.10.0: 2× Xeon Gold 6330 (a) (`x86-icelake`)

One bench-hashes (0.22) built both libraries and measured them in alternating pairs. Verdicts come from `bench-hashes compare` over the check's kept runs (3% bench mark; 5% project margin).

`old side: bench 97c0fbcbedb8 clean; servil 2c60420c2bcc clean`; `new side: bench 97c0fbcbedb8 clean; servil d3ed18d19a49 clean`

556 cells over 4 pairs: **8 faster, 5 slower**, the rest level.

| form | scenario | use case | cells | faster | slower | median time ratio | range |
|---|---|---|---:|---:|---:|---:|---|
| one thread | solo | OneMessage | 28 | 0 | 0 | ×1.002 | ×0.962–×1.119 |
| one thread | solo | ManyMessages | 24 | 0 | 0 | ×1.003 | ×0.987–×1.044 |
| one thread | solo | ManyLongMessages | 4 | 0 | 0 | ×0.999 | ×0.996–×1.006 |
| one thread | solo | IdleOneMessage | 28 | 0 | 0 | ×1.001 | ×0.953–×1.043 |
| one thread | solo | IdleManyMessages | 24 | 0 | 0 | ×1.002 | ×0.909–×1.040 |
| one thread | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×0.998 | ×0.981–×1.005 |
| one thread | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×0.992 | ×0.919–×1.022 |
| one thread | solo | ContinuousMessages | 13 | 0 | 0 | ×0.997 | ×0.979–×1.012 |
| one thread | solo | ContinuousBatches | 7 | 0 | 0 | ×1.002 | ×0.987–×1.010 |
| one thread | solo | LentMessages | 13 | 0 | 0 | ×1.000 | ×0.988–×1.012 |
| one thread | solo | Interleaved | 1 | 0 | 0 | ×1.006 | ×1.006–×1.006 |
| one thread | solo | Collection | 2 | 0 | 0 | ×0.998 | ×0.998–×0.998 |
| one thread | solo | LentBatches | 7 | 0 | 0 | ×1.002 | ×0.987–×1.010 |
| one thread | solo | ContinuousCollection | 2 | 0 | 0 | ×1.000 | ×1.000–×1.000 |
| one thread | solo | MerkleTree | 8 | 0 | 0 | ×0.998 | ×0.995–×1.000 |
| one thread | solo | InterleavedTurn | 2 | 0 | 0 | ×1.023 | ×1.010–×1.036 |
| one thread | solo | IdleInterleavedTurn | 2 | 0 | 1 | ×1.023 | ×1.008–×1.039 |
| one thread | solo | BriefIdleInterleavedTurn | 2 | 0 | 1 | ×1.034 | ×1.017–×1.050 |
| one thread | solo | ReceivingMessage | 3 | 2 | 0 | ×0.809 | ×0.808–×1.000 |
| every core | solo | OneMessage | 28 | 0 | 0 | ×1.008 | ×0.879–×1.079 |
| every core | solo | ManyMessages | 24 | 0 | 0 | ×1.003 | ×0.980–×1.026 |
| every core | solo | ManyLongMessages | 4 | 0 | 0 | ×1.006 | ×1.002–×1.010 |
| every core | solo | IdleOneMessage | 28 | 0 | 1 | ×0.992 | ×0.913–×1.054 |
| every core | solo | IdleManyMessages | 24 | 0 | 0 | ×0.998 | ×0.956–×1.063 |
| every core | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×0.998 | ×0.903–×1.007 |
| every core | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×0.996 | ×0.940–×1.020 |
| every core | solo | ContinuousMessages | 13 | 0 | 0 | ×0.999 | ×0.986–×1.011 |
| every core | solo | ContinuousBatches | 7 | 0 | 0 | ×0.991 | ×0.943–×1.007 |
| every core | solo | LentMessages | 13 | 0 | 0 | ×1.001 | ×0.993–×1.011 |
| every core | solo | Interleaved | 1 | 0 | 0 | ×1.009 | ×1.009–×1.009 |
| every core | solo | Collection | 2 | 0 | 0 | ×1.035 | ×1.013–×1.058 |
| every core | solo | LentBatches | 7 | 0 | 0 | ×0.991 | ×0.943–×1.007 |
| every core | solo | ContinuousCollection | 2 | 0 | 0 | ×0.996 | ×0.994–×0.999 |
| every core | solo | MerkleTree | 8 | 0 | 0 | ×0.996 | ×0.978–×1.000 |
| every core | solo | InterleavedTurn | 2 | 0 | 0 | ×1.010 | ×1.000–×1.019 |
| every core | solo | IdleInterleavedTurn | 2 | 0 | 1 | ×1.016 | ×1.000–×1.033 |
| every core | solo | BriefIdleInterleavedTurn | 2 | 0 | 1 | ×1.028 | ×1.016–×1.041 |
| every core | solo | ReceivingMessage | 3 | 2 | 0 | ×0.859 | ×0.859–×1.000 |
| every core | shared-one-cache | ContinuousMessages | 13 | 0 | 0 | ×1.001 | ×0.990–×1.016 |
| every core | shared-one-cache | ContinuousBatches | 7 | 0 | 0 | ×0.994 | ×0.988–×1.004 |
| every core | shared-one-cache | LentMessages | 13 | 0 | 0 | ×1.001 | ×0.997–×1.025 |
| every core | shared-one-cache | Interleaved | 1 | 0 | 0 | ×1.000 | ×1.000–×1.000 |
| every core | shared-one-cache | Collection | 2 | 0 | 0 | ×0.994 | ×0.990–×0.998 |
| every core | shared-one-cache | LentBatches | 7 | 0 | 0 | ×0.994 | ×0.988–×1.004 |
| every core | shared-one-cache | ContinuousCollection | 2 | 0 | 0 | ×0.998 | ×0.997–×0.999 |
| every core | shared-one-cache | MerkleTree | 8 | 0 | 0 | ×0.994 | ×0.946–×1.004 |
| every core | shared-one-cache | ReceivingMessage | 3 | 2 | 0 | ×0.860 | ×0.859–×1.000 |
| every core | shared-two-caches | ContinuousMessages | 13 | 0 | 0 | ×1.002 | ×0.987–×1.027 |
| every core | shared-two-caches | ContinuousBatches | 7 | 0 | 0 | ×1.000 | ×0.979–×1.011 |
| every core | shared-two-caches | LentMessages | 13 | 0 | 0 | ×1.001 | ×0.987–×1.015 |
| every core | shared-two-caches | Interleaved | 1 | 0 | 0 | ×1.000 | ×1.000–×1.000 |
| every core | shared-two-caches | Collection | 2 | 0 | 0 | ×0.959 | ×0.921–×0.997 |
| every core | shared-two-caches | LentBatches | 7 | 0 | 0 | ×1.000 | ×0.979–×1.011 |
| every core | shared-two-caches | ContinuousCollection | 2 | 0 | 0 | ×0.997 | ×0.995–×0.999 |
| every core | shared-two-caches | MerkleTree | 8 | 0 | 0 | ×0.998 | ×0.954–×1.014 |
| every core | shared-two-caches | ReceivingMessage | 3 | 2 | 0 | ×0.858 | ×0.858–×1.000 |

Slower cells (beyond 5% in bold):

- `blake3-servil-mt|solo|IdleOneMessage|32 KiB`: **+5% slower** (4 pairs)
- `blake3-servil-st|solo|BriefIdleInterleavedTurn|1024 x 1 KiB`: +5% slower (4 pairs)
- `blake3-servil-mt|solo|BriefIdleInterleavedTurn|1024 x 1 KiB`: +4% slower (4 pairs)
- `blake3-servil-st|solo|IdleInterleavedTurn|1024 x 1 KiB`: +4% slower (4 pairs)
- `blake3-servil-mt|solo|IdleInterleavedTurn|1024 x 1 KiB`: +3% slower (4 pairs)

Ten largest gains:

- `blake3-servil-st|solo|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-st|solo|ReceivingMessage|2 μs`: 1.2× faster
- `blake3-servil-mt|shared-two-caches|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-mt|shared-two-caches|ReceivingMessage|2 μs`: 1.2× faster
- `blake3-servil-mt|solo|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-mt|solo|ReceivingMessage|2 μs`: 1.2× faster
- `blake3-servil-mt|shared-one-cache|ReceivingMessage|saturated`: 1.2× faster
- `blake3-servil-mt|shared-one-cache|ReceivingMessage|2 μs`: 1.2× faster


Repeated by name with 16 or more link layouts a side, after the check above marked them from four pairs:

- `blake3-servil-mt|solo|IdleOneMessage|32 KiB`: level over 16 pairs (5.4% slower from four)
