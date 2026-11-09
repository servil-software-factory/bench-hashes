# Native2062: grouped updates built in place

Exact old source: `0b5bf7b71cb5f850067fa4182d9a3c724d7e5710`.
Exact new source: `f13933376c8b8236adaeffe6aa9d25ad25f2af24`.
Executing adapter: BLAKE3-Servil `12a13061548ddf60c6233797fcc484adb4696a94`,
`tools/studies/mac_cycles.py`; a copy is in each phase's out directory.
The adapter extracts the unchanged Rust sampler/judge from the included
`instrument.sh` (Sam's bench-hashes source
`0b2e9892f6cb30e38ddce5ba82ad7b8d85f060d0`). Clocks/judgment code:
bench-hashes `9994de190259648ede82f7c5e4dbac4f1534a41d`.

`aa/out` is the identical-library control; `ab/out` is the paired change.
Each has exact identities, feature/digest rejection controls, restorations,
four paired layouts, and Rust judgments. `old-N.tsv`/`new-N.tsv` carry
run means; their matching `.err` files carry per-kind time, core cycles
and instructions from consumed counters. Missing values are `NA`.
Judge those figures with the included canonical Rust reader/judge.

AC/HighPower, VM idle. A/A:256Level judgments per phase. Complete-cycle
wall time:51Faster/13Level/0Slower. Update:52Faster/12Level/0Slower.
Finish wall time:6Faster/58Level/0Slower. Four finish core-cycle cells
remain Slower despite Level wall/instructions: mixed128×16KiB(+3.4%),
mixed24×32KiB(+6.5%), mixed8×16KiB(+4.1%), shared128×32KiB(+4.0%).
Their state/work explanation and the90s mutation budget remain open.
This is partial recovery against the safe selector, not acceptance against
the earlier equality-dependent code. Candidate branches remain prepared.

Only text evidence is included. Build trees and executable files stay local.

[Sam’s offline attribution](https://github.com/samshackleton-tech/bench-hashes/blob/843d8382b3ae7444aa469ca780286c015501f5f6/process/fast-feedback/independent-integration/reviewed-base/BOUNDED-DIAGNOSIS.md)
finds all four cycle findings P-only, with slightly lower cycles/ns.
Counters bracket more work than wall time: their P-time-minus-wall gaps
grow 2.82–7.11ns/message. A matched reader/timer-bookkeeping control is
still needed. The four Slower cycle verdicts remain recorded.
