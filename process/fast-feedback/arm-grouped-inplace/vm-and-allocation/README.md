# VM grouped-cycle findings and allocator scope

Sources: old `0b5bf7b71cb5f850067fa4182d9a3c724d7e5710`, new
`f13933376c8b8236adaeffe6aa9d25ad25f2af24`. Adapter BLAKE3-Servil
`12a13061548ddf60c6233797fcc484adb4696a94` extracts the unchanged
canonical Rust sampler/judge. Each phase retains identities, source,
paired means, controls and judgments. A/A completed before the A/B
setup's GitHub connection failure. The complete A/B then used the exact
committed local Git objects; original Cargo edits were preserved.

## Timing

No native timing ran during VM work. A/A:128Level judgments per phase.
VM complete-cycle wall:32Faster/32Level/0Slower; update30Faster/34Level.
Finish wall and CPU each have62Level/2Slower:

| Plain case | Old finish ns/message | New | Wall change |
|---|---:|---:|---:|
|24×32KiB,4KiB pieces|83.1981|89.1585|+7.2%|
|512×16KiB,4KiB pieces|71.8756|87.2244|+21.4%|

The VM supplies wall/CPU time, not per-kind core cycles/instructions.
Both finish losses remain open. Native2062's separate four finish-cycle
findings also remain open. See sibling2062 for its native rows.

## Stored-state diagnosis

`state/` holds the selected release-test logs and a one-extra-byte control.
The probes are BLAKE3-Servil `566f66ec90e08e9ef908b402253962920c543e9f`
(old) and `aa8b1631472e2404a87e063826f302fc53aa3bb7` (new). They log
only lengths/counters, public flags and platform, with golden output checks.
Under identity caller order, all hashers on both slower shapes and two
controls have staged length 0, stack depth 2, chunk counter 16 or 32,
chunk bytes 0, mode flags 0, platform 7 and initial counter 0. One added input byte moves staged length
to 1: the observation distinguishes shape changes. The initial root-block
counter covers XOF rather than this root-hash call, so the committed
probes use shape alone. The matched metadata shows no stored-shape difference in this identity-order
control. The timed sampler uses a 61-permuted caller order. Code generation,
caller order and machine state still need attribution. This is not a
cache-cause verdict.

## Rust allocator observation

Probe `eddedc1ee172195cb1b6f9678addd56c7a37dbde` retains f139's runtime.
Its new large-cycle assertion intentionally fails: 25 allocations,
0 reallocations, 24 frees and 26,752 requested bytes. The observed first
shared-key case runs three cycles of four 4MiB messages, with plain,
keyed and derived modes.
Attribution puts them on the caller, with 0 observed named-worker requests.
Source leads are `lanes::tree_cvs`' two vectors and first-use `job_source`.
The unchanged small grouped scopes remain allocator-free as tested.

The meter detects a 4096B allocation planted in the real pool worker.
Disabling its cross-thread recording fails the independent positive control.
Restoration passes all seven green scopes; the large-cycle test stays red.
Observation covers Rust GlobalAlloc requests on the caller and named
library OS threads. Native C/C++/Objective-C/GPU-driver allocators and
unnamed bootstrap work are outside that scope.

Main/next and published releases remain unchanged. Mutation budget and
finish findings keep the grouped change prepared.
