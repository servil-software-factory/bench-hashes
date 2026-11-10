//! `bench-hashes check --fork DIR [--against REV] [--new REV] [--old-bench REV] [--points POINTS | --probe POINTS | --cells all] [--layouts N] [--keep DIR] [--when-quiet]`:
//! did a change to BLAKE3-Servil hurt its speed in the cells its author names?
//! Run from a checkout of bench-hashes, it compares BLAKE3-Servil's working
//! tree (or `--new REV`) with a commit (`--against`, HEAD by default) and
//! prints the cells judged slower or faster, with their ratios; exit 0 when
//! no named cell is slower, 1 when one is, 2 when no scope was named or a
//! run's load left no verdict, 3 when a side's run stopped (a panic, a
//! crash: `regress` names the side and quotes its last words).
//!
//! What a check does, every time, with nothing to choose:
//! 0. Builds bench-hashes against both sides in turn from one path and
//!    compares the two builds (src/samecode.rs): where they hold the same
//!    code, the commits and the panic locations' line numbers aside, the
//!    verdict is "same code", exit 0, and nothing is measured (bench-hashes
//!    #125). Otherwise it names the functions that differ and goes on.
//! 1. Builds bench-hashes twice, against BLAKE3-Servil at each side (a commit
//!    of the working tree as `git add -A` would stage it, and the commit
//!    checked against), in worktrees of its own under `target/check/`, and
//!    relinks each side once per pair with its functions in a fresh random
//!    order: a binary's layout alone moves cells several percent (x86: 8-10%),
//!    the same for every run of that binary, so no sampling of one binary
//!    sees it; two sides with different layouts each pair do, and their
//!    seeds stay apart so identical code still meets a new layout in every run.
//! 2. Takes its scope from its caller. The check first prints a short
//!    summary of the diff (its files, its added and deleted lines, and the
//!    items its hunks sit in). Its author reads that summary, thinks through
//!    which cells the change can move, and names them with `--points`, in
//!    the labels `--points` takes ("2 KiB,idle 2 KiB,brief idle 3 KiB").
//!    `--probe POINTS` names the same scope for a three-pair survey with no
//!    verdict, before the change is built. `--cells all` measures every
//!    point, for a release or a change whose author judges it can act
//!    anywhere. With no scope named, the check prints the summary and
//!    stops before any build: choosing cells is the author's judgment, and
//!    the check carries no second guess at it. It prints how many points
//!    it measured and how many it left unmeasured, so the choice stays in
//!    the record.
//! 3. Runs `bench-hashes regress` over the pairs (LAYOUTS new links a side,
//!    each run FIRST_RUNS times): servil st and servil mt
//!    (the code that changes) at the regression check's visits
//!    (TO_CHECK_FOR_REGRESSIONS), judged all at once (clocks::summary).
//! 4. When no cell is slower, runs the cells `regress` names as suspects
//!    (level, the pairs' mean beyond the margin; at most 8) again on
//!    SECOND_PAIRS fresh pairs (SECOND_LAYOUTS new links a side, each run
//!    SECOND_RUNS times), judged among
//!    themselves: judged among every cell, a slowdown at one point is
//!    level unless it is many times its pairs' spread (bench-hashes #66).
//!
//! A change to BLAKE3-Servil's API changes the bench with it, so one bench builds
//! against one side alone: `--old-bench REV` builds the old side with this
//! bench at REV (a commit of this bench's repository, in a worktree under
//! `target/check/`; a bench that builds beside BLAKE3-Servil, so 1f4dfd63 or
//! later), the new side with the bench as it is. The cells are
//! keyed by use case and point, not by call, so the two sides' cells meet.
//!
//! It replaces BLAKE3-Servil's `tools/perf_regress.py` (October 19, 2026,
//! Zooko: one tool that does everything a change needs, at no cost to the
//! one who made it) and `REGRESS_POINTS`, a fixed list of five cells that
//! was blind to the cells after a gap and to the stream, where BLAKE3-Servil
//! had regressed before.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Links a side a check takes (each run FIRST_RUNS times). Measured October 20,
/// 2026 (i7-12700K; the data branch, 2026-10-20/check-validation), 4 pairs
/// against 8: a hash() plant 6% a call caught in 48 cells against 60; a
/// plant in the pool's split path in 22 of its 28 cells against 28; the
/// A/A over the same 146 cells no cell slower either way; a check that
/// chooses 73 points 6 minutes against 14. Every plant is still found;
/// `--layouts 8` sees more of its cells.
const LAYOUTS: usize = 4;
/// Pairs of the second stage: when the first finds no cell slower, the
/// cells it names as suspects (level, beyond the margin) run again on
/// fresh layouts and are judged among themselves. Judged among the 75
/// cells a hash() change reaches, a lone cell's paired t on 4 pairs must
/// pass about 15, and a slowdown planted at one point (+20% of calls,
/// its cell +23% to +77% in every pair) was judged level; among 8 cells
/// on 16 pairs the bar is about 3 (bench-hashes #66; the data branch,
/// 2026-10-24/check-one-point).
const SECOND_LAYOUTS: usize = 16;
/// Runs of each second-stage layout: its pairs are SECOND_LAYOUTS times
/// this. A relink costs about 18 s, a run of the suspects 1 to 3 s,
/// and a layout's own bias is a small part of a cold cell's spread (seven
/// A/A checks of the same layouts: their cells' ratios correlated 0.03).
/// At 16 pairs, #119's +3% to +11% on cold one-message calls passed 3
/// checks of 3 whose suspects held them; on 64 pairs from the same 16
/// layouts, about 2.5 minutes more, its cells were caught 3 of 3, and 3
/// A/A runs found no cell slower (the data branch,
/// 2026-10-31/check-errors-i7).
const SECOND_RUNS: usize = 4;
const SECOND_PAIRS: usize = SECOND_LAYOUTS * SECOND_RUNS;
/// Runs of each first-stage layout: once, so the first stage is LAYOUTS
/// pairs. Its pairs choose the second stage's suspects, ranked by evidence
/// (clocks::summary::Judgment::rank). Each run twice (8 pairs) cost about
/// 12 minutes more a check and bought nothing: read from each check's
/// first 4 pairs alone, its suspects held as many of #119's slowed cells
/// (by evidence 5 against 5), and every check's suspects held at least
/// one; the misses fell in the second stage. At 4 pairs, ranked, with 64
/// second-stage pairs, plant/spin0 was caught 2 of 2 and #119 2 of 2 (the
/// i7; Devon Jonte, bench-hashes #98, the data branch
/// 2026-11-01/links-019-i7; Zooko, November 1: "I love saving 12 minutes
/// without losing sensitivity").
const FIRST_RUNS: usize = 1;
/// A probe's layouts a side: three pairs, the least the rule reads, with
/// the points named rather than found. About five minutes with the builds
/// cached: the measurement before a change is built, in place of the hour
/// of a check after (Zooko, November 5, 2026: measure first, and make the
/// measurement cheap).
const PROBE_LAYOUTS: usize = 3;

/// `exes`, each `times` times, in turn: the same layouts run again cost a
/// run each, where a new layout costs a relink.
fn each_run(exes: Vec<String>, times: usize) -> Vec<String> {
    exes.iter()
        .cycle()
        .take(exes.len() * times)
        .cloned()
        .collect()
}
/// The two sides' contenders: the code a change to BLAKE3-Servil changes.
const SUBJECTS: [&str; 3] = [
    "blake3-servil-st",
    "blake3-servil-mt",
    "blake3-servil-pipeline",
];

/// The contenders a comparison across two benches runs: those the older
/// bench knows too, from its `--list` (each line starts with a key). A
/// contender one side lacks has nothing to be compared with: the pipeline
/// joined after BLAKE3-Servil 0.8, whose bench stops on its name.
fn shared_subjects(old_list: &str) -> Vec<&'static str> {
    let known: BTreeSet<&str> = old_list
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    SUBJECTS
        .into_iter()
        .filter(|subject| known.contains(subject))
        .collect()
}

/// Equal library trees settle a comparison only when both use this bench.
/// An explicit old bench is a comparison of the instrument itself.
fn unchanged_inputs(library_changed: bool, old_bench: Option<&str>) -> bool {
    !library_changed && old_bench.is_none()
}

pub fn command(arguments: &[String]) -> i32 {
    let usage = "usage: bench-hashes check --fork DIR [--against REV] [--new REV] [--old-bench REV] [--points POINTS | --probe POINTS | --cells all] [--layouts N] [--keep DIR] [--when-quiet]";
    let (mut against, mut all, mut layouts_a_side, mut keep) =
        ("HEAD".to_string(), false, LAYOUTS, None::<PathBuf>);
    let mut when_quiet = false;
    let mut probe = None::<String>;
    let mut named = None::<String>;
    let mut new_rev = None::<String>;
    let mut old_bench_rev = None::<String>;
    let mut fork = None::<PathBuf>;
    let mut it = arguments.iter();
    while let Some(name) = it.next() {
        match name.as_str() {
            "--fork" => {
                fork = Some(PathBuf::from(it.next().unwrap_or_else(|| {
                    panic!("--fork needs BLAKE3-Servil's checkout\n{usage}")
                })))
            }
            "--against" => {
                against = it
                    .next()
                    .unwrap_or_else(|| panic!("--against needs a revision\n{usage}"))
                    .clone()
            }
            "--old-bench" => {
                old_bench_rev = Some(
                    it.next()
                        .unwrap_or_else(|| panic!("--old-bench needs a revision\n{usage}"))
                        .clone(),
                )
            }
            "--cells" => {
                let value = it
                    .next()
                    .unwrap_or_else(|| panic!("--cells takes `all`\n{usage}"));
                assert_eq!(value, "all", "--cells takes `all`\n{usage}");
                all = true;
            }
            "--layouts" => {
                layouts_a_side = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&n| n >= 3)
                    .unwrap_or_else(|| panic!("--layouts takes a count of 3 or more (regress's rule needs three pairs)\n{usage}"))
            }
            "--keep" => {
                keep =
                    Some(PathBuf::from(it.next().unwrap_or_else(|| {
                        panic!("--keep needs a directory\n{usage}")
                    })))
            }
            "--when-quiet" => when_quiet = true,
            "--new" => {
                new_rev = Some(
                    it.next()
                        .unwrap_or_else(|| panic!("--new needs a revision\n{usage}"))
                        .clone(),
                )
            }
            "--points" => {
                named = Some(it.next().unwrap_or_else(|| {
                    panic!("--points takes the points to measure, as --points names them (\"3 KiB,brief idle 3 KiB\")\n{usage}")
                }).clone());
            }
            "--probe" => {
                probe = Some(it.next().unwrap_or_else(|| {
                    panic!("--probe takes the points to measure, as --points names them (\"3 KiB,brief idle 3 KiB\")\n{usage}")
                }).clone());
                // Three fresh pairs: the least the rule reads, the most a
                // probe needs to say whether an idea is worth building.
                layouts_a_side = PROBE_LAYOUTS;
            }
            other => panic!("unknown option {other}\n{usage}"),
        }
    }
    assert!(
        [all, named.is_some(), probe.is_some()]
            .into_iter()
            .filter(|&set| set)
            .count()
            <= 1,
        "--points, --probe or --cells all: one scope\n{usage}"
    );
    // The machine first, before a build: a check on a busy machine measures
    // for an hour and abstains. --when-quiet waits for one instead.
    if when_quiet {
        clocks::load::wait_until_quiet();
    } else if let Err(reason) = clocks::load::quiet_for(10) {
        println!("check: {reason}. No verdict (exit 2); run when nothing else runs on the machine, or with --when-quiet to wait for that.");
        return 2;
    }
    let bench = bench_checkout("check", usage);
    let fork =
        fork_checkout(&fork.unwrap_or_else(|| {
            panic!("check: --fork DIR names BLAKE3-Servil's checkout\n{usage}")
        }));
    let old = git(&fork, &["rev-parse", &format!("{against}^{{commit}}")]);
    // The new side: the revision --new names, else the working tree as it
    // stands (a check of a branch names it, so the tree's checkout at the
    // time cannot put another change on the scale: November 5, 2026, a
    // check meant for one branch measured the one checked out).
    let (new, new_name) = match &new_rev {
        Some(rev) => (
            git(&fork, &["rev-parse", &format!("{rev}^{{commit}}")]),
            rev.clone(),
        ),
        None => (working_tree_commit(&fork), "working tree".to_string()),
    };
    eprintln!(
        "check: BLAKE3-Servil's {new_name} ({}) against {} ({})",
        &new[..12],
        against,
        &old[..12]
    );
    let library_changed = change_summary(&fork, &old, &new);
    if unchanged_inputs(library_changed, old_bench_rev.as_deref()) {
        println!("check: the two sides' trees are the same; nothing to measure");
        return 0;
    }
    if !library_changed {
        eprintln!("check: the library is unchanged; comparing benchmark versions (--old-bench)");
    }
    if !all && named.is_none() && probe.is_none() {
        println!(
            "check: name the cells this change can move, from the summary above: --points POINTS for the gate, --probe POINTS for three pairs and no verdict, or --cells all for every point. No build or measurement ran."
        );
        return 2;
    }
    // The named scope, read and checked before a build: a name this
    // bench has no point for stops here, in seconds.
    let named_points: Option<Vec<String>> = if all {
        None
    } else {
        let points: Vec<String> = scope_points(
            probe
                .as_ref()
                .or(named.as_ref())
                .expect("a scope, checked above"),
        );
        assert!(
            !points.is_empty(),
            "a scope names a point at least\n{usage}"
        );
        for point in &points {
            assert!(
                super::point_if_named(point).is_some(),
                "--points: no point {point:?}; as --points names them, \"idle 2 KiB\" is the 2 KiB call after a pause\n{usage}"
            );
        }
        eprintln!(
            "check: measuring the {} named points ({} of {} points; the rest unmeasured in this check): {}",
            points.len(),
            points.len(),
            super::POINTS.len(),
            points.join(", ")
        );
        Some(points)
    };
    let cache = fork.join("target").join("check");
    std::fs::create_dir_all(&cache).expect("the check's cache directory");
    // The old side's bench: this one, or this repository at --old-bench.
    let old_bench = match &old_bench_rev {
        None => bench.clone(),
        Some(rev) => {
            let commit = git(&bench, &["rev-parse", &format!("{rev}^{{commit}}")]);
            // A bench from before the bench and BLAKE3-Servil became separate
            // checkouts (0.18.3 and older) builds only inside BLAKE3-Servil:
            // its results are read with its own tools.
            assert!(
                git(&bench, &["show", &format!("{commit}:build.rs")]).contains("BENCH_HASHES_FORK"),
                "--old-bench {rev}: a bench that builds beside BLAKE3-Servil (BENCH_HASHES_FORK, from candidate/bench-0.18 1f4dfd63 on); \
                 0.18.3 and older build only inside BLAKE3-Servil's checkout"
            );
            // Made afresh each check (a second's checkout): a worktree left
            // by another bench checkout may point at a repository gone (a
            // check from before the separate checkouts left one inside the
            // fork's target/, whose .git named BLAKE3-Servil's bench-hashes/:
            // the Mac's job 1831).
            let dir = cache.join("old-bench");
            if dir.exists() {
                std::fs::remove_dir_all(&dir)
                    .unwrap_or_else(|e| panic!("removing {}: {e}", dir.display()));
            }
            git(&bench, &["worktree", "prune"]);
            git(
                &bench,
                &[
                    "worktree",
                    "add",
                    "--detach",
                    dir.to_str().unwrap(),
                    &commit,
                ],
            );
            eprintln!(
                "check: the old side with the bench at {rev} ({})",
                &commit[..12]
            );
            dir
        }
    };
    // The first verdict (bench-hashes #125): both sides built from one path,
    // compared; the same code is the change's verdict, and nothing runs. One
    // bench for both sides only: with --old-bench the benches differ.
    if old_bench_rev.is_none() {
        match same_path_verdict(&fork, &bench, &cache, &old, &new) {
            Some(crate::samecode::Verdict::Same) => {
                println!("check: same code: the change alters nothing the bench runs here");
                return 0;
            }
            Some(crate::samecode::Verdict::Differs {
                sections,
                resized,
                changed,
            }) => {
                let list = |v: &[String]| {
                    let mut s = v.iter().take(12).cloned().collect::<Vec<_>>().join("; ");
                    if v.len() > 12 {
                        s += &format!("; and {} more", v.len() - 12);
                    }
                    s
                };
                eprintln!(
                    "check: the change alters the bench's code here: {}; {} functions of another size or on one side alone ({}); {} of other bytes ({})",
                    sections.join(", "),
                    resized.len(),
                    list(&resized),
                    changed.len(),
                    list(&changed)
                )
            }
            None => eprintln!("check: no same-code comparison on this machine's executable format (ELF only); measuring"),
        }
    }
    let old_exe = side(&fork, &old_bench, &cache, "old", &old);
    let new_exe = side(&fork, &bench, &cache, "new", &new);
    // The scope: every point, or the points named before the builds.
    // Every contender that calls the changed library participates; across
    // two benches, every one the older bench knows too.
    let subjects: Vec<&str> = if old_bench_rev.is_some() {
        let out = Command::new(&old_exe)
            .arg("--list")
            .output()
            .unwrap_or_else(|e| panic!("{} --list: {e}", old_exe.display()));
        assert!(out.status.success(), "{} --list failed", old_exe.display());
        let shared = shared_subjects(&String::from_utf8_lossy(&out.stdout));
        for left_out in SUBJECTS.iter().filter(|s| !shared.contains(s)) {
            eprintln!("check: --old-bench: the old bench has no contender {left_out}; its cells are left out");
        }
        assert!(
            !shared.is_empty(),
            "--old-bench: the old bench knows none of BLAKE3-Servil's contenders"
        );
        shared
    } else {
        SUBJECTS.to_vec()
    };
    let contenders: Vec<String> = subjects.iter().map(|s| s.to_string()).collect();
    let points: Option<Vec<String>> = if all { None } else { named_points };
    let olds = each_run(
        layouts(&old_bench, &cache, "old", &old_exe, 0..layouts_a_side),
        FIRST_RUNS,
    );
    let news = each_run(
        layouts(
            &bench,
            &cache,
            "new",
            &new_exe,
            layouts_a_side..2 * layouts_a_side,
        ),
        FIRST_RUNS,
    );
    // Each pair's runs on one CPU (regress's default, bench-hashes #69),
    // where both sides are this bench's; an older bench at --old-bench may
    // predate pinning, so its comparison leaves both sides to the OS.
    let pinning: Vec<String> = if old_bench_rev.is_some() {
        eprintln!("check: --old-bench: the sides' benches differ, and the older may predate pinning; regress --unpinned");
        vec!["--unpinned".into()]
    } else {
        Vec::new()
    };
    let mut regress: Vec<String> = vec![
        "regress".into(),
        "--pairs".into(),
        (layouts_a_side * FIRST_RUNS).to_string(),
        "--contenders".into(),
        contenders.join(","),
    ];
    if let Some(points) = &points {
        regress.extend(["--points".to_string(), points.join(",")]);
    }
    // A probe keeps its runs (in the cache unless --keep says where), to
    // read each cell's ratio from them afterwards.
    let keep = keep.or_else(|| {
        probe.as_ref().map(|_| {
            // Fresh each probe: a run kept from the last probe, of other
            // commits, would be read as one of this one's pairs.
            let dir = cache.join("probe");
            if dir.exists() {
                std::fs::remove_dir_all(&dir)
                    .unwrap_or_else(|e| panic!("clearing {}: {e}", dir.display()));
            }
            dir
        })
    });
    if let Some(keep) = &keep {
        regress.extend(["--keep".to_string(), keep.display().to_string()]);
    }
    regress.extend(pinning.iter().cloned());
    regress.extend(olds);
    regress.push("--".into());
    regress.extend(news);
    let (code, out) = run_regress(&new_exe, &bench, &regress);
    if probe.is_some() && code >= 2 {
        return code;
    }
    if probe.is_some() {
        // The numbers, no verdict: each cell's old and new means over the
        // pairs and their ratio (compare), for the question "is this idea
        // worth building"; the verdict is the full check's, on the change
        // once built.
        let keep = keep.as_ref().unwrap();
        let mut runs: Vec<PathBuf> = std::fs::read_dir(keep)
            .expect("the probe's kept runs")
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("run-"))
            })
            .collect();
        runs.sort();
        let (olds, news): (Vec<_>, Vec<_>) = runs
            .iter()
            .partition(|p| p.to_string_lossy().ends_with("-old.tsv"));
        let mut compare: Vec<String> = vec!["compare".into()];
        compare.extend(olds.iter().map(|p| p.display().to_string()));
        compare.push("--".into());
        compare.extend(news.iter().map(|p| p.display().to_string()));
        println!("check: the probe's cells over {} pairs (compare; no verdict: the full check's is the gate)", olds.len());
        run_regress(&new_exe, &bench, &compare);
        return 0;
    }
    let suspects: Vec<String> = out
        .lines()
        .find_map(|l| l.strip_prefix("regress: suspects: "))
        .map(|s| s.split(';').map(str::to_string).collect())
        .unwrap_or_default();
    if code != 0 || suspects.is_empty() {
        return code;
    }
    // The second stage: the suspects on fresh layouts (seeds past both
    // sides' first ones), judged among themselves.
    eprintln!(
        "check: a second stage for {} suspects on {SECOND_PAIRS} fresh pairs: {}",
        suspects.len(),
        suspects.join(", ")
    );
    let field = |k: &String, i: usize| k.split('|').nth(i).expect("a cell key").to_string();
    let contenders: BTreeSet<String> = suspects.iter().map(|k| field(k, 0)).collect();
    let points: BTreeSet<String> = suspects
        .iter()
        .map(|k| {
            point_label(&field(k, 2), &field(k, 3)).expect("a suspect's use case is this bench's")
        })
        .collect();
    let first = 2 * layouts_a_side;
    let olds = layouts(
        &old_bench,
        &cache,
        "old",
        &old_exe,
        first..first + SECOND_LAYOUTS,
    );
    let news = layouts(
        &bench,
        &cache,
        "new",
        &new_exe,
        first + SECOND_LAYOUTS..first + 2 * SECOND_LAYOUTS,
    );
    let (olds, news) = (each_run(olds, SECOND_RUNS), each_run(news, SECOND_RUNS));
    let mut second: Vec<String> = vec![
        "regress".into(),
        "--pairs".into(),
        SECOND_PAIRS.to_string(),
        "--contenders".into(),
        contenders.into_iter().collect::<Vec<_>>().join(","),
        "--points".into(),
        points.into_iter().collect::<Vec<_>>().join(","),
        "--judge".into(),
        suspects.join(";"),
    ];
    if let Some(keep) = &keep {
        second.extend([
            "--keep".to_string(),
            keep.join("second").display().to_string(),
        ]);
    }
    second.extend(pinning);
    second.extend(olds);
    second.push("--".into());
    second.extend(news);
    run_regress(&new_exe, &bench, &second).0
}

/// A scope's points, as its caller names them: comma-separated, each
/// trimmed, empties dropped.
fn scope_points(scope: &String) -> Vec<String> {
    scope
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// A short summary of the change for the reader who names its cells:
/// each file the diff touches with its added and deleted lines, and the
/// items its hunks sit in (the context `git diff` prints after each hunk
/// header). True where the two trees differ at all. This summary informs
/// the choice of cells; it chooses none.
fn change_summary(fork: &Path, old: &str, new: &str) -> bool {
    let numstat = git(fork, &["diff", "--numstat", &format!("{old}..{new}")]);
    if numstat.trim().is_empty() {
        return false;
    }
    let mut files: Vec<String> = Vec::new();
    let (mut added, mut deleted) = (0usize, 0usize);
    for line in numstat.lines() {
        let mut fields = line.split('\t');
        let (a, d, file) = (
            fields.next().unwrap_or("0"),
            fields.next().unwrap_or("0"),
            fields.next().unwrap_or(""),
        );
        added += a.parse().unwrap_or(0);
        deleted += d.parse().unwrap_or(0);
        files.push(format!("{file} (+{a}/-{d})"));
    }
    eprintln!(
        "check: the change touches {added} added and {deleted} deleted lines in {} files: {}",
        files.len(),
        first_of(&files, 12)
    );
    let diff = git(fork, &["diff", "--unified=0", &format!("{old}..{new}")]);
    let mut items: Vec<String> = Vec::new();
    for line in diff.lines().filter(|l| l.starts_with("@@")) {
        if let Some(context) = line.split("@@").nth(2).map(str::trim) {
            if !context.is_empty() && !items.iter().any(|i| i == context) {
                items.push(context.to_string());
            }
        }
    }
    if !items.is_empty() {
        eprintln!("check: its hunks sit in: {}", first_of(&items, 12));
    }
    true
}

/// The first `n` of a list, joined, with how many more follow.
fn first_of(items: &[String], n: usize) -> String {
    let mut out = items[..items.len().min(n)].join(", ");
    if items.len() > n {
        out += &format!(", and {} more", items.len() - n);
    }
    out
}

/// The `--points` label of a cell's point, from the use case's name and the
/// point as a cell key spells them ("IdleOneMessage", "256 B": "idle 256
/// B"); None for a use case of another bench version.
fn point_label(use_case: &str, point: &str) -> Option<String> {
    super::UseCase::ALL
        .into_iter()
        .find(|u| format!("{u:?}") == use_case)
        .map(|u| format!("{}{point}", u.label_prefix()))
}

/// `regress` with these arguments, its output passed through and kept.
fn run_regress(exe: &Path, bench: &Path, arguments: &[String]) -> (i32, String) {
    let out = Command::new(exe)
        .args(arguments)
        .current_dir(bench)
        .stderr(std::process::Stdio::inherit())
        .output()
        .expect("regress runs");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    print!("{text}");
    (out.status.code().unwrap_or(2), text)
}

/// BLAKE3-Servil's checkout that --fork names: a git repository holding
/// `Cargo.toml` with `name = "blake3-servil"`, wherever it sits.
fn fork_checkout(dir: &Path) -> PathBuf {
    let fork = std::fs::canonicalize(dir)
        .unwrap_or_else(|e| panic!("check: --fork {}: {e}", dir.display()));
    let manifest = std::fs::read_to_string(servil_crate(&fork).join("Cargo.toml")).unwrap_or_else(|e| {
        panic!(
            "check: {} holds no Cargo.toml: {e}; --fork names BLAKE3-Servil's checkout",
            fork.display()
        )
    });
    assert!(
        manifest.contains("name = \"blake3-servil\""),
        "check: {} is not BLAKE3-Servil",
        fork.display()
    );
    fork
}

/// The crate the bench depends on, in a checkout of BLAKE3-Servil: its
/// `threads/` where the checkout holds two crates (BLAKE3 #335: the
/// one-thread core at the root, the crate with the pool beside it, which
/// re-exports the core), else the checkout itself.
fn servil_crate(checkout: &Path) -> PathBuf {
    let threads = checkout.join("threads");
    if threads.join("Cargo.toml").exists() {
        threads
    } else {
        checkout.to_owned()
    }
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {} in {} failed: {}",
        args.join(" "),
        cwd.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .trim()
        .to_string()
}

/// A commit of the working tree as `git add -A` would stage it, HEAD its
/// parent, through a copy of the index and with fixed dates, so one tree
/// always gives one commit and nothing of the user's changes; HEAD itself
/// where the tree is HEAD's, so a clean checkout names its own commit (an
/// A/A's two sides then read as one build).
fn working_tree_commit(fork: &Path) -> String {
    let real = git(fork, &["rev-parse", "--git-path", "index"]);
    let real = if Path::new(&real).is_absolute() {
        PathBuf::from(real)
    } else {
        fork.join(real)
    };
    let copy = fork.join("target").join("check-index");
    std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
    std::fs::copy(&real, &copy).expect("the index copies");
    // With the original's modification time: git trusts an entry whose
    // file's size and time match, unless the file changed in the second
    // the index was written, which it tells from the index file's own
    // time. A copy made later hid an edit of the same size made in that
    // second (a_clean_working_tree_is_its_head, one run in three under
    // load: one tree gave two commits).
    let written = std::fs::metadata(&real)
        .and_then(|m| m.modified())
        .expect("the index has a time");
    std::fs::File::options()
        .write(true)
        .open(&copy)
        .and_then(|f| f.set_modified(written))
        .expect("the copy takes the index's time");
    let with = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(fork)
            .env("GIT_INDEX_FILE", &copy)
            .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z")
            // An identity of its own, as its date: git otherwise takes the
            // account's, or guesses one from the host's name and refuses a
            // guess without a domain (the Mac's runner on another network,
            // job 1830: "benchrunner@Mac.(none)").
            .env("GIT_AUTHOR_NAME", "bench-hashes check")
            .env("GIT_AUTHOR_EMAIL", "check@bench-hashes.invalid")
            .env("GIT_COMMITTER_NAME", "bench-hashes check")
            .env("GIT_COMMITTER_EMAIL", "check@bench-hashes.invalid")
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    with(&["add", "-A"]);
    let tree = with(&["write-tree"]);
    if tree == git(fork, &["rev-parse", "HEAD^{tree}"]) {
        return git(fork, &["rev-parse", "HEAD"]);
    }
    with(&[
        "commit-tree",
        &tree,
        "-p",
        "HEAD",
        "-m",
        "check: the working tree",
    ])
}

/// One side's bench-hashes executable, built against BLAKE3-Servil at `commit`:
/// a worktree of BLAKE3-Servil under the cache (moved only when the commit
/// changes), this bench's sources copied beside it, Cargo pointed at the
/// worktree for BLAKE3-Servil, keeping this bench's clocks, and a target
/// directory of the side's own, so an unchanged side rebuilds nothing.
fn side(fork: &Path, bench: &Path, cache: &Path, name: &str, commit: &str) -> PathBuf {
    let checkout = cache.join(name).join("src");
    if !checkout.exists() {
        git(fork, &["worktree", "prune"]);
        std::fs::create_dir_all(checkout.parent().unwrap()).unwrap();
        git(
            fork,
            &[
                "worktree",
                "add",
                "--detach",
                checkout.to_str().unwrap(),
                commit,
            ],
        );
    } else if git(&checkout, &["rev-parse", "HEAD"]) != commit {
        git(
            &checkout,
            &["checkout", "--quiet", "--force", "--detach", commit],
        );
    }
    // The side is its commit: no file a build left behind (git's ignored
    // files, the build's own, stay).
    git(&checkout, &["clean", "-fdq"]);
    build_against(&checkout, bench, &cache.join(name).join("target"), name)
}

/// Both sides built in turn from one worktree, `target/check/same/src`, into
/// one target directory, and compared (crate::samecode, with the commits
/// and the panic locations' lines masked): at two paths every build
/// differs, at one only what the change alters (bench-hashes #125). None
/// where the executable is not ELF.
fn same_path_verdict(
    fork: &Path,
    bench: &Path,
    cache: &Path,
    old: &str,
    new: &str,
) -> Option<crate::samecode::Verdict> {
    let mut exes = Vec::new();
    for (name, commit) in [("old", old), ("new", new)] {
        let exe = side(fork, bench, cache, "same", commit);
        let kept = cache.join("same").join(format!("bench-hashes-{name}"));
        std::fs::copy(&exe, &kept).unwrap_or_else(|e| panic!("{}: {e}", kept.display()));
        exes.push(kept);
    }
    let masks = [old, new, &old[..12], &new[..12]];
    crate::samecode::compare(&exes[0], &exes[1], &masks, true)
}

/// The bench `check` and `build-against` copy and build: the current
/// directory, which must be a bench-hashes checkout (they stop, saying so,
/// anywhere else).
fn bench_checkout(command: &str, usage: &str) -> PathBuf {
    let dir = std::env::current_dir().expect("the current directory");
    if let Err(why) = is_bench_checkout(&dir) {
        panic!("{command}: run it from the bench-hashes checkout it should build: {why}\n{usage}");
    }
    dir
}

/// Whether `dir` is a bench-hashes checkout: its Cargo.toml names the
/// package and it has the sources.
fn is_bench_checkout(dir: &Path) -> Result<(), String> {
    let toml = std::fs::read_to_string(dir.join("Cargo.toml"))
        .map_err(|_| format!("{} has no Cargo.toml", dir.display()))?;
    if toml.lines().any(|l| l.trim() == "name = \"bench-hashes\"")
        && dir.join("src/main.rs").is_file()
    {
        Ok(())
    } else {
        Err(format!("{} is not a bench-hashes checkout", dir.display()))
    }
}

/// `bench-hashes build-against DIR`: this bench built against BLAKE3-Servil
/// checkout at DIR as it is, into `bench-hashes-build/` of BLAKE3-Servil's target directory; the
/// executable's path on stdout (the Mac runner's benchmark jobs, and a
/// maintainer measuring the working tree, run it from there). The two
/// checkouts sit anywhere; neither holds the other.
pub fn build_against_command(arguments: &[String]) -> i32 {
    let usage = "usage: bench-hashes build-against DIR    (DIR: a checkout of BLAKE3-Servil)";
    let [dir] = arguments else { panic!("{usage}") };
    let checkout = std::fs::canonicalize(dir).unwrap_or_else(|e| panic!("{dir}: {e}\n{usage}"));
    assert!(
        checkout.join("Cargo.toml").exists() && checkout.join("c").is_dir(),
        "{dir} is no checkout of BLAKE3-Servil\n{usage}"
    );
    let bench = bench_checkout("build-against", usage);
    // Where Cargo would build (CARGO_TARGET_DIR where set: the VM builds on
    // a tmpfs, since its shared mount runs no executable), else the
    // checkout's target/.
    let shared = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
    let target = shared.clone().unwrap_or_else(|| checkout.join("target"));
    // A directory no bench before the separation (0.18.3 and earlier) builds
    // in: those built in target/build-against, from a copy at
    // checkout/bench-hashes, under the same package name and version, so
    // Cargo gave both layouts' build scripts one unit. Its record of a file
    // inside the target directory (the copy here) is relative to the
    // target, so an old bench built after a new one checked the new copy's
    // build.rs, found it fresh, and ran the new build script (the Mac's job
    // 1781, October 31).
    let exe = build_against(
        &checkout,
        &bench,
        &build_against_dir(&target, &checkout, shared.is_some()),
        "build-against",
    );
    println!("{}", named_for_provenance(&exe).display());
    0
}

/// `exe`, copied into a directory beside it named for its provenance
/// (`bench-hashes provenance`: the bench's commit and BLAKE3-Servil's, each with
/// its tree's state): a path kept from an earlier build still runs what it
/// names, and whoever holds the path can tell what it measures.
fn named_for_provenance(exe: &Path) -> PathBuf {
    let out = Command::new(exe)
        .arg("provenance")
        .output()
        .expect("the build runs");
    assert!(out.status.success(), "{} provenance failed", exe.display());
    let line = String::from_utf8(out.stdout).expect("UTF-8");
    let name = line.trim_end().replace("; ", "_").replace(' ', "-");
    let dir = exe.parent().expect("a directory").join(&name);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let named = dir.join("bench-hashes");
    std::fs::copy(exe, &named).unwrap_or_else(|e| panic!("{}: {e}", named.display()));
    eprintln!("build-against: {}", line.trim_end());
    named
}

/// Where `build-against` builds for `checkout`, inside `target`: its
/// bench-hashes-build, and, where the target directory is shared by every
/// checkout (CARGO_TARGET_DIR), a directory of the checkout's own in it.
/// Shared, three checkouts' builds once wrote one executable, so each
/// comparison of them ran one build against itself (BLAKE3 #196, the VM,
/// November 2).
fn build_against_dir(target: &Path, checkout: &Path, shared: bool) -> PathBuf {
    let dir = target.join("bench-hashes-build");
    if !shared {
        return dir;
    }
    let path = checkout.as_os_str().as_encoded_bytes();
    dir.join(&blake3::hash(path).to_hex()[..16])
}

/// The copy of the bench a build against BLAKE3-Servil compiles, in the build's
/// target directory: outside both checkouts, so neither reads as changed.
fn bench_copy(target: &Path) -> PathBuf {
    target.join("bench-src")
}

/// This bench, built against the BLAKE3-Servil checkout at `checkout` as it is:
/// its sources copied into the build's target directory (bench_copy),
/// Cargo pointed at the checkout for BLAKE3-Servil (and build.rs told where it
/// is, BENCH_HASHES_FORK), keeping this bench's clocks, a lock and a
/// target directory of the build's own (the committed lock is never
/// written); the executable's path. `check`'s two sides and the Mac
/// runner's benchmark jobs build this way, and so does a maintainer
/// measuring a working tree (`bench-hashes build-against DIR`).
pub fn build_against(checkout: &Path, bench: &Path, target: &Path, name: &str) -> PathBuf {
    let commit = git(checkout, &["rev-parse", "HEAD"]);
    let copy = bench_copy(target);
    copy_bench(bench, &copy);
    let target = target.to_path_buf();
    let patch_fork = fork_patch(&copy, &checkout);
    // The side's lock: the bench's, with BLAKE3-Servil taken from the worktree.
    // Cargo patches at the locked version only, so a version change wants
    // `cargo update -p blake3-servil`.
    let out = Command::new("cargo")
        .args([
            "--config",
            &patch_fork,
            "update",
            "--quiet",
            "-p",
            "blake3-servil",
        ])
        .current_dir(&copy)
        .env("CARGO_TARGET_DIR", &target)
        .env("BENCH_HASHES_CHECKOUT", bench)
        .env("BENCH_HASHES_FORK", checkout)
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "check: {name}: cargo update: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new("cargo")
        .args(["--config", &patch_fork, "build", "--release", "--quiet"])
        .current_dir(&copy)
        .env("CARGO_TARGET_DIR", &target)
        .env("BENCH_HASHES_CHECKOUT", bench)
        .env("BENCH_HASHES_FORK", checkout)
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "check: {name}: building bench-hashes against BLAKE3-Servil at {}:\n{}",
        &commit[..12],
        String::from_utf8_lossy(&out.stderr)
    );
    // BLAKE3-Servil must come from the side's checkout, never the commit the
    // bench's lock pins: Cargo names each package's source as it builds.
    let lock = std::fs::read_to_string(copy.join("Cargo.lock")).expect("the side's lock");
    let entry = lock
        .split("[[package]]")
        .find(|p| p.contains("name = \"blake3-servil\""))
        .expect("the lock names blake3-servil");
    assert!(
        !entry.contains("source = "),
        "check: {name}: the bench built blake3-servil from its pinned source, not the checkout {}:\n{entry}",
        checkout.display()
    );
    let exe = target.join("release").join("bench-hashes");
    require_sme2_kernel(&exe);
    exe
}

/// On a CPU with SME2, the build must carry the SME2 kernel (BLAKE3-Servil
/// builds without it, with a warning, when the C compiler cannot assemble
/// SME2): a check would measure NEON alone and miss every SME2 change. The
/// kernel's own symbol in the executable says whether it is there.
fn require_sme2_kernel(exe: &Path) {
    let has_sme2 = if cfg!(target_os = "macos") {
        Command::new("sysctl")
            .args(["-n", "hw.optional.arm.FEAT_SME2"])
            .output()
            .is_ok_and(|o| o.stdout.trim_ascii() == b"1")
    } else {
        std::fs::read_to_string("/proc/cpuinfo").is_ok_and(|c| c.contains(" sme2"))
    };
    if !has_sme2 {
        return;
    }
    let bytes = std::fs::read(exe).expect("the executable reads");
    // The kernel's name without its crate prefix: `blake3_servil_` since BLAKE3 #164, `blake3_` before.
    let symbol = b"sme2_hash16_chunks_512";
    assert!(
        bytes.windows(symbol.len()).any(|w| w == symbol),
        "check: this CPU has SME2, and BLAKE3-Servil was built without its SME2 kernel: point CC at a compiler that assembles SME2 (CC=clang-19 on Debian 12)"
    );
}

/// This bench's sources, copied file by file where they differ (so Cargo
/// sees only real changes); `target`, results and the lock stay the copy's.
fn copy_bench(from: &Path, to: &Path) {
    let skip = |p: &Path| {
        p.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some("target" | "benchmark-results" | ".git" | "tmp")
            )
        })
    };
    let mut wanted = BTreeSet::new();
    for entry in walk(from) {
        let relative = entry.strip_prefix(from).unwrap().to_path_buf();
        if skip(&relative) || relative == Path::new("Cargo.lock") {
            continue;
        }
        wanted.insert(relative.clone());
        let dest = to.join(&relative);
        let bytes = std::fs::read(&entry).unwrap();
        if std::fs::read(&dest).ok().as_deref() != Some(&bytes[..]) {
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::write(&dest, bytes).unwrap();
        }
    }
    if !to.join("Cargo.lock").exists() {
        std::fs::copy(from.join("Cargo.lock"), to.join("Cargo.lock")).unwrap();
    }
    for entry in walk(to) {
        let relative = entry.strip_prefix(to).unwrap().to_path_buf();
        if !skip(&relative) && relative != Path::new("Cargo.lock") && !wanted.contains(&relative) {
            std::fs::remove_file(&entry).unwrap();
        }
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| {
                    matches!(
                        n.to_str(),
                        Some(".git" | "target" | "tmp" | "benchmark-results")
                    )
                }) {
                    continue;
                }
                files.extend(walk(&path));
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// `exe` relinked once per seed, its functions in a fresh random order
/// each (a linker order file: lld on Linux, ld's -order_file on macOS).
/// The bench crate alone relinks; its dependencies stay built.
fn layouts(
    bench: &Path,
    cache: &Path,
    name: &str,
    exe: &Path,
    seeds: std::ops::Range<usize>,
) -> Vec<String> {
    let nm = Command::new("nm").arg(exe).output().expect("nm runs");
    let text = String::from_utf8_lossy(&nm.stdout).into_owned();
    let mut symbols: Vec<&str> = text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() == 3 && matches!(f[1], "t" | "T")).then(|| f[2])
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let side_dir = cache.join(name);
    let copy = bench_copy(&side_dir.join("target"));
    let checkout = side_dir.join("src");
    let patch_fork = fork_patch(&copy, &checkout);
    let mut exes = Vec::new();
    for seed in seeds {
        shuffle(&mut symbols, seed as u64);
        let order = side_dir.join(format!("order-{seed}.txt"));
        std::fs::write(&order, symbols.join("\n") + "\n").unwrap();
        let flags: Vec<String> = if cfg!(target_os = "macos") {
            vec![format!("-Clink-arg=-Wl,-order_file,{}", order.display())]
        } else {
            let lld = lld_dir(cache);
            vec![
                "-Clink-arg=-fuse-ld=lld".into(),
                format!("-Clink-arg=-B{}", lld.display()),
                format!("-Clink-arg=-Wl,--symbol-ordering-file={}", order.display()),
                "-Clink-arg=-Wl,--no-warn-symbol-ordering".into(),
            ]
        };
        let out = Command::new("cargo")
            .args([
                "--config",
                &patch_fork,
                "rustc",
                "--release",
                "--quiet",
                "--bin",
                "bench-hashes",
                "--",
            ])
            .args(&flags)
            .current_dir(&copy)
            .env("CARGO_TARGET_DIR", side_dir.join("target"))
            .env("BENCH_HASHES_CHECKOUT", bench)
            .env("BENCH_HASHES_FORK", &checkout)
            .output()
            .expect("cargo runs");
        assert!(
            out.status.success(),
            "check: relinking {name} with seed {seed}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let kept = side_dir.join(format!("bench-hashes-{seed}"));
        std::fs::copy(
            side_dir.join("target").join("release").join("bench-hashes"),
            &kept,
        )
        .unwrap();
        exes.push(kept.display().to_string());
    }
    exes
}

/// A directory holding `ld.lld`, for the C compiler's -B (Linux): the one
/// on PATH, else the newest versioned one.
fn lld_dir(cache: &Path) -> PathBuf {
    let link = cache.join("lld").join("ld.lld");
    if link.exists() {
        return link.parent().unwrap().to_path_buf();
    }
    let found = std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join("ld.lld"))
                .find(|p| p.exists())
        })
        .or_else(|| {
            let mut v: Vec<PathBuf> = std::fs::read_dir("/usr/bin")
                .ok()?
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("ld.lld-"))
                })
                .collect();
            v.sort();
            v.pop()
        })
        .expect("check: relinking needs ld.lld (apt install lld)");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    // The relinking runs on Linux (ld.lld) and macOS (its linker's order
    // file); elsewhere check stops here.
    #[cfg(unix)]
    std::os::unix::fs::symlink(found, &link).unwrap();
    #[cfg(not(unix))]
    panic!(
        "check relinks each side with a function order, on Linux and macOS only; {} is the ld.lld found",
        found.display()
    );
    #[cfg(unix)]
    link.parent().unwrap().to_path_buf()
}

/// Fisher-Yates with a small deterministic generator: the same seed gives
/// the same order on every machine.
fn shuffle(items: &mut [&str], seed: u64) {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in (1..items.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

/// Cargo's `--config` patch for a fork checkout. Each side's bench keeps
/// its own clocks, so their timing and summary rules are its.
fn fork_patch(copy: &Path, checkout: &Path) -> String {
    let manifest =
        std::fs::read_to_string(copy.join("Cargo.toml")).expect("the bench's Cargo.toml");
    let section = manifest
        .split("[dependencies.blake3-servil]")
        .nth(1)
        .expect("Cargo.toml names blake3-servil's dependency");
    let git = section
        .lines()
        .find_map(|l| l.trim().strip_prefix("git = "))
        .expect("blake3-servil comes by git")
        .trim_matches('"');
    format!(
        "patch.\"{git}\".blake3-servil.path=\"{}\"",
        servil_crate(checkout).display()
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_bench_is_compared_even_when_the_library_is_identical() {
        use super::unchanged_inputs;
        assert!(unchanged_inputs(false, None));
        assert!(!unchanged_inputs(true, None));
        assert!(
            !unchanged_inputs(false, Some("previous-bench")),
            "different instruments need measurement even with the same library"
        );
        assert!(!unchanged_inputs(true, Some("previous-bench")));
    }

    /// Each checkout builds in a directory of its own where the target
    /// directory is shared, and in its own target's bench-hashes-build
    /// where it isn't.
    #[test]
    fn checkouts_sharing_a_target_directory_build_apart() {
        use super::build_against_dir;
        use std::path::Path;
        let shared = Path::new("/tmp/shared-target");
        let (a, b) = (Path::new("/w/probe"), Path::new("/w/base"));
        assert_ne!(
            build_against_dir(shared, a, true),
            build_against_dir(shared, b, true)
        );
        assert_eq!(
            build_against_dir(shared, a, true),
            build_against_dir(shared, a, true)
        );
        assert!(build_against_dir(shared, a, true).starts_with(shared.join("bench-hashes-build")));
        assert_eq!(
            build_against_dir(&a.join("target"), a, false),
            a.join("target/bench-hashes-build")
        );
    }

    /// `check` and `build-against` build the bench in the current directory:
    /// this checkout is one; an empty directory and another package aren't.
    #[test]
    fn only_a_bench_checkout_is_taken_for_the_bench() {
        use super::is_bench_checkout;
        assert_eq!(
            is_bench_checkout(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))),
            Ok(())
        );
        let dir = std::env::temp_dir().join(format!("bench-checkout-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        assert!(is_bench_checkout(&dir)
            .unwrap_err()
            .contains("has no Cargo.toml"));
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"other\"\n").unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        assert!(is_bench_checkout(&dir)
            .unwrap_err()
            .contains("is not a bench-hashes checkout"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Each link runs again in turn, so neighbouring pairs keep different
    /// links (a link run twice in a row would pair its runs' drift).
    #[test]
    fn each_run_takes_the_links_in_turn() {
        let links = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(super::each_run(links, 2), ["a", "b", "c", "a", "b", "c"]);
    }

    /// A clean checkout's working tree is its HEAD (so an A/A's sides name
    /// one commit and read as one build); a changed one is a commit of the
    /// change on top of HEAD, the same commit each time for one tree.
    #[test]
    fn a_clean_working_tree_is_its_head() {
        let repo = std::env::temp_dir().join(format!("wt-commit-test-{}", std::process::id()));
        std::fs::create_dir_all(&repo).unwrap();
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(&repo)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t.invalid")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t.invalid")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        };
        run(&["init", "-q"]);
        std::fs::write(repo.join("a"), "one").unwrap();
        // As every checkout of BLAKE3-Servil, target/ ignored (the index copy
        // lives there).
        std::fs::write(repo.join(".gitignore"), "target\n").unwrap();
        run(&["add", "a", ".gitignore"]);
        run(&["commit", "-q", "-m", "one"]);
        let head = run(&["rev-parse", "HEAD"]);
        assert_eq!(super::working_tree_commit(&repo), head);
        std::fs::write(repo.join("a"), "two").unwrap();
        let changed = super::working_tree_commit(&repo);
        assert_ne!(changed, head);
        assert_eq!(run(&["rev-parse", &format!("{changed}^")]), head);
        assert_eq!(
            super::working_tree_commit(&repo),
            changed,
            "one tree, one commit"
        );
        std::fs::remove_dir_all(&repo).unwrap();
    }

    /// A named scope reads as its points, and a long summary list says
    /// how many it left out.
    #[test]
    fn named_scopes_and_summaries_read_as_written() {
        assert_eq!(
            super::scope_points(&" 2 KiB, idle 2 KiB,,brief idle 3 KiB ".to_string()),
            ["2 KiB", "idle 2 KiB", "brief idle 3 KiB"]
        );
        assert_eq!(
            super::first_of(&["a".to_string(), "b".to_string()], 12),
            "a, b"
        );
        assert_eq!(
            super::first_of(&(0..15).map(|n| n.to_string()).collect::<Vec<_>>(), 12),
            "0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, and 3 more"
        );
    }

    /// Across two benches the comparison runs the contenders the older one
    /// lists: a bench from before the pipeline leaves it out, in the
    /// subjects' own order, and a bench that lists all three keeps all.
    #[test]
    fn an_older_bench_compares_the_contenders_it_knows() {
        let before_the_pipeline = "  blake3-official      BLAKE3 official        available\n  blake3-servil-st     BLAKE3-Servil st       available\n  sha256               SHA-256                available\n  blake3-servil-mt     BLAKE3-Servil mt       available\n";
        assert_eq!(
            super::shared_subjects(before_the_pipeline),
            ["blake3-servil-st", "blake3-servil-mt"]
        );
        let today = format!(
            "{before_the_pipeline}  blake3-servil-pipeline BLAKE3-Servil pipeline available\n"
        );
        assert_eq!(super::shared_subjects(&today), super::SUBJECTS);
        // A name inside another word is no contender of that bench.
        assert!(super::shared_subjects("  not-blake3-servil-st  x  available\n").is_empty());
    }

    #[test]
    fn regression_subjects_cover_library_contenders() {
        let expected: Vec<_> = crate::Algorithm::ALL
            .into_iter()
            .map(crate::Algorithm::key)
            .filter(|key| key.starts_with("blake3-servil-"))
            .collect();
        assert_eq!(
            super::SUBJECTS.to_vec(),
            expected,
            "all BLAKE3-Servil contenders participate in regression checks"
        );
    }

    #[test]
    fn every_cell_key_names_the_point_that_measures_it() {
        for (index, point) in super::super::POINTS.iter().enumerate() {
            let label = super::point_label(&format!("{:?}", point.use_case), point.label)
                .expect("this bench's use case");
            assert_eq!(super::super::point_named(&label), index, "{label}");
        }
    }
}
