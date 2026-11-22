# Library cells, 0.9.1 → 0.10.0: Apple M6 (`apple-m6`)

One bench-hashes (0.22) built both libraries and measured them in alternating pairs. Verdicts come from `bench-hashes compare` over the check's kept runs (3% bench mark; 5% project margin).

`old side: bench 97c0fbcbedb8 clean; servil 2c60420c2bcc clean`; `new side: bench 97c0fbcbedb8 clean; servil 2fd4ff6f643e clean`

500 cells over 4 pairs: **6 faster, 1 slower**, the rest level.

| form | scenario | use case | cells | faster | slower | median time ratio | range |
|---|---|---|---:|---:|---:|---:|---|
| one thread | solo | OneMessage | 28 | 0 | 0 | ×0.996 | ×0.932–×1.079 |
| one thread | solo | ManyMessages | 24 | 0 | 0 | ×1.000 | ×0.964–×1.170 |
| one thread | solo | ManyLongMessages | 4 | 0 | 0 | ×1.001 | ×0.998–×1.028 |
| one thread | solo | IdleOneMessage | 28 | 0 | 0 | ×1.000 | ×0.983–×1.023 |
| one thread | solo | IdleManyMessages | 24 | 0 | 0 | ×0.978 | ×0.944–×1.032 |
| one thread | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×0.998 | ×0.982–×1.032 |
| one thread | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×1.000 | ×0.976–×1.059 |
| one thread | solo | ContinuousMessages | 13 | 0 | 0 | ×0.999 | ×0.989–×1.010 |
| one thread | solo | ContinuousBatches | 7 | 0 | 0 | ×1.000 | ×0.982–×1.004 |
| one thread | solo | LentMessages | 13 | 0 | 0 | ×1.000 | ×0.990–×1.010 |
| one thread | solo | Interleaved | 1 | 0 | 0 | ×0.975 | ×0.975–×0.975 |
| one thread | solo | Collection | 2 | 0 | 0 | ×1.001 | ×1.000–×1.002 |
| one thread | solo | LentBatches | 7 | 0 | 0 | ×1.000 | ×0.982–×1.004 |
| one thread | solo | ContinuousCollection | 2 | 0 | 0 | ×0.998 | ×0.996–×1.000 |
| one thread | solo | MerkleTree | 8 | 0 | 0 | ×1.000 | ×0.986–×1.006 |
| one thread | solo | InterleavedTurn | 2 | 0 | 0 | ×1.003 | ×0.982–×1.025 |
| one thread | solo | IdleInterleavedTurn | 2 | 0 | 0 | ×1.017 | ×0.990–×1.044 |
| one thread | solo | BriefIdleInterleavedTurn | 2 | 0 | 0 | ×1.004 | ×1.001–×1.007 |
| one thread | solo | ReceivingMessage | 3 | 2 | 0 | ×0.912 | ×0.907–×1.000 |
| every core | solo | OneMessage | 28 | 0 | 0 | ×0.991 | ×0.929–×1.011 |
| every core | solo | ManyMessages | 24 | 0 | 1 | ×1.000 | ×0.942–×1.110 |
| every core | solo | ManyLongMessages | 4 | 0 | 0 | ×1.003 | ×0.982–×1.008 |
| every core | solo | IdleOneMessage | 28 | 0 | 0 | ×1.002 | ×0.947–×1.031 |
| every core | solo | IdleManyMessages | 24 | 0 | 0 | ×0.981 | ×0.928–×1.092 |
| every core | solo | BriefIdleOneMessage | 28 | 0 | 0 | ×0.998 | ×0.984–×1.026 |
| every core | solo | BriefIdleManyMessages | 24 | 0 | 0 | ×0.998 | ×0.960–×1.073 |
| every core | solo | ContinuousMessages | 13 | 0 | 0 | ×1.000 | ×0.991–×1.007 |
| every core | solo | ContinuousBatches | 7 | 0 | 0 | ×0.999 | ×0.986–×1.031 |
| every core | solo | LentMessages | 13 | 0 | 0 | ×0.998 | ×0.991–×1.013 |
| every core | solo | Interleaved | 1 | 0 | 0 | ×0.989 | ×0.989–×0.989 |
| every core | solo | Collection | 2 | 0 | 0 | ×1.006 | ×1.005–×1.006 |
| every core | solo | LentBatches | 7 | 0 | 0 | ×0.999 | ×0.986–×1.031 |
| every core | solo | ContinuousCollection | 2 | 0 | 0 | ×0.998 | ×0.998–×0.999 |
| every core | solo | MerkleTree | 8 | 0 | 0 | ×1.002 | ×0.970–×1.021 |
| every core | solo | InterleavedTurn | 2 | 0 | 0 | ×0.999 | ×0.975–×1.023 |
| every core | solo | IdleInterleavedTurn | 2 | 0 | 0 | ×0.999 | ×0.953–×1.045 |
| every core | solo | BriefIdleInterleavedTurn | 2 | 0 | 0 | ×0.991 | ×0.972–×1.011 |
| every core | solo | ReceivingMessage | 3 | 2 | 0 | ×0.911 | ×0.909–×1.000 |
| every core | shared-os | ContinuousMessages | 13 | 0 | 0 | ×0.998 | ×0.992–×1.007 |
| every core | shared-os | ContinuousBatches | 7 | 0 | 0 | ×0.995 | ×0.974–×1.010 |
| every core | shared-os | LentMessages | 13 | 0 | 0 | ×1.002 | ×0.958–×1.009 |
| every core | shared-os | Interleaved | 1 | 0 | 0 | ×0.998 | ×0.998–×0.998 |
| every core | shared-os | Collection | 2 | 0 | 0 | ×1.020 | ×1.008–×1.032 |
| every core | shared-os | LentBatches | 7 | 0 | 0 | ×0.995 | ×0.974–×1.010 |
| every core | shared-os | ContinuousCollection | 2 | 0 | 0 | ×1.002 | ×1.001–×1.003 |
| every core | shared-os | MerkleTree | 8 | 0 | 0 | ×0.998 | ×0.972–×1.020 |
| every core | shared-os | ReceivingMessage | 3 | 2 | 0 | ×0.941 | ×0.937–×1.000 |

Slower cells (beyond 5% in bold):

- `blake3-servil-mt|solo|ManyMessages|1`: **+7% slower** (4 pairs)

Ten largest gains:

- `blake3-servil-st|solo|ReceivingMessage|2 μs`: 1.1× faster
- `blake3-servil-mt|solo|ReceivingMessage|2 μs`: 1.1× faster
- `blake3-servil-mt|solo|ReceivingMessage|saturated`: 1.1× faster
- `blake3-servil-st|solo|ReceivingMessage|saturated`: 1.1× faster
- `blake3-servil-mt|shared-os|ReceivingMessage|saturated`: 1.1× faster
- `blake3-servil-mt|shared-os|ReceivingMessage|2 μs`: 1.1× faster


Repeated by name with 16 or more link layouts a side, after the check above marked them from four pairs:

- `blake3-servil-mt|solo|ManyMessages|1`: level over 64 pairs (6.6% slower from four; Mac job 3545)
