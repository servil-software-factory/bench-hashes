//! Paired comparison (`bench-hashes compare`) and regression judging (`bench-hashes regress`).

use crate::*;

/*
 * `bench-hashes compare OLD... -- NEW...`: the runs in pairs, the i-th old
 * with the i-th new (as they ran, back to back in alternating order), and
 * every cell both measured judged at once (clocks::summary::discoveries:
 * a paired t-test per cell, the false discovery rate held to
 * COMPARE_Q_PERMILLE over all cells, the regression check's margin): each
 * side's mean (the geometric mean of its runs'), the ratio judged (the
 * geometric mean of the pairs' ratios, new over old, which is the two
 * sides' figures' ratio), the verdict, and how many pairs ran faster. At least
 * three pairs. For two builds, `regress` (through BLAKE3-Servil's
 * tools/perf_regress.py, which relinks each side with a fresh layout per
 * pair) runs and judges the pairs itself: link layout moves cold calls'
 * cells by itself (audit/variance-on-unchanged-code.md at aadeca9d).
 */
/// The false discovery rate `compare` holds its verdicts to, over all the
/// cells it judges.
pub(crate) const COMPARE_Q_PERMILLE: u64 = 50;

pub(crate) fn compare_command(arguments: &[String]) {
    let usage = "usage: bench-hashes compare [--markdown] OLD.tsv... -- NEW.tsv... (one new run for each old run, in the order they ran)";
    // --markdown: the cells as a Markdown table, for a post or a record, so
    // that no figure in one is typed by hand.
    let markdown = arguments.first().is_some_and(|a| a == "--markdown");
    let arguments = &arguments[usize::from(markdown)..];
    let split = arguments
        .iter()
        .position(|argument| argument == "--")
        .expect(usage);
    let (old, new) = (&arguments[..split], &arguments[split + 1..]);
    assert!(
        old.len() >= 3 && old.len() == new.len(),
        "{usage}; at least three pairs"
    );
    // Each cell's mean wall time per unit, and (where its samples carry
    // them) its CPU time, energy, and hardware counts per unit: all the
    // samples' totals over their units, in the same fixed point.
    let (mut old_builds, mut new_builds) = (Vec::new(), Vec::new());
    let means = |path: &String, builds: &mut Vec<String>| {
        let file = read_samples(path);
        let build = file_provenance(&file);
        if !builds.contains(&build) {
            builds.push(build);
        }
        if clocks::load::MEASURED && !file.load.starts_with("quiet") {
            println!(
                "{path}: {}: its samples are no evidence of speed",
                file.load
            );
        }
        cell_means(file)
    };
    // Every cell's means a pair, wall time and (where both sides carry them)
    // CPU time, energy, and hardware counts, in the first old run's order.
    type Pairs = Vec<(u128, u128)>;
    #[derive(Default)]
    struct CompareCell {
        wall: Pairs,
        cpu: Pairs,
        energy: Pairs,
        cycles: Pairs,
        instructions: Pairs,
        misses: Pairs,
    }
    let mut order: Vec<String> = Vec::new();
    let mut cells: crate::Map<String, CompareCell> = crate::Map::default();
    for (old_path, new_path) in old.iter().zip(new) {
        let new_means: crate::Map<String, CellSummary> = means(new_path, &mut new_builds)
            .into_iter()
            .map(|s| (s.key.clone(), s))
            .collect();
        for old_cell in means(old_path, &mut old_builds) {
            let Some(new_cell) = new_means.get(&old_cell.key) else {
                continue;
            };
            if !cells.contains_key(&old_cell.key) {
                order.push(old_cell.key.clone());
            }
            let entry = cells.entry(old_cell.key).or_default();
            entry.wall.push((old_cell.wall, new_cell.wall));
            if let (Some(o), Some(n)) = (old_cell.cpu, new_cell.cpu) {
                entry.cpu.push((o, n));
            }
            if let (Some(o), Some(n)) = (old_cell.energy, new_cell.energy) {
                entry.energy.push((o, n));
            }
            if let (Some(o), Some(n)) = (old_cell.counters.cycles, new_cell.counters.cycles) {
                entry.cycles.push((o, n));
            }
            if let (Some(o), Some(n)) = (
                old_cell.counters.instructions,
                new_cell.counters.instructions,
            ) {
                entry.instructions.push((o, n));
            }
            if let (Some(o), Some(n)) = (
                old_cell.counters.cache_misses,
                new_cell.counters.cache_misses,
            ) {
                entry.misses.push((o, n));
            }
        }
    }
    print_sides("compare", &old_builds, &new_builds);
    // Only cells every pair measured.
    order.retain(|key| cells[key].wall.len() == old.len());
    let tested: Vec<Pairs> = order.iter().map(|key| cells[key].wall.clone()).collect();
    let (judged, _) = judge_once(&tested);
    let times = |permille: u64| format!("x{}.{:03}", permille / 1000, permille % 1000);
    if markdown {
        println!("| cell | old ns/unit | new ns/unit | new/old | verdict | pairs faster | CPU time | energy | cycles | instructions | cache misses |");
        println!("|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|");
    }
    for (key, judgment) in order.into_iter().zip(judged) {
        let cell = &cells[&key];
        let wall = &cell.wall;
        let verdict = match judgment.verdict {
            clocks::summary::Verdict::Slower => "slower",
            clocks::summary::Verdict::Faster => "faster",
            clocks::summary::Verdict::Level => "level",
        };
        let faster = wall.iter().filter(|(old, new)| new < old).count();
        // CPU time, energy, and hardware counts beside wall time, by the same
        // ratio, with no verdict (the rule judges wall time), where every pair
        // has them.
        let beside = |name: &str, pairs: &Pairs| {
            if pairs.len() == wall.len() {
                format!(
                    "; {name} {}",
                    times(clocks::summary::geometric_mean_permille(pairs))
                )
            } else {
                String::new()
            }
        };
        let (a, b) = (
            geometric_mean(wall.iter().map(|p| p.0)),
            geometric_mean(wall.iter().map(|p| p.1)),
        );
        if markdown {
            let column = |pairs: &Pairs| {
                if pairs.len() == wall.len() {
                    times(clocks::summary::geometric_mean_permille(pairs))
                } else {
                    "-".to_owned()
                }
            };
            println!(
                "| {} | {} | {} | {} | {verdict} | {faster} of {} | {} | {} | {} | {} | {} |",
                key.replace('|', " / "),
                Fixed(a).format_ns(),
                Fixed(b).format_ns(),
                times(judgment.ratio_permille),
                wall.len(),
                column(&cell.cpu),
                column(&cell.energy),
                column(&cell.cycles),
                column(&cell.instructions),
                column(&cell.misses)
            );
            continue;
        }
        let extra = beside("CPU time", &cell.cpu)
            + &beside("energy", &cell.energy)
            + &beside("cycles", &cell.cycles)
            + &beside("instructions", &cell.instructions)
            + &beside("cache misses", &cell.misses);
        println!(
            "{key}: {} -> {} ns/unit  {} {verdict}, {faster} of {} pairs faster{extra}",
            Fixed(a).format_ns(),
            Fixed(b).format_ns(),
            times(judgment.ratio_permille),
            wall.len()
        );
    }
}

/// Every cell judged at once (clocks::summary::discoveries), each
/// measurement once: cells whose means agree in every pair, on both sides,
/// are one measurement (an owned cell measured as its lent twin,
/// lent_twin), judged as one and given its verdict on each row, so the
/// family the false discovery rate is controlled over counts it once
/// (bench-hashes #98, Devon Jonte). Returns each row's judgment and the
/// first row holding its measurement.
pub(crate) fn judge_once(
    tested: &[Vec<(u128, u128)>],
) -> (Vec<clocks::summary::Judgment>, Vec<usize>) {
    let mut first: std::collections::BTreeMap<&Vec<(u128, u128)>, usize> = Default::default();
    let mut unique: Vec<Vec<(u128, u128)>> = Vec::new();
    let mut of = Vec::new();
    let mut held = Vec::new();
    for (i, pairs) in tested.iter().enumerate() {
        let k = *first.entry(pairs).or_insert_with(|| {
            unique.push(pairs.clone());
            held.push(i);
            unique.len() - 1
        });
        of.push(k);
    }
    let judged = clocks::summary::discoveries(&unique, COMPARE_Q_PERMILLE, REGRESS_MARGIN_PERMILLE);
    (
        of.iter().map(|&k| judged[k]).collect(),
        of.iter().map(|&k| held[k]).collect(),
    )
}

/// The geometric mean of some positive Q64.64 values: each side's figure in
/// compare, whose ratio is the ratio judged. For display only.
pub(crate) fn geometric_mean(values: impl Iterator<Item = u128>) -> u128 {
    let logs: Vec<f64> = values.map(|v| (v as f64).ln()).collect();
    assert!(!logs.is_empty(), "a mean needs values");
    (logs.iter().sum::<f64>() / logs.len() as f64).exp().round() as u128
}

/*
 * `bench-hashes regress [--contenders A,B [--points P,Q]] OLD... -- NEW...`:
 * whether the new build is slower than the old, measured on this machine.
 * `bench-hashes check` builds the two sides and runs the cells its caller
 * names, then calls this (src/check.rs); it is also callable by hand with builds of
 * one's own. REGRESS_PAIRS pairs of runs, pair i running the i-th
 * executable of each side (round robin), back to back in alternating order
 * (A B, B A, ...), each at the regression check's visits
 * (TO_CHECK_FOR_REGRESSIONS) over every cell of the default contenders,
 * or with --contenders (and --points) what a run with them measures, every
 * cell judged. Each pair gives each cell one ratio, new mean over old; the
 * cells are judged all at once by clocks::summary::discoveries (a false
 * discovery rate of 5% over them, a 5% margin). Calibrated on the Mac (fork
 * NOTES, "The regression check, calibrated"). Exit 0: no cell slower; 1: a
 * cell slower; 2: no verdict, when a run's load was busy or unobserved
 * (clocks::load).
 */
pub(crate) const REGRESS_ROUNDS: usize = 24;
pub(crate) const REGRESS_PAIRS: usize = 8;
/// This invocation owns a new or empty record directory. Check before setup:
/// overwriting old run numbers loses evidence, and leftovers mix into probes.
pub(crate) fn prepare_keep_dir(keep: &std::path::Path) {
    fs::create_dir_all(keep)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", keep.display()));
    assert!(
        fs::read_dir(keep)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", keep.display()))
            .next()
            .is_none(),
        "--keep {} must be new or empty; existing records are preserved",
        keep.display()
    );
}

/// Preserve abandoned attempts as diagnostics. Probe comparisons read only
/// the accepted top-level pairs, while the full raw record stays available.
pub(crate) fn keep_retried_pair(old: Option<&std::path::Path>, new: Option<&std::path::Path>) {
    for path in [old, new].into_iter().flatten() {
        let dir = path.parent().unwrap().join("retried");
        fs::create_dir_all(&dir).expect("the retried pair's diagnostic directory");
        fs::rename(path, dir.join(path.file_name().unwrap()))
            .expect("keep the retried pair outside the comparison");
    }
}

/// How many times a pair runs again when one of its runs is no evidence.
pub(crate) const REGRESS_REPEATS: usize = 3;

/// Distinct invocation paths used by round-robin runs. A repeated list or
/// repeated execution adds runs, while retaining the same program paths.
pub(crate) fn executable_paths_used(exes: &[String], runs: usize) -> usize {
    exes.iter()
        .take(runs.min(exes.len()))
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

/// The margin a cell's verdict uses, in the regression check and in the
/// b3sum suite: 5% since November 18, 2026 (Zooko: marks between 3% and
/// 5% were too often false or too costly to chase), 3% before.
pub(crate) const REGRESS_MARGIN_PERMILLE: u64 = 50;
/// The most level cells a regression run names as suspects for a second
/// stage, and so the largest family that stage judges (bench-hashes #66).
pub(crate) const REGRESS_SUSPECTS: usize = 8;
/// The second stage's suspects, at most REGRESS_SUSPECTS: the level cells
/// whose instruction counts moved (`moved`, the largest move first) and
/// the level cells slower beyond the margin (`slower`, the strongest
/// evidence first), taken in turn, a moved count first. Each kind keeps
/// places of its own: when the moved counts alone took every place, as
/// they did before every wall-ranked cell, a release check's 17 moved
/// counts left no place for a cell 38% slower in every pair, and the
/// check read "no cell slower" (bench-hashes #166).
pub(crate) fn suspects_in_turn<'a>(moved: &[&'a str], slower: &[&'a str]) -> Vec<&'a str> {
    let mut suspects = Vec::new();
    for i in 0..moved.len().max(slower.len()) {
        suspects.extend(moved.get(i));
        suspects.extend(slower.get(i));
    }
    suspects.truncate(REGRESS_SUSPECTS);
    suspects
}

/// The margin beyond which a level cell's instructions, moved, name it a
/// suspect beside the wall-ranked ones (Zooko, November 4, 2026, on
/// John's rule; bench-hashes #127): instructions retired are the same
/// count whatever the core's clock or state, so a move is the code's,
/// and #119's cold code moved them 1.2% in every pair where wall missed
/// it three times. Wall still decides, on the second stage's fresh pairs.
/// The rule reads the solo cells of the single-threaded contenders
/// alone, where the calling thread does the whole work: a pool's caller
/// spins while it waits, and its count moved 1-9% between two runs of
/// the same code (data branch, 2026-11-04/judge2-i7).
pub(crate) const INSTRUCTIONS_MARGIN_PERMILLE: u64 = 5;

/// What each of the check's runs measures, and which of its cells are
/// judged: the regress points' solo cells; a default run's every cell
/// (--full); or the cells a run with the caller's --contenders (and
/// --points) measures, every one judged.
pub(crate) enum RegressScope {
    Full,
    Chosen {
        contenders: String,
        points: Option<String>,
    },
    Cells(String),
}

impl RegressScope {
    /// The run's arguments, `rounds` rounds (0: a default run, its own).
    pub(crate) fn arguments(&self, rounds: usize) -> Vec<String> {
        let count = rounds;
        let rounds = || {
            vec![
                "--for-regression-check".to_string(),
                "--rounds".to_string(),
                count.to_string(),
            ]
        };
        match self {
            RegressScope::Full => {
                let contenders: Vec<&str> = DEFAULT_CONTENDERS.iter().map(|a| a.key()).collect();
                [vec!["--contenders".into(), contenders.join(",")], rounds()].concat()
            }
            RegressScope::Chosen { contenders, points } => {
                let points = points
                    .iter()
                    .flat_map(|p| ["--points".to_string(), p.clone()])
                    .collect();
                [
                    vec!["--contenders".into(), contenders.clone()],
                    points,
                    rounds(),
                ]
                .concat()
            }
            RegressScope::Cells(cells) => {
                [vec!["--cells".into(), cells.clone()], rounds()].concat()
            }
        }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            RegressScope::Full => "every cell of a default run".to_string(),
            RegressScope::Chosen { contenders, points } => format!(
                "--contenders {contenders}{}",
                points
                    .as_ref()
                    .map_or(String::new(), |p| format!(" --points {p:?}"))
            ),
            RegressScope::Cells(cells) => format!("--cells {cells:?}"),
        }
    }
}

/// Two sizes files as one: every cell of `old`, with its size, and the cells
/// only `new` has, with theirs.
pub(crate) fn merge_sizes(old: &str, new: &str) -> String {
    let cell = |line: &str| {
        line.rsplit_once('\t')
            .map(|(cell, _)| cell.to_owned())
            .unwrap_or_default()
    };
    let old_cells: crate::Set<String> = old.lines().skip(1).map(cell).collect();
    old.lines()
        .chain(
            new.lines()
                .skip(1)
                .filter(|line| !old_cells.contains(&cell(line))),
        )
        .map(|line| format!("{line}\n"))
        .collect()
}

/// Flush stdio before leaving via `std::process::exit`: when stdout is
/// redirected to a pipe or log file (block-buffered), `process::exit` alone
/// discards buffered lines such as `regress` diagnostics.
pub(crate) fn exit_with(code: i32) -> ! {
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = std::io::Write::flush(&mut std::io::stderr());
    std::process::exit(code);
}

/// `regress`'s exit code when a side's run stopped (a panic, a crash):
/// beside 0 (no cell slower), 1 (a cell slower) and 2 (no verdict: the
/// load).
pub(crate) const REGRESS_STOPPED: i32 = 3;

/// What `regress` prints when a side's run stops: which side, its build,
/// how it ended, and its own last words (its stderr without the progress
/// display, the last twelve lines), then where the whole stderr is kept.
/// No speed verdict follows a stopped side. (The 0.21 release candidate's
/// gate of BLAKE3 #331: the old side panicked in a pipeline cell, and the
/// check said so with a panic of its own, the message under a screen of
/// progress lines.)
pub(crate) fn stopped_report(
    side: &str,
    exe: &str,
    build: Option<&str>,
    ended: &str,
    stderr: &str,
    record: &str,
) -> String {
    // A progress line starts "[ 12.3s] "; Rust's backtrace hint says
    // nothing about this run.
    let progress = |line: &str| {
        line.strip_prefix('[')
            .and_then(|rest| rest.split_once("s] "))
            .is_some_and(|(time, _)| time.trim().parse::<f64>().is_ok())
    };
    let words: Vec<&str> = stderr
        .lines()
        .filter(|line| {
            !line.trim().is_empty() && !progress(line) && !line.contains("RUST_BACKTRACE")
        })
        .collect();
    let mut text = format!(
        "regress: the {side} side STOPPED: {exe}{} ended with {ended}\n",
        build.map_or(String::new(), |build| format!(" ({build})"))
    );
    if words.is_empty() {
        text += "regress:   it wrote nothing to stderr beside its progress\n";
    }
    for line in &words[words.len().saturating_sub(12)..] {
        text += &format!("regress:   {line}\n");
    }
    text += &format!("regress: its whole stderr: {record}\n");
    text += &format!(
        "regress: STOPPED: a side's run failed. No speed verdict (exit {REGRESS_STOPPED}): the failure is the finding.\n"
    );
    text
}

/// One run of `exe` over `scope`, `rounds` rounds (0: a default run's
/// own), its samples sized by the file at `sizes`: its samples file, also
/// copied to `keep`, and the load this process saw while it ran
/// ([`regress_load`]). The run is this process's
/// child (clocks::child::run), so its CPU time is this process's own and
/// the windows around it stay a second long: a run needs no least length,
/// and its own load line (of windows inside it) goes unread.
pub(crate) fn regress_run(
    exe: &str,
    side: &str,
    rounds: usize,
    scope: &RegressScope,
    sizes: &std::path::Path,
    keep: Option<&std::path::Path>,
    pin: Option<usize>,
) -> (SamplesFile, String) {
    let directory = std::env::temp_dir().join(format!(
        "bench-hashes-regress-{}-{}",
        std::process::id(),
        clocks::load::now_ns()
    ));
    fs::create_dir_all(&directory).unwrap();
    let stderr_path = directory.join("stderr.txt");
    let stderr = fs::File::create(&stderr_path).unwrap();
    let mut command = std::process::Command::new(exe);
    match pin {
        Some(cpu) => command.env(PIN_CPU_VARIABLE, cpu.to_string()),
        None => command.env_remove(PIN_CPU_VARIABLE),
    };
    let run = clocks::child::run(
        command
            .args(scope.arguments(rounds))
            .arg("--sizes")
            .arg(sizes)
            .current_dir(&directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(stderr),
    );
    if !run.success() {
        // A side that stops is the gravest finding a comparison can make:
        // said in lines of its own, with an exit code of its own.
        let record = match keep {
            Some(kept) => {
                let record = kept.with_extension("stderr.txt");
                fs::copy(&stderr_path, &record).unwrap_or_else(|error| {
                    panic!(
                        "cannot keep the stopped run's stderr at {}: {error}",
                        record.display()
                    )
                });
                record
            }
            None => stderr_path.clone(),
        };
        let build = std::process::Command::new(exe)
            .arg("provenance")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
        print!(
            "{}",
            stopped_report(
                side,
                exe,
                build.as_deref(),
                &run.ended.to_string(),
                &fs::read_to_string(&stderr_path).unwrap_or_default(),
                &record.display().to_string(),
            )
        );
        exit_with(REGRESS_STOPPED);
    }
    let found: Vec<_> = fs::read_dir(directory.join("benchmark-results"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("bench-hashes.samples.tsv"))
        .collect();
    assert_eq!(found.len(), 1, "one samples file");
    let file = read_samples(found[0].to_str().unwrap());
    // Both sides of a pair on its CPU, or neither: a build that ignores the
    // variable (one older than pinning) would measure unpinned beside a
    // pinned one, a difference between the sides that is no change of theirs.
    if let Some(cpu) = pin {
        let text = fs::read_to_string(&found[0]).unwrap();
        let line = text
            .lines()
            .find_map(|l| l.strip_prefix("# measuring thread: "))
            .unwrap_or("not stated");
        assert!(
            line.starts_with(&format!("pinned to CPU {cpu} ")),
            "{exe}: asked to pin its measuring thread to CPU {cpu}, its samples say \"{line}\" (a bench older than pinning: run check or regress with --unpinned)"
        );
    }
    let load = regress_load(run.started_ns, run.started_ns + run.wall_ns);
    if let Some(keep) = keep {
        // The kept file says what regress judged the run by: its load line
        // the windows around the run, which compare reads in turn (the
        // windows listed stay the run's own, on its samples' clock).
        let text = fs::read_to_string(&found[0]).unwrap();
        let kept: String = text.lines().map(|line| match line.strip_prefix("# load: ") {
            Some(_) => format!("# load: {load}\n# load measured by: bench-hashes regress, around the run (the windows below are the run's own)\n"),
            None => format!("{line}\n"),
        }).collect();
        fs::write(keep, kept)
            .unwrap_or_else(|error| panic!("cannot keep {}: {error}", keep.display()));
    }
    fs::remove_dir_all(&directory).unwrap();
    (file, load)
}

/// Maximum local wait for load windows to cover a finished child run before
/// treating background load as unavailable instead of spinning indefinitely.
const REGRESS_LOAD_DEADLINE_NS: u64 = 2 * clocks::load::WINDOW_NS;

fn regress_load_with(
    start_ns: u64,
    end_ns: u64,
    measured: bool,
    deadline_ns: u64,
    mut poll: impl FnMut() -> (Vec<clocks::load::Window>, bool, u64),
    mut sleep_step: impl FnMut(),
) -> String {
    if !measured {
        return clocks::load::describe(&[]);
    }
    loop {
        let (windows, available, elapsed_ns) = poll();
        if windows.last().is_some_and(|last| last.end_ns >= end_ns) {
            let overlapping: Vec<clocks::load::Window> = windows
                .into_iter()
                .filter(|w| w.end_ns > start_ns && w.start_ns < end_ns)
                .collect();
            return clocks::load::describe(&overlapping);
        }
        if !available || elapsed_ns >= deadline_ns {
            return "unavailable: background load windows did not cover the run".to_owned();
        }
        sleep_step();
    }
}

/// The load in the windows of this process overlapping `start_ns` to
/// `end_ns` (clocks::load's line for them), once a window covers the end:
/// stops with an unavailable (non-quiet) line if the load instrument stops
/// responding or fails to cover `end_ns` within `REGRESS_LOAD_DEADLINE_NS`.
pub(crate) fn regress_load(start_ns: u64, end_ns: u64) -> String {
    let started = clocks::now();
    regress_load_with(
        start_ns,
        end_ns,
        clocks::load::MEASURED,
        REGRESS_LOAD_DEADLINE_NS,
        || {
            (
                clocks::load::windows(),
                clocks::load::available(),
                clocks::since_ns(started),
            )
        },
        || std::thread::sleep(std::time::Duration::from_millis(50)),
    )
}

/// A ratio in thousandths as the signed change it is: 1,206 is "+20.6%".
pub(crate) fn percent(permille: u64) -> String {
    let d = permille as i64 - 1000;
    format!(
        "{}{}.{}%",
        if d < 0 { "-" } else { "+" },
        d.abs() / 10,
        d.abs() % 10
    )
}

/// Hardware-counter summary appended to a judged `regress` line when every
/// pair carries non-zero counts on both sides.
pub(crate) fn counter_note(
    cycles: &[(u128, u128)],
    instructions: &[(u128, u128)],
    misses: &[(u128, u128)],
    pairs: usize,
) -> String {
    let mut parts = Vec::new();
    for (name, series) in [
        ("cycles", cycles),
        ("instructions", instructions),
        ("cache misses", misses),
    ] {
        if series.len() == pairs && pairs > 0 {
            parts.push(format!(
                "{name} {}",
                percent(clocks::summary::geometric_mean_permille(series))
            ));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    }
}

/// A judged cell's line: its verdict and the ratio judged, then how far
/// the pairs spread (each pair is its own link layout, so the spread is
/// what a layout and a run can move this cell), then every pair.
pub(crate) fn judged_line(word: &str, key: &str, ratio_permille: u64, pairs: &[u64]) -> String {
    let (least, most) = (
        pairs
            .iter()
            .copied()
            .min()
            .expect("a judged cell has pairs"),
        pairs
            .iter()
            .copied()
            .max()
            .expect("a judged cell has pairs"),
    );
    let all: Vec<String> = pairs.iter().map(|&r| percent(r)).collect();
    format!(
        "  {word} {key}: {} (the pairs' geometric mean; {} pairs, from {} to {}: {})",
        percent(ratio_permille),
        pairs.len(),
        percent(least),
        percent(most),
        all.join(" ")
    )
}

pub(crate) fn regress_command(arguments: &[String]) -> i32 {
    let usage = "usage: bench-hashes regress [--contenders A,B [--points P,Q] | --cells KEY|P;...] [--judge CELL;CELL] [--pairs N] [--rounds K] [--keep DIR] [--unpinned] OLD_EXE... -- NEW_EXE... (pair i runs the i-th of each, round robin; bench-hashes check builds the sides and runs the cells its caller names; each pair's runs take their samples after a gap on one CPU, pairs alternating core parity, unless --unpinned)";
    let mut arguments = arguments;
    let (mut contenders, mut points, mut cells, mut pairs, mut chosen_rounds, mut keep) =
        (None, None, None, REGRESS_PAIRS, None, None);
    let mut only: Option<Vec<String>> = None;
    let mut unpinned = false;
    while let Some(name) = arguments
        .first()
        .filter(|a| a.starts_with("--") && *a != "--")
    {
        if name == "--unpinned" {
            unpinned = true;
            arguments = &arguments[1..];
            continue;
        }
        assert!(arguments.len() > 1, "{name} needs a value\n{usage}");
        let value = arguments[1].clone();
        let count = || {
            value
                .parse::<usize>()
                .ok()
                .filter(|&n| n > 0)
                .unwrap_or_else(|| panic!("{name} takes a positive count\n{usage}"))
        };
        match name.as_str() {
            "--contenders" => contenders = Some(value.clone()),
            "--points" => points = Some(value.clone()),
            "--cells" => cells = Some(value.clone()),
            "--pairs" => pairs = count(),
            "--rounds" => chosen_rounds = Some(count()),
            "--keep" => keep = Some(std::path::PathBuf::from(&value)),
            "--judge" => only = Some(value.split(';').map(str::to_string).collect()),
            _ => panic!("unknown option {name}\n{usage}"),
        }
        arguments = &arguments[2..];
    }
    assert!(
        pairs >= 3,
        "--pairs: the rule needs at least three pairs\n{usage}"
    );
    if let Some(keep) = &keep {
        prepare_keep_dir(keep);
    }
    // The machine first: a run on a busy machine abstains at its end; ten
    // quiet seconds now say so before it (check asks the same before its
    // build), tried for up to two minutes, since check's builds leave load
    // behind them for a few seconds.
    if let Err(reason) = clocks::load::quiet_within(120) {
        println!(
            "regress: {reason}. No verdict (exit 2); run when nothing else runs on the machine."
        );
        return 2;
    }
    assert!(
        cells.is_none() || (contenders.is_none() && points.is_none()),
        "--cells stands in place of --contenders and --points\n{usage}"
    );
    assert!(
        points.is_none() || contenders.is_some(),
        "--points narrows a --contenders check\n{usage}"
    );
    let scope = match (cells, contenders) {
        (Some(cells), _) => RegressScope::Cells(cells),
        (None, Some(contenders)) => RegressScope::Chosen { contenders, points },
        (None, None) => RegressScope::Full,
    };
    let split = arguments.iter().position(|a| a == "--").expect(usage);
    let (old_exes, new_exes) = (&arguments[..split], &arguments[split + 1..]);
    assert!(!old_exes.is_empty() && !new_exes.is_empty(), "{usage}");
    let rounds = chosen_rounds.unwrap_or(REGRESS_ROUNDS);
    // Each pair's CPU (bench-hashes #69): its two runs take their samples
    // after a gap there, so the core is no difference between the sides.
    let allowed = allowed_cpu();
    let pins: Vec<Option<usize>> = (0..pairs)
        .map(|pair| {
            if unpinned {
                None
            } else {
                topology().measuring_cpu(pair, &allowed)
            }
        })
        .collect();
    match (unpinned, pins.iter().flatten().count()) {
        (true, _) => eprintln!("regress: --unpinned: the OS places every run's measuring thread"),
        (false, 0) => eprintln!("regress: no CPU to pin to (no thread affinity or topology here): the OS places every run's measuring thread"),
        (false, _) => {
            let mut cpus: Vec<usize> = pins.iter().flatten().copied().collect();
            cpus.sort();
            cpus.dedup();
            eprintln!("regress: each pair's runs take their samples after a gap on one CPU, the pairs alternating core parity over the cache domains: CPUs {}", cpus.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
        }
    }
    eprintln!(
        "regress: {pairs} run pairs using {} distinct new executable paths and {} old, over {}{}",
        executable_paths_used(new_exes, pairs),
        executable_paths_used(old_exes, pairs),
        scope.describe(),
        chosen_rounds.map_or(String::new(), |k| format!(", {k} rounds a run"))
    );
    let mut run_index = 0;
    // One calibration a side, by its first build, for the whole check: every
    // run takes the sizes (Sizes). A cell both sides measure takes the old
    // side's size, so both do the same work a sample; a cell only one side
    // measures (a --bench comparison whose cells changed), its own side's.
    // Each side reads a file of its own: the old side's builds its own
    // calibration (an older bench rejects a size for a cell it lacks, such
    // as one this bench added), the new side's the merged file.
    let sizes = std::env::temp_dir().join(format!(
        "bench-hashes-regress-sizes-{}.tsv",
        std::process::id()
    ));
    let old_sizes = std::env::temp_dir().join(format!(
        "bench-hashes-regress-sizes-{}-old-side.tsv",
        std::process::id()
    ));
    let calibrate = |exe: &str, side: &str| -> String {
        let path = std::env::temp_dir().join(format!(
            "bench-hashes-regress-sizes-{}-{side}.tsv",
            std::process::id()
        ));
        let calibration = std::process::Command::new(exe)
            .args(scope.arguments(rounds))
            .arg("--calibrate-only")
            .arg(&path)
            .stdout(std::process::Stdio::null())
            .output()
            .unwrap_or_else(|error| panic!("failed to run {exe}: {error}"));
        assert!(
            calibration.status.success(),
            "{exe} --calibrate-only failed\n{}",
            String::from_utf8_lossy(&calibration.stderr).trim_end()
        );
        let text = fs::read_to_string(&path).unwrap();
        fs::remove_file(&path).unwrap();
        text
    };
    let old_calibration = calibrate(&old_exes[0], "old");
    let merged = merge_sizes(&old_calibration, &calibrate(&new_exes[0], "new"));
    fs::write(&sizes, merged)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", sizes.display()));
    fs::write(&old_sizes, old_calibration)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", old_sizes.display()));
    if let Some(keep) = &keep {
        fs::copy(&sizes, keep.join("sizes.tsv"))
            .unwrap_or_else(|error| panic!("cannot keep the sizes: {error}"));
    }
    let mut unreliable = Vec::new();
    let mut powers: Vec<String> = Vec::new();
    let (mut old_builds, mut new_builds): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    let mut ratios: std::collections::BTreeMap<String, Vec<u64>> =
        std::collections::BTreeMap::new();
    let mut pairs_means: std::collections::BTreeMap<String, Vec<(u128, u128)>> =
        std::collections::BTreeMap::new();
    // A run's means, and its load line when clocks found it no evidence
    // (other programs busy, or no load window).
    let mut instruction_ratios: std::collections::BTreeMap<String, Vec<u64>> =
        std::collections::BTreeMap::new();
    let mut cycle_pairs: std::collections::BTreeMap<String, Vec<(u128, u128)>> =
        std::collections::BTreeMap::new();
    let mut instruction_pairs: std::collections::BTreeMap<String, Vec<(u128, u128)>> =
        std::collections::BTreeMap::new();
    let mut miss_pairs: std::collections::BTreeMap<String, Vec<(u128, u128)>> =
        std::collections::BTreeMap::new();
    // A run's means (wall, and hardware counts a unit where the machine
    // counts them), and its load line when clocks found it no evidence
    // (other programs busy, or no load window).
    let mut means = |exe: &str,
                     side: &str,
                     pin: Option<usize>|
     -> (
        crate::Map<String, u128>,
        crate::Map<String, CellCounters>,
        Option<String>,
        Option<std::path::PathBuf>,
    ) {
        run_index += 1;
        let kept = keep
            .as_ref()
            .map(|dir| dir.join(format!("run-{run_index:03}-{side}.tsv")));
        let side_sizes = if side == "old" { &old_sizes } else { &sizes };
        let (file, load) = regress_run(exe, side, rounds, &scope, side_sizes, kept.as_deref(), pin);
        let builds = if side == "old" {
            &mut old_builds
        } else {
            &mut new_builds
        };
        let build = file_provenance(&file);
        if !builds.contains(&build) {
            builds.push(build);
        }
        if !powers.contains(&file.power) {
            powers.push(file.power.clone());
        }
        // Where clocks measures no load (macOS: clocks::load's doc), a run's
        // line says "not measured", and the run is evidence: keep that
        // machine quiet by hand while it measures.
        let busy = (clocks::load::MEASURED && !load.starts_with("quiet")).then_some(load);
        let counters = cell_counters(&file);
        (
            file.cells
                .into_iter()
                .map(|(key, samples)| (key, Mean::of(&samples).fixed().0))
                .collect(),
            counters,
            busy,
            kept,
        )
    };
    for pair in 0..pairs {
        let began = clocks::now();
        // A pair with a run that is no evidence runs again, in the same
        // order, up to REGRESS_REPEATS times (a process launch on macOS
        // now and then costs other programs about 1.7 CPU-seconds inside
        // a run's window: fork runner jobs 1304-1305, October 5, 2026).
        let mut attempt = 0;
        let (old_exe, new_exe) = (
            &old_exes[pair % old_exes.len()],
            &new_exes[pair % new_exes.len()],
        );
        let (old, old_counters, new, new_counters) = loop {
            let pin = pins[pair];
            let ((old, oc, a, old_path), (new, nc, b, new_path)) = if pair % 2 == 0 {
                let o = means(old_exe, "old", pin);
                (o, means(new_exe, "new", pin))
            } else {
                let n = means(new_exe, "new", pin);
                (means(old_exe, "old", pin), n)
            };
            let busy: Vec<String> = a.into_iter().chain(b).collect();
            if busy.is_empty() {
                break (old, oc, new, nc);
            }
            attempt += 1;
            if attempt > REGRESS_REPEATS {
                unreliable.extend(busy);
                break (old, oc, new, nc);
            }
            keep_retried_pair(old_path.as_deref(), new_path.as_deref());
            eprintln!("regress: pair {} again: {}", pair + 1, busy.join("; "));
        };
        for (key, old_count) in &old_counters {
            let Some(new_count) = new_counters.get(key) else {
                continue;
            };
            if let (Some(o), Some(n)) = (old_count.cycles, new_count.cycles) {
                cycle_pairs.entry(key.clone()).or_default().push((o, n));
            }
            if let (Some(o), Some(n)) = (old_count.instructions, new_count.instructions) {
                instruction_ratios
                    .entry(key.clone())
                    .or_default()
                    .push(clocks::summary::ratio_permille(n, o));
                instruction_pairs
                    .entry(key.clone())
                    .or_default()
                    .push((o, n));
            }
            if let (Some(o), Some(n)) = (old_count.cache_misses, new_count.cache_misses) {
                miss_pairs.entry(key.clone()).or_default().push((o, n));
            }
        }
        for (key, old_mean) in &old {
            let Some(&new_mean) = new.get(key) else {
                continue;
            };
            ratios
                .entry(key.clone())
                .or_default()
                .push(clocks::summary::ratio_permille(new_mean, *old_mean));
            pairs_means
                .entry(key.clone())
                .or_default()
                .push((*old_mean, new_mean));
        }
        let tenths = (clocks::since_ns(began) + 50_000_000) / 100_000_000;
        eprintln!(
            "regress: pair {} of {pairs} in {}.{} s",
            pair + 1,
            tenths / 10,
            tenths % 10
        );
    }
    print_sides("regress", &old_builds, &new_builds);
    let power = format!("regress: power during the check: {}", powers.join("; "));
    if !unreliable.is_empty() {
        println!("regress: other programs kept the machine busy, or clocks saw no load window, during {} runs. No verdict (exit 2); run again when nothing else runs on the machine.", unreliable.len());
        for load in &unreliable {
            println!("  {load}");
        }
        println!("{power}");
        return 2;
    }
    // The cells judged (the points' solo cells; with --full or a chosen
    // selection, every cell every pair measured), all at once
    // (clocks::summary::discoveries).
    // With --judge, those cells alone: a second stage's family (check).
    let judged: Vec<&String> = pairs_means
        .iter()
        .filter(|(_, m)| m.len() == pairs)
        .map(|(k, _)| k)
        .filter(|k| only.as_ref().is_none_or(|only| only.contains(k)))
        .collect();
    if let Some(only) = &only {
        assert_eq!(
            judged.len(),
            only.len(),
            "--judge: every cell named is measured in every pair"
        );
    }
    let tested: Vec<Vec<(u128, u128)>> = judged.iter().map(|k| pairs_means[*k].clone()).collect();
    let (judgments, measurement) = judge_once(&tested);
    let mut slower = 0;
    // The level cells a second stage may take: those whose instructions
    // moved beyond their margin (the code's doing), by their place among
    // the moved, and those slower beyond the wall margin, by their rank.
    let mut moved_suspects: Vec<(usize, &String)> = Vec::new();
    let mut slower_suspects: Vec<(usize, &String)> = Vec::new();
    let moved: Vec<(String, u64)> = instructions_moved(&instruction_ratios, pairs)
        .into_iter()
        .filter(|(k, _)| only.as_ref().is_none_or(|only| only.contains(k)))
        .collect();
    for (i, (key, judgment)) in judged.into_iter().zip(judgments).enumerate() {
        // A row whose measurement an earlier row holds (an owned cell
        // measured as its lent twin): its verdict printed, not counted again.
        let first = measurement[i] == i;
        if judgment.verdict == clocks::summary::Verdict::Level {
            if first && moved.iter().any(|(k, _)| k == key) {
                // Its place among the moved, the largest move first.
                let place = moved.iter().position(|(k, _)| k == key).unwrap();
                moved_suspects.push((place, key));
            } else if first && judgment.ratio_permille > 1000 + REGRESS_MARGIN_PERMILLE {
                slower_suspects.push((judgment.rank, key));
            }
            continue;
        }
        let word = if judgment.verdict == clocks::summary::Verdict::Slower {
            slower += usize::from(first);
            "SLOWER"
        } else {
            "faster"
        };
        let note = counter_note(
            cycle_pairs.get(key).map(Vec::as_slice).unwrap_or(&[]),
            instruction_pairs.get(key).map(Vec::as_slice).unwrap_or(&[]),
            miss_pairs.get(key).map(Vec::as_slice).unwrap_or(&[]),
            pairs,
        );
        println!(
            "{}{note}",
            judged_line(word, key, judgment.ratio_permille, &ratios[key])
        );
    }
    if slower > 0 {
        println!("regress: REGRESSION: {slower} cells slower");
    } else {
        println!("regress: no cell slower");
        // The level cells slower beyond the margin, the strongest evidence
        // first: what a second stage confirms on fresh pairs (check). At 8
        // pairs, ranked by ratio, #119's slowed cells were among the 8 in
        // 42 resamplings of 55, ranked by evidence in 52 (the data branch,
        // 2026-10-31/check-errors-i7).
        moved_suspects.sort();
        slower_suspects.sort();
        let moved_in_order: Vec<&str> = moved_suspects.iter().map(|(_, k)| k.as_str()).collect();
        let slower_in_order: Vec<&str> = slower_suspects.iter().map(|(_, k)| k.as_str()).collect();
        let suspects = suspects_in_turn(&moved_in_order, &slower_in_order);
        if instruction_ratios.is_empty() {
            println!("regress: this machine counts no instructions; the suspects are wall's alone");
        } else if moved.is_empty() {
            println!(
                "regress: no level cell's instructions moved beyond {}",
                percent(1000 + INSTRUCTIONS_MARGIN_PERMILLE)
            );
        } else {
            let listed: Vec<String> = moved
                .iter()
                .map(|(k, r)| format!("{k} {}", percent(*r)))
                .collect();
            println!(
                "regress: instructions moved beyond {} in {} level cells, suspects first: {}",
                percent(1000 + INSTRUCTIONS_MARGIN_PERMILLE),
                moved.len(),
                listed.join("; ")
            );
        }
        if !suspects.is_empty() {
            println!("regress: suspects: {}", suspects.join(";"));
        }
    }
    let measures_servil_mt = pairs_means
        .keys()
        .any(|k| k.starts_with("blake3-servil-mt|"));
    if only.is_none()
        && measures_servil_mt
        && supports_pipeline_lifecycle(&old_exes[0])
        && supports_pipeline_lifecycle(&new_exes[0])
    {
        if !run_pipeline_lifecycle_check(old_exes, new_exes) {
            slower += 1;
            println!("regress: REGRESSION: Pipeline cold create/drop lifecycle regressed");
        }
    }
    println!("{power}");
    i32::from(slower > 0)
}

/// Minimum RSS delta (bytes) and relative growth (permille) required in every
/// pair to flag an eager-faulting / memory-lifecycle regression.
const LIFECYCLE_RSS_MIN_DELTA_BYTES: u64 = 32 * 1024 * 1024;
const LIFECYCLE_RSS_MARGIN_PERMILLE: u64 = 250;
/// Minimum wall-time delta (ns) and relative growth (permille) required in
/// every pair to flag a cold Pipeline create/drop latency regression without
/// false alarms from 1-2 ms child-process startup jitter.
const LIFECYCLE_WALL_MIN_DELTA_NS: u64 = 10_000_000;
const LIFECYCLE_WALL_MARGIN_PERMILLE: u64 = 500;
const LIFECYCLE_PAIRS: usize = 4;

fn supports_pipeline_lifecycle(exe: &str) -> bool {
    std::process::Command::new(exe)
        .args(["pipeline-lifecycle", "--check"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[derive(Debug, PartialEq, Eq)]
struct LifecycleVerdict {
    wall_slower: bool,
    rss_higher: bool,
}

fn judge_lifecycle_pairs(wall: &[(u64, u64)], rss: &[(u64, u64)]) -> LifecycleVerdict {
    let wall_slower = !wall.is_empty()
        && wall.iter().all(|&(o, n)| {
            n.saturating_sub(o) >= LIFECYCLE_WALL_MIN_DELTA_NS
                && u128::from(n) * 1000
                    >= u128::from(o) * u128::from(1000 + LIFECYCLE_WALL_MARGIN_PERMILLE)
        });
    let rss_higher = !rss.is_empty()
        && rss.iter().all(|&(o, n)| {
            n.saturating_sub(o) >= LIFECYCLE_RSS_MIN_DELTA_BYTES
                && u128::from(n) * 1000
                    >= u128::from(o) * u128::from(1000 + LIFECYCLE_RSS_MARGIN_PERMILLE)
        });
    LifecycleVerdict {
        wall_slower,
        rss_higher,
    }
}

fn median_u64(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn run_pipeline_lifecycle_once(exe: &str, side: &str) -> (u64, Option<u64>) {
    let mut command = std::process::Command::new(exe);
    let run = clocks::child::run(
        command
            .arg("pipeline-lifecycle")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped()),
    );
    assert!(
        run.success(),
        "regress: {side} {exe} pipeline-lifecycle failed ({})",
        run.ended
    );
    (run.wall_ns, run.max_rss_bytes)
}

/// Compare cold `Pipeline::new(rooms=4096)` + 1 message + `Drop` across
/// alternating fresh child processes, returning `true` when both cold wall time
/// and peak RSS stay within bounds (bench-hashes #303).
fn run_pipeline_lifecycle_check(old_exes: &[String], new_exes: &[String]) -> bool {
    let mut wall_pairs = Vec::with_capacity(LIFECYCLE_PAIRS);
    let mut rss_pairs = Vec::with_capacity(LIFECYCLE_PAIRS);
    for pair in 0..LIFECYCLE_PAIRS {
        let old_exe = &old_exes[pair % old_exes.len()];
        let new_exe = &new_exes[pair % new_exes.len()];
        let ((ow, or), (nw, nr)) = if pair % 2 == 0 {
            let o = run_pipeline_lifecycle_once(old_exe, "old");
            (o, run_pipeline_lifecycle_once(new_exe, "new"))
        } else {
            let n = run_pipeline_lifecycle_once(new_exe, "new");
            (run_pipeline_lifecycle_once(old_exe, "old"), n)
        };
        wall_pairs.push((ow, nw));
        if let (Some(o), Some(n)) = (or, nr) {
            rss_pairs.push((o, n));
        }
    }
    let verdict = judge_lifecycle_pairs(&wall_pairs, &rss_pairs);
    let old_w = median_u64(&wall_pairs.iter().map(|&(o, _)| o).collect::<Vec<_>>()) as f64 / 1e6;
    let new_w = median_u64(&wall_pairs.iter().map(|&(_, n)| n).collect::<Vec<_>>()) as f64 / 1e6;
    let rss_text = if rss_pairs.is_empty() {
        "peak RSS unavailable".to_owned()
    } else {
        let old_r = median_u64(&rss_pairs.iter().map(|&(o, _)| o).collect::<Vec<_>>()) as f64
            / (1024.0 * 1024.0);
        let new_r = median_u64(&rss_pairs.iter().map(|&(_, n)| n).collect::<Vec<_>>()) as f64
            / (1024.0 * 1024.0);
        format!("peak RSS {old_r:.1} -> {new_r:.1} MiB")
    };
    if verdict.wall_slower || verdict.rss_higher {
        let mut reasons = Vec::new();
        if verdict.wall_slower {
            reasons.push("wall time");
        }
        if verdict.rss_higher {
            reasons.push("peak RSS");
        }
        println!(
            "  SLOWER blake3-servil-mt|lifecycle|PipelineCreateDrop|4096 rooms: wall {old_w:.2} -> {new_w:.2} ms; {rss_text} (regressed: {})",
            reasons.join(", ")
        );
        false
    } else {
        println!(
            "regress: pipeline lifecycle (4096 rooms, {LIFECYCLE_PAIRS} pairs): wall {old_w:.2} -> {new_w:.2} ms; {rss_text} (level)"
        );
        true
    }
}

/// The cells whose instructions a unit moved beyond INSTRUCTIONS_MARGIN_
/// PERMILLE the same way in every pair, the largest median move first:
/// the solo cells of single-threaded contenders (whose calling thread
/// does the whole work), each counted in every pair (a cell a side lacks
/// counts for says nothing). Every pair, since a change in the code moves
/// the count the same way whatever the layout (#119's cells: x1.0119 in
/// five pairs to four places), where a count that moves with the link
/// layout or the run (the interleaved turns, 0.95-1.02 between pairs of
/// one build) moves both ways (data branch, 2026-11-04/judge3-i7 and
/// judge-i7, aa_instructions.py).
pub(crate) fn instructions_moved(
    ratios: &std::collections::BTreeMap<String, Vec<u64>>,
    pairs: usize,
) -> Vec<(String, u64)> {
    let mut moved: Vec<(String, u64)> = ratios
        .iter()
        .filter(|(key, r)| r.len() == pairs && instructions_tell(key))
        .filter_map(|(key, r)| {
            let mut sorted = r.clone();
            sorted.sort_unstable();
            let median = sorted[sorted.len() / 2];
            let (least, most) = (sorted[0], sorted[sorted.len() - 1]);
            let all_more = least > 1000 + INSTRUCTIONS_MARGIN_PERMILLE;
            let all_fewer = most < 1000 - INSTRUCTIONS_MARGIN_PERMILLE;
            (all_more || all_fewer).then(|| (key.clone(), median))
        })
        .collect();
    moved.sort_by_key(|(key, median)| (std::cmp::Reverse(median.abs_diff(1000)), key.clone()));
    moved
}

/// Whether a cell's calling-thread instructions are the code's alone: a
/// solo cell of a contender without a pool.
pub(crate) fn instructions_tell(key: &str) -> bool {
    let mut fields = key.split('|');
    let (Some(contender), Some(scenario)) = (fields.next(), fields.next()) else {
        return false;
    };
    scenario == "solo"
        && Algorithm::ALL
            .into_iter()
            .find(|a| a.key() == contender)
            .is_some_and(|a| !a.multithreaded())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regress_load_stops_on_lost_coverage_and_unavailable_instrument() {
        let w = |start_ns, end_ns, other_milli_cpus| clocks::load::Window {
            start_ns,
            end_ns,
            other_milli_cpus,
            steal_milli_cpus: 0,
        };

        // 1. Instrument becomes unavailable after an earlier window ending before end_ns:
        // must stop immediately with a non-quiet unavailable result instead of spinning.
        let mut sleeps = 0usize;
        let out = regress_load_with(
            1_000,
            5_000,
            true,
            REGRESS_LOAD_DEADLINE_NS,
            || (vec![w(0, 2_000, 100)], false, 0),
            || sleeps += 1,
        );
        assert_eq!(sleeps, 0);
        assert!(!out.starts_with("quiet"), "must not read as quiet: {out}");
        assert!(out.starts_with("unavailable:"), "{out}");

        // 2. Coverage stalls even while available=true: must stop at deadline.
        let elapsed = std::cell::Cell::new(0u64);
        let out = regress_load_with(
            1_000,
            5_000,
            true,
            100,
            || (vec![w(0, 2_000, 100)], true, elapsed.get()),
            || elapsed.set(elapsed.get() + 50),
        );
        assert_eq!(elapsed.get(), 100);
        assert!(out.starts_with("unavailable:"), "{out}");

        // 3. Window covers end_ns on the second poll: returns normal quiet line.
        let mut poll_n = 0usize;
        let out = regress_load_with(
            1_000,
            5_000,
            true,
            1_000,
            || {
                poll_n += 1;
                if poll_n == 1 {
                    (vec![w(0, 2_000, 100)], true, 50)
                } else {
                    (vec![w(0, 2_000, 100), w(2_000, 6_000, 50)], true, 100)
                }
            },
            || {},
        );
        assert!(out.starts_with("quiet:"), "{out}");
    }

    #[test]
    fn lifecycle_judge_catches_eager_faulting_without_startup_jitter_false_alarms() {
        const MIB: u64 = 1024 * 1024;
        const MS: u64 = 1_000_000;
        // 1. Level lazy create/drop passes.
        assert_eq!(
            judge_lifecycle_pairs(&[(2 * MS, 2 * MS); 4], &[(8 * MIB, 8 * MIB); 4],),
            LifecycleVerdict {
                wall_slower: false,
                rss_higher: false,
            }
        );
        // 2. Small child-process startup jitter (+50% relative, < +10 ms absolute) passes.
        assert_eq!(
            judge_lifecycle_pairs(&[(2 * MS, 3 * MS); 4], &[(8 * MIB, 12 * MIB); 4],),
            LifecycleVerdict {
                wall_slower: false,
                rss_higher: false,
            }
        );
        // 3. Eager page faulting (+94 ms wall, +256 MiB RSS) fails on both.
        assert_eq!(
            judge_lifecycle_pairs(&[(2 * MS, 96 * MS); 4], &[(8 * MIB, 264 * MIB); 4],),
            LifecycleVerdict {
                wall_slower: true,
                rss_higher: true,
            }
        );
        // 4. RSS-only regression (+256 MiB, wall unchanged) fails on RSS.
        assert_eq!(
            judge_lifecycle_pairs(&[(2 * MS, 2 * MS); 4], &[(8 * MIB, 264 * MIB); 4],),
            LifecycleVerdict {
                wall_slower: false,
                rss_higher: true,
            }
        );
    }
}
