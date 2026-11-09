# Counting cycles on the Mac: what a read costs, and how to count around it

November 9, 2026. Measurements on the Mac runner (an M4 Max), jobs
2068–2073. The sources of every program here are in this directory.
BLAKE3-Servil's branch `probe/measure-clocks3` (commit
`32bc2bfe94f45ac12d8cc815308eba436fe8e459`) runs them.

## The question

Our samplers time a call like this: read the thread's counters, read the
wall clock, make the call, read the wall clock, read the counters. Four
cases of the grouped work on `candidate/grouped-noalloc` (`f1393337`)
against its base (`0b5bf7b7`) showed **more cycles** for producing the final
hashes while **wall time and instruction counts stayed level** (Mac job
2062). Two instruments disagreed. Which one to believe depends on what
each one brackets, and on what a read itself costs.

## 1. Every clock on the Mac (zooko/measure-clocks3)

[measure-clocks3](https://github.com/zooko/measure-clocks3) at commit
`5005425b906de1a7a70421681407f1a91b79632b` times a fixed ~1.2 µs function
100,000 times with each clock. Results: [plain run](2068-measure-clocks3/Darwin-Apple_M4_Max-2026-11-09_14-33-48.txt),
[--overthread](2068-measure-clocks3/Darwin-Apple_M4_Max-2026-11-09_14-33-49.txt),
[summary](2068-measure-clocks3/measure-clocks3-summary.txt). They agree
with Zooko's December 2025 run on the same model.

- Every wall clock ticks at 24 MHz: one tick is 41.67 ns. `mach_absolute_time`,
  `CLOCK_UPTIME_RAW`, `CLOCK_MONOTONIC_RAW` and `Instant` are one counter.
  `CLOCK_REALTIME` and `CLOCK_MONOTONIC` round to whole microseconds. The
  `_APPROX` clocks change every few hundred microseconds.
- The thread's own CPU-time clock has the smallest spread, because it
  stops while the thread is interrupted or switched out. Under
  `--overthread` (576 threads on 16 cores) wall readings averaged 60 µs.
  Thread-time readings averaged 1.7 µs.
- Process CPU time steps in whole microseconds and adds every thread.

Our `clocks` crate already reads the raw 24 MHz counter for wall time and
the thread's own counts beside it. These results confirm that choice.

## 2. What one read costs

[bracket-control.txt](2068-measure-clocks3/bracket-control.txt) runs our
samplers' exact sequence around a known amount of register arithmetic,
20,000 times each, through the `clocks` crate.

| | wall ns | thread time ns | cycles | instructions |
|---|---:|---:|---:|---:|
| one wall-clock read | 15 | | | |
| one `thread_selfcounts` read (a system call) | 175 | 160 | 393 | 1,558 |
| the sequence around **no work** | 10 | 151 | 471 | 1,792 |
| around ~1 µs of arithmetic | 842 | 964 | 3,513 | 4,097 |
| around ~10 µs of arithmetic | 6,883 | 6,983 | 31,157 | 24,764 |

So every sample's cycle and thread-time figures carry about 470 cycles and
1,792 instructions that belong to the reads, and the wall figure carries
about 10 ns of them. The instruction count of that overhead is constant to
±1. Its cycle count varies (452 to 476 in the middle 95%, tail to 3,146),
with whatever cache state the work left behind. Inside pure arithmetic the
counters report 4.46 GHz. Around the system calls they report about
3.1 GHz. The kernel's entry and exit add thread time that no counted
cycle matches. That is the mechanism behind "the counters bracket more
than the wall clock does".

For a sample whose work is 0.6 µs (8 messages' final hashes) the overhead
is a quarter of the counted interval. At 36 µs (512 messages) it is 0.4%.

## 3. What root buys ([kpc_probe.c](kpc_probe.c))

Apple's `kpc` interface, in the private kperf framework, configures the
counters only as root. The runner's Mac now allows one program path to run
as root (`tools/runner/README.md`, "Root for a probe").

[Without root](2069-kpc-user/kpc-probe-user.txt) and
[as root](2072-kpc-root/kpc-probe-root.txt):

- A direct read of a counter (`mrs` of Apple's PMC0 and PMC1, or the
  standard `PMCCNTR_EL0`) traps with `SIGILL` in user mode, as root too,
  counting on or off. The hardware has a bit that would allow it
  (`PMCR0.USEREN`); XNU leaves it off and exposes no switch. On macOS,
  every counter read is a system call.
- `kpc_get_thread_counters` costs 578 ns, 2,391 cycles and 12,370
  instructions per read: four to six times `thread_selfcounts`. For a
  cheap read, root buys nothing. `thread_selfcounts` is the floor.
- Root buys the **events**: the Mac's database has 104, and a thread can
  count eight of them beside cycles and instructions. They include L1 data
  and instruction misses, TLB misses, page walks, dispatch stalls
  (`MAP_STALL`), branch mispredictions by kind, pending interrupts, and the
  SME unit's waits (`LDST_UNIT_WAITING_SME_ENGINE_MEM_DATA`,
  `SCHEDULE_WAITING_SME_ENGINE_REG_DATA`, `SME_ENGINE_SM_ENABLE`).
  Validated on known work: ~10 µs of a dependent multiply-add chain shows
  44,400 cycles, 36,600 of them dispatch-stalled, no cache misses, two TLB
  misses, no interrupts.

## 4. Dilution, and the events around the final hashes

The way to make 470 (or 2,400) cycles of overhead negligible is to count
more work per pair of reads. [events_sampler.rs](events_sampler.rs)
prepares **eight sets** of hashers a shape, feeds each set its messages in
4 KiB pieces in the canonical caller order (hasher (i·61) mod n for
piece i), and then counts the final hashes of all eight sets inside one
pair of reads, 64 samples a shape. A "nothing" shape brackets no work, so
the overhead is on record beside the results. [events_judge.rs](events_judge.rs)
applies the servil projects' one comparison rule (`clocks::summary::discoveries`:
a paired t-test per cell over pairs of runs, false discovery rate 5% over
all cells, 3% margin) and refuses runs whose output hashes differ. On
synthetic runs it finds a planted 10% and leaves the rest level.

Job 2073 ran four pairs of runs, old and new in alternating order, each
pair with a fresh link layout, as root, with these events: cycles,
instructions, `L1D_CACHE_MISS_LD`, `L1I_CACHE_MISS_DEMAND`, `L1D_TLB_MISS`,
`MAP_STALL`, the three SME-unit waits, `INTERRUPT_PENDING`.
[Judgment](2073-finish-events/events-judgment.txt); raw samples in the
same directory; [identity](2073-finish-events/events-identity.txt).

Per message, old → new, with the judge's verdict:

| shape | cycles | instructions | L1D misses | TLB misses | dispatch stalls | wall ns |
|---|---:|---:|---:|---:|---:|---:|
| mixed 128 × 16 KiB | 301 → 318 (+5.2%, level) | 1336.7 → 1336.9 | 21.8 → 21.5 | 4.2 → 4.6 | 157 → 172 (+9.0%, level) | 73 → 82 (level) |
| mixed 24 × 32 KiB | 291 → 297 (+1.8%) | 1391.9 → 1391.1 | 21.6 → 21.3 | 6.9 → 7.0 | 128 → 136 (+6.2%) | 79 → 83 |
| mixed 8 × 16 KiB | 322 → 333 (+3.2%) | 1525.1 → 1524.6 | 19.3 → 18.9 | 5.8 → 5.5 | 125 → 133 (+6.6%) | 83.6 → 83.6 |
| shared 128 × 32 KiB | 309 → 327 (+6.1%, level) | 1336.8 → 1337.0 | 21.8 → 21.5 | 4.7 → 5.3 | 163 → 182 (+11.5%, level) | 77 → 81 |
| plain 128 × 16 KiB | 295 → 284 (−3.6%) | 1337.1 → 1336.6 | 21.8 → 21.4 | 3.9 → 3.9 | 149 → 140 | 71 → 63 (**FEWER**) |
| plain 24 × 32 KiB | 290 → 300 (+3.6%) | 1394.4 → 1390.9 | 21.5 → 20.8 | 6.8 → 6.8 | 125 → 138 (**MORE**, +10.4%) | 83 → 67 (**FEWER**) |
| plain 512 × 16 KiB | 554 → 498 (**FEWER**, −10.3%) | 1329.4 → 1328.0 | 23.1 → 22.6 | 34.3 → 33.8 | 409 → 355 | 127 → 112 |
| nothing (per sample) | 2,780 → 2,600 | 12,563 → 12,566 | 1.7 → 1.3 | 5.0 → 4.8 | 89 → 75 | 8.9 → 6.8 |

Every cell is level except the ones marked. Instruction-cache misses are
below 0.01 per message everywhere; the SME-unit waits and pending
interrupts are zero in every run (the final hashes use neither).

What this says:

- **The instructions are identical**, to 0.1%, in every shape. The new
  code does the same work.
- **No memory cause.** L1 data misses, TLB misses and instruction misses
  are the same or fewer in the new code, in every shape.
- **Where cycles rise, dispatch stalls rise by the same count**: mixed
  128: +16.5 cycles, +15.5 stalls per message; shared 128: +18.6 and
  +18.5; mixed 8: +10.4 and +8.4. The core had the instructions and could
  not issue them: a scheduling or dependency effect in the new code's
  final-hash path, with no misses behind it.
- **With the reads diluted eight-fold and four fresh pairs, none of the
  four earlier cases reaches a verdict.** Their point estimates still lean
  up (1.8% to 6.1%). Four pairs are few; this run does not erase job 2062,
  and it does not confirm it.
- **The two shapes that lost in the VM gain here**: plain 24 × 32 KiB
  wall 83 → 67 ns (FEWER), plain 512 × 16 KiB cycles 554 → 498 (FEWER).
  The VM's losses are the VM's to explain; they do not appear on the Mac.

## 5. What to carry forward

1. Keep `thread_selfcounts` for cycles and instructions in routine
   measurements; it is the cheapest read this kernel offers (recorded in
   the `clocks` crate's notes, bench-hashes `decb4468c84391456fcd6d5174b2874232f623cd`).
2. Make every counted interval long against 470 cycles: a batch of calls,
   or several prepared sets finished in one bracket, as here.
3. Use the root interface for diagnosis: when cycles move and instructions
   do not, count misses, TLB, stalls and the SME waits around the same
   diluted work, old against new, and read which one moves with the
   cycles.
4. Treat the instruction count as the check that the work is unchanged.
