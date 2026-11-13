# Working on bench-hashes

This file holds what you need to change bench-hashes, or the BLAKE3 fork
it measures, and keep your results comparable with ours.
[METHODOLOGY.md](METHODOLOGY.md) explains the measurement design.

## Layout

- `src/main.rs`: the benchmark (contenders, timing, statistics, the text
  report, `compare` and `regress`); `src/check.rs`, `bench-hashes check`;
  `src/map.rs` and `src/map.html`, the map; `src/overview.rs` and
  `src/overview.html`, the overview page; `src/chart.rs`, the charts;
  `src/b3sum.rs`, `bench-hashes b3sum`; `src/leaves.rs`, `bench-hashes
  leaves`; `src/crowd.rs`, `bench-hashes crowd` (many programs at once); `src/fingerprint.rs`, a dirty tree's fingerprint, shared with
  `build.rs`; `src/harness_tests.rs`, tests of the benchmark's own work
  (zeroing, orders, the output directory); `src/diag_alloc.rs`,
  diagnostic allocators for the cache-line study (#65), never a
  measurement configuration.
- `build.rs`: embeds the provenance (this repository's commit and state,
  and each contender crate's version and source).
- `tools/`: `b3sum-contenders.py` builds
  b3sum's contenders; `gen-ver.py` makes a release; `graph-check/map.js`
  drives the map in Chromium (its README says how); `require_nextest.rs`,
  included by every crate's tests, fails under plain `cargo test` and
  says to use nextest.
- `bench-hashes compare OLD.tsv... -- NEW.tsv...`: compares runs'
  samples files in pairs (the i-th old with the i-th new), each cell
  judged by its pairs' ratios as `regress` judges.

## Build and test

The tests run under [cargo-nextest](https://nexte.st), each in its own
process (`cargo test` stops at once and says so).

```sh
cargo nextest run --release              # the code's tests: under two seconds
cargo nextest run --release -P machine   # the machine's: clocks, counters, children; seconds each
cargo run --release -- --quick     # seconds; a full run takes minutes
node tools/graph-check/map.js benchmark-results/FOLDER/bench-hashes.map.html
```

## Adding a contender

Copy what SHA3-256 does: `grep -n Sha3_256 src/main.rs build.rs` lists
every place, and the compiler names any match you miss.

1. Add your crate to `Cargo.toml`, and its provenance to `build.rs`
   (`emit_required_package`, read back as a `*_SOURCE_INFO` constant).
2. Add a variant to `Algorithm` and to `Algorithm::ALL`, with its key
   (for `--contenders`), name, one-line description, colour, provenance,
   and mode (single-threaded or multithreaded). A hash with both modes
   is two contenders, as BLAKE3-Servil st and mt are.
3. Give it three calls, each the plain entry point your users call: one
   message (`one_message_call`), a batch of 64-byte messages
   (`hash_batch`; a loop over its one-message call when it has no batch
   entry point), and its incremental API, for long messages read in
   pieces (`hash_stream`) and for many messages at once
   (`hash_interleaved`).
Then `cargo nextest run --release` and
`cargo run --release -- --contenders yours,sha256-ring,blake3-servil-mt`.
The harness does the rest: interleaving, the use cases and scenarios,
statistics, the report, and the map.

## Working on BLAKE3-Servil alongside

Measure before building: `bench-hashes check --fork ../BLAKE3 --against
next --probe "3 KiB,brief idle 3 KiB"` gives each named cell's ratio in
about three minutes (three fresh pairs, no verdict). Once the change is
built, the same names with `--points` are its gate (four pairs, then the
second stage on its suspects). `check` prints the diff's files and the
items its hunks sit in first; its author reads that summary and names the
cells the change can move. With no scope named it stops there, before any
build. `--cells all` measures every point, for a release or a change its
author judges can act anywhere. A change to the library carries its
probe's line in its pull request.

### When a check reads a cell slower

There is no rule for this, on purpose (Zooko, November 12, 2026: goals,
and good judgment in their service). The goals:

- **A user is never surprised by a slowdown we could have known.** A
  real one is fixed, or said plainly where users read (the changelog),
  with an owner.
- **We do not fool ourselves.** A reading the code did not cause is worth
  understanding far enough to stop it recurring, because unexplained
  flags teach everyone to skip flags, the real ones with them.
- **Releases come soon.** An hour spent chasing a cell is an hour, and is
  weighed against what the chase can buy.

Whoever lands the change or cuts the release decides what a slower cell
means for it, and says why in a sentence where the others will read it
(the pull request, the release's issue). What bears on the call: how
large the reading is, whether it repeats, how many machines agree,
whether a user's call can reach the cell, what finding out would cost,
and what being wrong would cost (a release is harder to take back than a
landing). When the call is hard and the stakes are high, ask.

These questions have found causes quickly, each a check of a few minutes
on the machine that read the cell:

1. **Does it repeat by name?** Run the check again with the cell and its
   neighbours as `--points` (and `--contenders` for one form). `check`
   takes its 64-pair second stage only when the first finds no slower
   cell and names suspects. The report says how many pairs ran and how
   far they spread ("from -27.0% to +194.6%"). Keep the full-run reading
   beside it: selecting cells changes the surrounding workload and can
   change their costs. A level rerun means the rule found insufficient
   evidence of a slowdown in that run's context; it does not establish
   equal speed or erase the earlier reading. Compare the ratios and
   spread, and repeat the original context when resolving that reading.
2. **Whose is it, the library's or the bench's?** A check against a past
   release crosses two benches (`--old-bench`). Change one thing at a
   time: the two library commits under one bench; then one library under
   the two benches (`--new C --against C --old-bench OLD`).
3. **Is it the logic, or how this one build compiled?** Run the same
   check with both sides compiled another way:
   `CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
   bench-hashes check ...`. A cost that appears under both builds has
   broader evidence. A cost that depends on the build calls for examining
   inlining, layout and code generation. Both are costs a compiled
   program can meet; these readings alone do not identify the cause.
4. **Which code changed?** `check`'s first lines name the functions whose
   compiled size differs between the sides, the changed lines' own and
   the others. The others, on the cell's path, are the suspects of a
   compiler's effect.

Cases of November 12, each on the data branch under `2026-11-12/`:
- Step 1 and 2 (`old-bench-pinned-9275f`): five cells of a gate against
  0.8 read 3 to 6.5% slower. One bench, two libraries: level. One
  library, two benches: 21% slower in one run, level in the next. The
  cause was this bench's: checks across two benches ran unpinned.
- Step 3 (`stream-1mib-epyc-compile`): a deletion of code the stream
  did not run read the stream 8 to 15% slower in six checks, and level
  compiled the second way; a change that really cost it (the long ring,
  BLAKE3 #349) read 11 to 30% slower compiled the second way too.

A compiler's effect is still an effect: a program compiled once meets it.
Inspect the generated code before choosing a source change, such as an
inlining annotation. Evaluate that change under the workload context that
exposed the cost, across link layouts and the relevant build settings.
Keep any remaining cost visible in the decision.

Nothing else runs on a machine while it measures: queue the check under
the machine's lock and read its files when it has ended.

The BLAKE3-Servil contenders (`blake3-servil-st`, `blake3-servil-mt`) use
the crate `blake3-servil`, a git dependency on
[github.com/servil-software-factory/BLAKE3](https://github.com/servil-software-factory/BLAKE3) at
the commit whose full hash `Cargo.toml` names (`rev`). The instrument,
`clocks/` (every clock a measurement reads), is part of this repository,
so a run's bench commit names the one that timed it, on both sides of a
comparison.
To measure your own checkout of BLAKE3-Servil, keep this repository beside
it (anywhere: neither holds the other) and name BLAKE3-Servil's checkout:

```sh
git clone --branch next https://github.com/servil-software-factory/BLAKE3
git clone https://github.com/servil-software-factory/bench-hashes
cd bench-hashes && cargo run --release -- check --fork ../BLAKE3 --points "3 KiB,brief idle 3 KiB"   # the named cells of BLAKE3-Servil's working tree against its HEAD
cd bench-hashes && cargo run --release -- check --fork ../BLAKE3 --new my-branch --against next --points "3 KiB"   # a branch by name, whatever the tree has checked out
cd bench-hashes && "$(cargo run --release --quiet -- build-against ../BLAKE3)" --quick   # one run of it
```

`check` builds in directories of its own (BLAKE3-Servil's `target/check/`), and
`build-against DIR` in DIR's `target/bench-hashes-build/` (a directory of
DIR's own there, under a shared `CARGO_TARGET_DIR`), each from a copy of
this repository whose `Cargo.lock` points at a checkout of BLAKE3-Servil, and leaves
your checkouts' files as they were. Run the executable from a
scratch directory (it writes `benchmark-results/` there); the report
names the BLAKE3-Servil checkout's commit and whether its tree was clean.
`build-against` prints an executable in a directory named for both
builds, as `bench-hashes provenance` prints them: each checkout's commit
and its tree's state (`clean`, or a dirty tree's sigil). So a path kept
from an earlier build runs what it names. `regress` and `compare` print
each side's builds the same way, and say so when both sides are one
build. BLAKE3-Servil's `CONTRIBUTING.md` covers the
fork's own tests and its performance-regression check, which runs this
benchmark.

To move the pin of BLAKE3-Servil, set its `rev` in `Cargo.toml` to the full commit
hash and build: Cargo updates the lock. `build-against` and relinking
replace BLAKE3-Servil and keep this repository's `clocks/`. A change
to `clocks/` is a change to this repository, its tests among the ones
`cargo nextest run` runs.

## Rules that keep results comparable

- **Contenders are black boxes.** The benchmark lists a contender, calls
  its plain entry point (single-threaded, multithreaded, a batch call, or
  its incremental API for messages in pieces) with no pool, cap,
  or wrapper of its own. A contender without a batch entry point hashes a
  batch as `for m in batch { hash(m) }`.
- **What the benchmark asks of BLAKE3-Servil is frozen.** `FROZEN.md` lists
  each use case's calls and call pattern, and a test compares it with the
  code; changing it is Zooko's decision, recorded there.
- **The benchmark checks no digests.** Each crate's own tests establish
  that it is correct; the benchmark only keeps every digest from being
  optimized away (`black_box`).
- **Results name their code.** Commit before you publish a result: a
  report whose provenance says `dirty-…` (the uncommitted changes' BLAKE3
  hash, as a sigil) measured uncommitted code. Sigil's Python module
  turns one into hex: `python3 -c 'import sys, sigilcode; print(sigilcode.decode(sys.argv[1], 32).hex())' SIGIL`,
  from a checkout of johnservil/sigil's `python/`.
- **BLAKE3-Servil's regression check runs `bench-hashes regress OLD... -- NEW...`**,
  which runs the two executables with `--contenders`, `--points`, and
  `--rounds` and reads their samples files: the rule lives here, so a
  change to the format changes its reader in the same change.

## Results from other machines

Pull requests that add a machine's results are welcome: one folder under
`benchmark-results/`, the files a run writes there, from a clean commit
of this repository. A second machine of a kind we already have gets a
folder name of its own.

## Text in the map and the report

The map and the report are read by newcomers holding only the page as
well as by regulars: text in them uses words a newcomer knows or the page
introduces, describes the page as it is, and is computed from the run's
own data (it names only contenders the run has). Details go behind the
page's doors (an opened chart, its values under the pointer, the links to
the documentation).
`AGENTS.md`, "Presentation: write each page for a reader who holds only
the page", has the whole practice. Check a change with
`node tools/graph-check/map.js` and by looking at the page.

## Whose move it is

Every open issue and pull request has one assignee, the person whose
move is next. Finish your move, say in a comment what the next move is,
and assign the person who makes it. Close the item when nothing is left.
A workflow assigns the author to an item left with no assignee and asks
when there are several.

## Maintainers' notes

`AGENTS.md` holds the maintainers' principles and the repository's shape;
the open work is in the issues. Contributing needs neither.
`METHODOLOGY.md` explains why the measurement works as it does,
should you want to change it; earlier findings are in `git log`, and in the
notes the tree held until commit `aadeca9d`.

## Releases

The overview shows one release a page, so a release is the moment every
machine runs again. In order:

1. **What it carries:** each entry under FROZEN.md's "Changes for the next
   release", and the pin of BLAKE3-Servil (its commit), a commit that
   passed BLAKE3-Servil's own checks.
2. **Checks on main:** `cargo nextest run --release`, `cargo nextest run
   --release -P machine` on the release machines, `cargo fmt --check`,
   and a `--quick` run that reports no busy window.
3. **The tag:** `python3 tools/gen-ver.py X.Y.Z` from a clean tree: one
   commit setting the version to X.Y.Z+<HEAD's hash>, and its tag.
4. **Every machine runs again at the tag,** each on a quiet machine
   (`check` and `regress` refuse a busy one before they start;
   `--when-quiet` waits for it; a script's other runs start with
   `bench-hashes quiet --wait`, the same rule),
   plugged in, with the CPU governor at `performance` (a tuning service
   such as `tuned` can reset it mid-run). A machine's run is:
   - BLAKE3-Servil's release check and, beside it, the same check with the same
     library on both sides (`--against` the measured commit, `--old-bench`
     the previous bench). This control tests benchmark-version effects
     with that library. Read its ratios and uncertainty alongside the
     release comparison: a level control leaves small effects unresolved,
     and both library and benchmark changes may contribute. Record both
     comparisons' benchmark commits and workload scopes, which can differ.
     Where the library's API has changed since its last release, the
     release check takes `--old-bench` too, a bench that builds against
     that release; the issue for the release names both commands;
     the same-library check's "instructions moved" line must name no
     cell: the same code retires the same instructions a unit in every
     single-threaded solo cell (within 1% on the i7-12700K, the
     sampler's own reads included), so a cell named there is the bench
     measuring different work in two runs, its bug to find before the
     release;
   - the full run, built against BLAKE3-Servil's checkout, never through a
     `--config` patch (Cargo's lock can build the pin instead):
     `bench-hashes build-against ..`, then the executable it prints, with
     `--all --expect-servil COMMIT`, from a scratch directory;
     on Linux, as root where you can (`sudo` on the executable, the
     scratch directory writable by root), so the run holds the cpuidle
     switch and measures the controlled after-pause cells; its header's
     `idle control` line says whether it did;
   - the b3sum run, `LAYOUTS=4 python3 tools/b3sum-contenders.py`, with
     `--files` on the machine's own storage. On tmpfs (a VM's `/tmp`, for
     one) nothing is read from storage, and the run reports its
     read-from-storage cells as not measured. A VM's storage is a disk
     image on its host, which may serve the reads from its own memory;
     that is what a program in the VM meets.
   - the crowd run, `bench-hashes crowd` from the same executable: one
     program a CPU, so a release's number for the pool under
     oversubscription is on record (`crowd.samples.tsv`).

   Check each report names the commit you meant, before you read its numbers.
5. **The results:** each machine's folder under `benchmark-results/` as
   the run wrote it, with SHA-256 and BLAKE3 manifests.
6. **The overview**, once the machines are in: `cargo run --release --
   overview index.html benchmark-results` (its test fails while it is
   stale).
7. **FROZEN.md:** the released entries leave "Changes for the next
   release" and stand under the release's version.
