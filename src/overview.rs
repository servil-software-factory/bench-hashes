//! `bench-hashes overview OUT.html DIR...`: one page above the maps. Each
//! row is a machine (one published results directory), each column one
//! representative measurement (FROZEN's "The overview"), and each cell the
//! ratio of a competitor's time to BLAKE3-Servil's in that run: read across
//! a row for one machine, down a column for one use across machines. The
//! cells are read from the same samples files the maps are drawn from,
//! through the same reader and summary rule; every figure on the page is
//! computed here, none typed.

use super::{read_samples, Algorithm, Measured, SamplesFile, UseCase, SAMPLES_VERSION};
use std::fmt::Write as _;
use std::path::Path;

/// One representative measurement: a column of the overview.
pub struct Pick {
    /// The column's heading.
    pub column: &'static str,
    /// The user's situation it stands for, under the heading.
    pub situation: &'static str,
    pub cell: Cell,
}

pub enum Cell {
    /// A hashing cell: `solo|use_case|point`, BLAKE3-Servil's contender in
    /// it, and whether BLAKE3 official's multithreaded form is the fair
    /// comparison (where servil's is multithreaded).
    Hashing {
        use_case: UseCase,
        point: &'static str,
        servil: Algorithm,
    },
    /// A b3sum cell: the cache state and the input, from `b3sum.samples.tsv`.
    B3sum {
        cache: &'static str,
        input: &'static str,
    },
}

/// The representative measurements, chosen once (FROZEN, "The overview").
/// The test frozen_contract_matches_frozen_md keeps this list and FROZEN's
/// block the same.
pub const PICKS: [Pick; 7] = [
    Pick {
        column: "64 B",
        situation: "a key, an identifier, a small record, hashed between a program's other work",
        cell: Cell::Hashing { use_case: UseCase::OneMessage, point: "64 B", servil: Algorithm::Blake3ServilSt },
    },
    Pick {
        column: "1 KiB",
        situation: "a small record, hashed between a program's other work",
        cell: Cell::Hashing { use_case: UseCase::OneMessage, point: "1 KiB", servil: Algorithm::Blake3ServilSt },
    },
    Pick {
        column: "1 MiB",
        situation: "a buffer the size of a file",
        cell: Cell::Hashing { use_case: UseCase::OneMessage, point: "1 MiB", servil: Algorithm::Blake3ServilSt },
    },
    Pick {
        column: "128 MiB",
        situation: "a download or a disk image",
        cell: Cell::Hashing { use_case: UseCase::OneMessage, point: "128 MiB", servil: Algorithm::Blake3ServilMt },
    },
    Pick {
        column: "64 KiB messages, one after another",
        situation: "messages arriving one after another, as fast as the program can feed them; BLAKE3-Servil's stream hashes them on its threads, the others on the calling thread",
        cell: Cell::Hashing { use_case: UseCase::ContinuousMessages, point: "64 KiB", servil: Algorithm::Blake3ServilMt },
    },
    Pick {
        column: "1024 messages of 64 B, batched",
        situation: "a Merkle layer, a content-addressed store, a database's keys: 1024 messages of 64 bytes in one call",
        cell: Cell::Hashing { use_case: UseCase::ManyMessages, point: "1024", servil: Algorithm::Blake3ServilSt },
    },
    Pick {
        column: "1000 files, b3sum",
        situation: "b3sum over a directory of 1000 files of assorted sizes, 74 MiB, in the page cache: the command itself, start to exit",
        cell: Cell::B3sum { cache: "warm", input: "mixed tree, 1000 files, 74 MiB" },
    },
];

/// The picks as FROZEN's block states them, one line each.
#[cfg(test)]
pub fn frozen_lines() -> String {
    let mut text = String::new();
    for pick in &PICKS {
        match &pick.cell {
            Cell::Hashing {
                use_case,
                point,
                servil,
            } => {
                let _ = writeln!(
                    text,
                    "overview cell: solo {use_case:?} {point}, {} ({})",
                    servil.key(),
                    pick.column
                );
            }
            Cell::B3sum { cache, input } => {
                let _ = writeln!(
                    text,
                    "overview cell: b3sum {cache} {input} ({})",
                    pick.column
                );
            }
        }
    }
    text
}

/// The b3sum names BLAKE3-Servil's build goes by in a run, newest convention
/// first (bench-hashes #55 settles on `b3sum`), and the competitors shown.
pub const FORK_B3SUM: [&str; 3] = ["b3sum", "devon", "fork"];
/// (run key, name shown, identity, colour, striped): b3sum's bars join
/// their library's multi-threaded form in the legend (b3sum hashes on
/// every core).
const B3SUM_COMPETITORS: [(&str, &str, &str, &str, bool); 4] = [
    (
        "official",
        "b3sum, official BLAKE3's",
        "blake3-official-mt",
        "#3b82f6",
        true,
    ),
    ("sha256sum", "sha256sum", "sha256", "#e07a45", false),
    // Drab greys: SHA-1 and MD5 are broken as cryptographic hashes
    // (collisions are cheap to make) and kept only as checksums people
    // still run; the page's door says so.
    ("sha1sum", "sha1sum", "sha1sum", "#b4b0a8", false),
    ("md5sum", "md5sum", "md5sum", "#cfcbc3", false),
];

/// One bar: a hash's speed in a cell.
#[derive(Clone)]
pub struct Bar {
    pub name: String,
    /// Which hash it is, for the legend's buttons: a contender's key (a
    /// b3sum bar shares its library's multi-threaded form's).
    pub id: &'static str,
    /// The map's colour for it: its library's.
    pub color: &'static str,
    /// A library's multi-threaded form: its colour in stripes.
    pub striped: bool,
    /// Bytes hashed per second, in GB/s (the map's unit), from the cell's
    /// mean through the shared rule.
    pub gbps: f64,
    /// A note the bar carries, such as "official on one thread".
    pub note: &'static str,
}

/// A cell's bars: BLAKE3-Servil's forms first, then each competitor's.
pub type Bars = Vec<Bar>;

/// Whether a bar is one of BLAKE3-Servil's forms (b3sum's shares the
/// multi-threaded form's key).
fn is_servil(b: &Bar) -> bool {
    b.id == Algorithm::Blake3ServilSt.key() || b.id == Algorithm::Blake3ServilMt.key()
}

/// A cell's three families, as the first view and the door draw them:
/// servil's form `form`, official's matching form (its multi-threaded one where servil's
/// is and the run measured it, else its one thread), and SHA-256.
fn trio<'a>(bars: &'a [Bar], form: Algorithm) -> [Option<&'a Bar>; 3] {
    let find = |a: Algorithm| bars.iter().find(|b| b.id == a.key());
    let servil = find(form);
    let official = match servil.map(|b| b.id) {
        Some(id) if id == Algorithm::Blake3ServilMt.key() => {
            find(Algorithm::Blake3Rayon).or_else(|| find(Algorithm::Blake3))
        }
        _ => find(Algorithm::Blake3),
    };
    [servil, official, find(Algorithm::Sha256)]
}

/// BLAKE3-Servil's other form.
fn other_form(servil: Algorithm) -> Algorithm {
    if servil == Algorithm::Blake3ServilMt {
        Algorithm::Blake3ServilSt
    } else {
        Algorithm::Blake3ServilMt
    }
}

/// The row's own servil bar: its pick's form (b3sum's, in its row), or on
/// a page without that form, the other.
fn ours<'a>(bars: &'a [Bar], pick: &Pick) -> Option<&'a Bar> {
    let form = match &pick.cell {
        Cell::Hashing { servil, .. } => *servil,
        Cell::B3sum { .. } => Algorithm::Blake3ServilMt,
    };
    bars.iter()
        .find(|b| b.id == form.key())
        .or_else(|| bars.iter().find(|b| b.id == other_form(form).key()))
}

/// One recorded run behind a machine's comparisons.
struct Record {
    label: &'static str,
    details: Vec<(String, String, String)>,
}

/// One machine's row.
pub struct Row {
    pub machine: String,
    pub cpus: String,
    pub os: String,
    pub day: String,
    pub fork: String,
    pub version: String,
    pub virtual_machine: bool,
    /// The results directory, as the page links to its maps.
    pub link: String,
    /// One entry per pick: its bars, or none where the run lacks the cell.
    pub cells: Vec<Bars>,
    /// Warnings from each present run (library or files), shared with the map.
    pub load_notes: Vec<String>,
    /// Exact recorded identities and settings, shown inside the hashes section.
    records: Vec<Record>,
    /// Set where the run is of another release of the benchmark: its
    /// samples are not read (their rule is their own release's), the row
    /// names the machine and links its map, and says a run under this
    /// release is awaited.
    pub stale: bool,
}

fn mean_q64(samples: &[Measured]) -> u128 {
    clocks::summary::mean(samples.iter().map(|m| (m.ns, m.units)))
}

/// A cell's speed in GB/s: `bytes_per_unit` bytes over its mean ns per
/// unit (the shared rule's Q64.64 figure; floating point at the last step).
fn gbps(samples: &[Measured], bytes_per_unit: f64) -> f64 {
    bytes_per_unit * 18_446_744_073_709_551_616.0 / mean_q64(samples) as f64
}

fn cell<'a>(file: &'a SamplesFile, key: &str) -> Option<&'a [Measured]> {
    file.cells
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, s)| s.as_slice())
}

/// The ids of BLAKE3-Servil's two forms as of the release before, drawn
/// where a machine's folder holds `before/` (`bench-hashes beside`).
const BEFORE_ST: &str = "blake3-servil-st@before";
const BEFORE_MT: &str = "blake3-servil-mt@before";

fn hashing_bars(
    file: &SamplesFile,
    before: Option<&SamplesFile>,
    use_case: UseCase,
    point: &str,
    servil: Algorithm,
) -> Bars {
    let key = |a: Algorithm| format!("{}|solo|{use_case:?}|{point}", a.key());
    let per_unit = match super::POINTS
        .iter()
        .find(|p| p.use_case == use_case && p.label == point)
    {
        Some(p) if use_case.batch() => p.message_len() as f64,
        _ => 1.0,
    };
    let bar = |a: Algorithm, samples: &[Measured], note| Bar {
        name: plain_name(a).to_owned(),
        id: a.key(),
        color: a.color(),
        striped: a.multithreaded(),
        gbps: gbps(samples, per_unit),
        note,
    };
    // The row's own form of servil must be measured; then every form of
    // each library the run measured, in the legend's order (servil's
    // single- and multi-threaded, official's), so the reader's choice of
    // forms holds in every row.
    // On a page of one kind of contender (bench-hashes beside --without),
    // the form the page has stands for the row.
    if cell(file, &key(servil)).is_none() && cell(file, &key(other_form(servil))).is_none() {
        return Vec::new();
    }
    let mut bars: Bars = [
        Algorithm::Blake3ServilSt,
        Algorithm::Blake3ServilMt,
        Algorithm::Blake3,
        Algorithm::Blake3Rayon,
    ]
    .into_iter()
    .filter_map(|a| cell(file, &key(a)).map(|s| bar(a, s, "")))
    .collect();
    // BLAKE3-Servil as of the release before, each form ahead of today's:
    // dark, named by its version's part before the `+`.
    if let Some(before) = before {
        let version = crate::map::servil_version(&before.headers);
        let version = version.split('+').next().unwrap_or("");
        for (a, id, at) in [
            (Algorithm::Blake3ServilMt, BEFORE_MT, 1),
            (Algorithm::Blake3ServilSt, BEFORE_ST, 0),
        ] {
            if let Some(samples) = cell(before, &key(a)) {
                let at = bars
                    .iter()
                    .position(|b| b.id == a.key())
                    .unwrap_or(at.min(bars.len()));
                bars.insert(
                    at,
                    Bar {
                        name: plain_name(a).replacen(
                            "BLAKE3-Servil",
                            &format!("BLAKE3-Servil {version}"),
                            1,
                        ),
                        id,
                        color: "#111827",
                        striped: a.multithreaded(),
                        gbps: gbps(samples, per_unit),
                        note: "the release before",
                    },
                );
            }
        }
    }
    // The fastest SHA-256 the run measured in this cell.
    let sha = [
        Algorithm::Sha256,
        Algorithm::Sha256Ring,
        Algorithm::Sha256CommonCrypto,
    ]
    .into_iter()
    .filter_map(|a| cell(file, &key(a)).map(|s| (a, mean_q64(s), s)))
    .min_by_key(|(_, m, _)| *m);
    if let Some((a, _, theirs)) = sha {
        // Named plainly; which implementation it was is the tooltip's
        // (the page's door says it is the fastest measured on the machine).
        bars.push(Bar {
            name: "SHA-256".to_owned(),
            id: Algorithm::Sha256.key(),
            color: Algorithm::Sha256.color(),
            striped: false,
            gbps: gbps(theirs, per_unit),
            note: match a {
                Algorithm::Sha256 => "the sha2 crate",
                Algorithm::Sha256Ring => "ring's",
                _ => "CommonCrypto's",
            },
        });
    }
    bars
}

fn b3sum_bars(file: &SamplesFile, cache: &str, input: &str) -> Bars {
    let key = |name: &str| format!("{name}|{cache}|b3sum|{input}");
    // b3sum's samples are ns over the input's bytes.
    let bar = |name: &str, id, color, striped, samples| Bar {
        name: name.to_owned(),
        id,
        color,
        striped,
        gbps: gbps(samples, 1.0),
        note: "",
    };
    let Some(ours) = FORK_B3SUM.iter().find_map(|n| cell(file, &key(n))) else {
        return Vec::new();
    };
    let mut bars = vec![bar(
        "b3sum, BLAKE3-Servil's",
        Algorithm::Blake3ServilMt.key(),
        Algorithm::Blake3ServilMt.color(),
        true,
        ours,
    )];
    bars.extend(
        B3SUM_COMPETITORS
            .iter()
            .filter_map(|(name, shown, id, color, striped)| {
                cell(file, &key(name)).map(|theirs| bar(shown, id, color, *striped, theirs))
            }),
    );
    bars
}

/// A bar's background: its colour, or a multi-threaded form's stripes of it.
fn paint(color: &str, striped: bool) -> String {
    if striped {
        format!("repeating-linear-gradient(135deg, {color} 0 3px, {color}73 3px 5px)")
    } else {
        color.to_owned()
    }
}

/// A contender's name in plain words: the map's "st" and "mt" spelled out.
fn plain_name(a: Algorithm) -> &'static str {
    match a {
        Algorithm::Blake3ServilSt => "BLAKE3-Servil, single-threaded",
        Algorithm::Blake3ServilMt => "BLAKE3-Servil, multi-threaded",
        Algorithm::Blake3 => "BLAKE3 official, single-threaded",
        Algorithm::Blake3Rayon => "BLAKE3 official, multi-threaded",
        other => other.name(),
    }
}

fn header<'a>(headers: &'a [(String, String)], key: &str) -> &'a str {
    headers
        .iter()
        .find(|(k, _)| k == key)
        .map_or("", |(_, v)| v.as_str())
}

/// A CPU's name as the vendor's string gives it, less its trademarks and
/// generation and core-count words: "12th Gen Intel(R) Core(TM) i7-12700K"
/// reads "Intel Core i7-12700K", "AMD EPYC 9654 96-Core Processor" "AMD EPYC 9654".
fn short_name(cpu: &str) -> String {
    let cleaned = cpu.replace("(R)", "").replace("(TM)", "");
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let number = |w: &str, suffix: &str| {
        w.strip_suffix(suffix)
            .is_some_and(|n| n.parse::<u32>().is_ok())
    };
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if number(w, "th") && words.get(i + 1) == Some(&"Gen") {
            i += 2;
        } else if w == "Processor" || w == "CPU" || number(w, "-Core") {
            i += 1;
        } else {
            out.push(w);
            i += 1;
        }
    }
    out.join(" ")
}

/// Read one results directory into a row: its hashing run, and its b3sum
/// run where it has one.
pub fn row(directory: &Path, link: String) -> Row {
    let hashing = directory.join("bench-hashes.samples.tsv");
    let b3sum = directory.join("b3sum.samples.tsv");
    let present: Vec<&Path> = [&hashing, &b3sum]
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| p.as_path())
        .collect();
    assert!(
        !present.is_empty(),
        "{} holds no samples file (bench-hashes.samples.tsv, b3sum.samples.tsv)",
        directory.display()
    );
    // A file of another release is left to its own tools: a folder with
    // none of this release's gets a row that names the machine and links
    // its map, its cells awaiting a run under this release.
    let current = |p: &Path| {
        std::fs::read_to_string(p)
            .expect("a samples file")
            .lines()
            .next()
            == Some(SAMPLES_VERSION)
    };
    if !present.iter().any(|p| current(p)) {
        // Header strings already name the old machine. Show those records
        // whole without interpreting the old format's measurement rows.
        let recorded: Vec<_> = present
            .iter()
            .map(|path| {
                let text = std::fs::read_to_string(path).expect("a samples file");
                let headers: Vec<(String, String)> = text
                    .lines()
                    .take_while(|l| l.starts_with('#'))
                    .filter_map(|l| l[1..].trim().split_once(": "))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                (path.file_name().unwrap().to_str().unwrap(), headers)
            })
            .collect();
        let mut row = described(
            &recorded[0].1,
            link,
            PICKS.iter().map(|_| Vec::new()).collect(),
            true,
        );
        row.records = recorded
            .iter()
            .map(|(name, headers)| Record {
                label: if *name == "bench-hashes.samples.tsv" {
                    "Library run"
                } else {
                    "File-tool run"
                },
                details: crate::map::details(
                    headers,
                    header(headers, "load"),
                    header(headers, "power"),
                    &format!("{}/{name}", row.link),
                ),
            })
            .collect();
        return row;
    }
    let hashing = (hashing.exists() && current(&hashing))
        .then(|| read_samples(hashing.to_str().expect("a path in UTF-8")));
    let b3sum = (b3sum.exists() && current(&b3sum))
        .then(|| read_samples(b3sum.to_str().expect("a path in UTF-8")));
    // The release before, where `bench-hashes beside` put it.
    let before = directory.join("before/bench-hashes.samples.tsv");
    let before = (before.exists() && current(&before))
        .then(|| read_samples(before.to_str().expect("a path in UTF-8")));
    let about = hashing
        .as_ref()
        .or(b3sum.as_ref())
        .expect("a samples file read");
    let cells = PICKS
        .iter()
        .map(|pick| match &pick.cell {
            Cell::Hashing {
                use_case,
                point,
                servil,
            } => hashing.as_ref().map_or(Vec::new(), |f| {
                hashing_bars(f, before.as_ref(), *use_case, point, *servil)
            }),
            Cell::B3sum { cache, input } => b3sum
                .as_ref()
                .map_or(Vec::new(), |f| b3sum_bars(f, cache, input)),
        })
        .collect();
    let mut row = described(&about.headers, link, cells, false);
    row.records = [
        ("Library run", "bench-hashes.samples.tsv", hashing.as_ref()),
        ("File-tool run", "b3sum.samples.tsv", b3sum.as_ref()),
        (
            "Earlier library run",
            "before/bench-hashes.samples.tsv",
            before.as_ref(),
        ),
    ]
    .into_iter()
    .filter_map(|(label, samples, file)| {
        let file = file?;
        Some(Record {
            label,
            details: crate::map::details(
                &file.headers,
                &file.load,
                &file.power,
                &format!("{}/{samples}", row.link),
            ),
        })
    })
    .collect();
    row.load_notes = [("Library", hashing.as_ref()), ("Files", b3sum.as_ref())]
        .into_iter()
        .filter_map(|(label, file)| {
            crate::map::load_warning(&file?.load).map(|warning| format!("{label}: {warning}"))
        })
        .collect();
    row
}

/// A row from a run's headers and its cells.
fn described(headers: &[(String, String)], link: String, cells: Vec<Bars>, stale: bool) -> Row {
    let os = header(headers, "os type");
    let os = if os.starts_with("darwin") {
        "macOS".to_owned()
    } else if os.starts_with("linux") {
        "Linux".to_owned()
    } else {
        os.to_owned()
    };
    let fork = crate::map::servil_version(headers);
    let identity = header(headers, "cpu identity");
    Row {
        machine: short_name(header(headers, "cpu type")),
        cpus: header(headers, "cpu count").to_owned(),
        virtual_machine: header(headers, "os type").contains("virt")
            || identity.contains("virt")
            || identity.contains("QEMU"),
        os,
        day: header(headers, "timestamp")
            .split(' ')
            .next()
            .unwrap_or("")
            .to_owned(),
        fork,
        version: header(headers, "bench-hashes version")
            .split('+')
            .next()
            .unwrap_or("")
            .to_owned(),
        link,
        cells,
        load_notes: Vec::new(),
        records: Vec::new(),
        stale,
    }
}

/// A ratio as the page says it, under the pointer: one figure after the
/// point below 10 ("2.5×"), whole numbers above ("12×"). Two figures would
/// claim a precision one run does not have.
fn figure(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{ratio:.0}×")
    } else {
        format!("{ratio:.1}×")
    }
}

/// A speed as the page says it: two significant figures, the precision one
/// run on one machine has.
fn speed(gbps: f64) -> String {
    if gbps <= 0.0 {
        return "0 GB/s".to_owned();
    }
    let digits = (1 - gbps.log10().floor() as i32).max(0) as usize;
    format!("{gbps:.digits$} GB/s")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The run records belong to the existing hashes disclosure. Each machine's
/// library, file-tool and earlier-library records keep their own identities.
fn records(rows: &[Row], order: &[usize]) -> String {
    let mut out = String::from(
        "<h3>Versions, sources and run settings</h3><p>Each run records its own versions and sources. Open a machine below for its contenders, build settings and raw samples.</p>",
    );
    for &i in order {
        let row = &rows[i];
        let _ = write!(
            out,
            "<details class=\"record\"><summary><b>{}</b> · {}, {} CPUs · {}</summary>",
            escape(&row.machine),
            escape(&row.os),
            escape(&row.cpus),
            escape(&row.day),
        );
        if row.stale {
            let _ = write!(
                out,
                "<p>These recorded headers describe an earlier benchmark format. Its measurements remain on its <a href=\"{}/bench-hashes.map.html\">original map</a>.</p>",
                escape(&row.link),
            );
        }
        for record in &row.records {
            let _ = write!(
                out,
                "<section class=\"run-record\"><h4>{}</h4><dl>",
                record.label
            );
            for (label, value, link) in &record.details {
                let _ = write!(out, "<dt>{}</dt><dd>", escape(label));
                if link.is_empty() {
                    out.push_str(&escape(value));
                } else {
                    let _ = write!(out, "<a href=\"{}\">{}</a>", escape(link), escape(value));
                }
                out.push_str("</dd>");
            }
            out.push_str("</dl></section>");
        }
        out.push_str("</details>\n");
    }
    out
}

/// The grid as text, for the terminal and the tests: each cell's speeds,
/// and servil's over each competitor's.
pub fn text(rows: &[Row]) -> String {
    let mut out = String::new();
    for row in rows {
        let _ = writeln!(
            out,
            "{}{}, {} CPUs, {} · {} · bench-hashes {}, BLAKE3-Servil {}",
            if row.virtual_machine { "(VM) " } else { "" },
            row.machine,
            row.cpus,
            row.os,
            row.day,
            row.version,
            row.fork
        );
        for note in &row.load_notes {
            let _ = writeln!(out, "  {note}");
        }
        if row.stale {
            let _ = writeln!(
                out,
                "  last measured under bench-hashes {}: a run under this release is awaited\n",
                row.version
            );
            continue;
        }
        for (pick, bars) in PICKS.iter().zip(&row.cells) {
            let line = match ours(bars, pick) {
                None => "not in this run".to_owned(),
                Some(ours) => {
                    let mut line = format!("{} {}", speed(ours.gbps), ours.name);
                    for b in bars.iter().filter(|b| !is_servil(b)) {
                        let note = if b.note.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", b.note)
                        };
                        let _ = write!(
                            line,
                            "; {} {} ({}){note}",
                            speed(b.gbps),
                            b.name,
                            figure(ours.gbps / b.gbps)
                        );
                    }
                    line
                }
            };
            let _ = writeln!(out, "  {:36} {line}", format!("{}:", pick.column));
        }
        out.push('\n');
    }
    out
}

/// The picks the first view draws: a short message (1 KiB on one thread,
/// between other work, where SHA-256 with the CPU's instructions leads)
/// inside the headline case (messages nonstop, 64 KiB each, every core),
/// so the view spans the range from where servil trails to where it
/// leads most (Zooko, October 24), by their index in PICKS.
/// The grid's columns, one a family of CPU (Zooko, October 27): in each,
/// the machine measured that is most like the family's machines shipping
/// through 2027. The Apple M4 Max stands for Apple silicon, the most common
/// Arm computer a program like this runs on; the Ryzen 7 9700X for AMD's
/// Zen 5 and its successors, desktops and laptops outnumbering servers; the
/// i7-12700K for Intel's hybrid PCs (performance and efficiency cores,
/// AVX2, no AVX-512). Where the results lack a family's chosen machine, its
/// first machine stands in, so a reader's own run shows their machine.
pub const CHOSEN: [(&str, &str); 3] = [
    ("Arm", "Apple M4 Max"),
    ("AMD", "AMD Ryzen 7 9700X"),
    ("Intel", "Intel Core i7-12700K"),
];

/// The families, in the grid's order.
const FAMILIES: [&str; 4] = ["Arm", "AMD", "Intel", "Other"];

/// A machine's family, from its CPU's name.
fn family(row: &Row) -> &'static str {
    let m = row.machine.as_str();
    if m.starts_with("Apple") || m.contains("aarch64") || m.contains("Neoverse") {
        "Arm"
    } else if m.starts_with("AMD") {
        "AMD"
    } else if m.starts_with("Intel") {
        "Intel"
    } else {
        "Other"
    }
}

/// The machine in each family's column, by index into `rows`, in the
/// families' order: the chosen machine where measured under this release,
/// else the family's first real machine with cells, else its first (a
/// chosen machine whose run is another release's left its column empty:
/// the 9700X at 0.20).
pub fn shown(rows: &[Row]) -> Vec<usize> {
    FAMILIES
        .iter()
        .filter_map(|f| {
            let members: Vec<usize> = (0..rows.len())
                .filter(|&i| family(&rows[i]) == *f)
                .collect();
            let chosen = CHOSEN.iter().find(|(c, _)| c == f).map(|(_, m)| *m);
            members
                .iter()
                .copied()
                .find(|&i| {
                    Some(rows[i].machine.as_str()) == chosen
                        && !rows[i].virtual_machine
                        && !rows[i].stale
                })
                .or_else(|| {
                    members
                        .iter()
                        .copied()
                        .find(|&i| !rows[i].virtual_machine && !rows[i].stale)
                })
                .or_else(|| members.first().copied())
        })
        .collect()
}

/// Each machine measured in plain words (its kind, design and year), for
/// its column and its card: (the CPU as the page names it, the words).
const KNOWN: [(&str, &str); 10] = [
    ("Apple M4 Max", "Apple laptop or desktop, 2024"),
    ("AMD Ryzen 7 9700X", "AMD desktop, Zen 5, 2024"),
    ("AMD EPYC 9255", "AMD server, Zen 5, 2024"),
    ("AMD EPYC 9254", "AMD server, Zen 4, 2022"),
    ("AMD EPYC 9275F", "AMD server, Zen 5, 2024"),
    ("AMD EPYC 7443P", "AMD server, Zen 3, 2021"),
    ("AMD EPYC 7443", "AMD server, two sockets, Zen 3, 2021"),
    ("Intel Core i7-12700K", "Intel desktop, Alder Lake, 2021"),
    ("Intel Xeon Gold 6330", "Intel server, Ice Lake, 2021"),
    (
        "Intel Xeon E5-1650 v4",
        "Intel workstation, Broadwell, 2016",
    ),
];

/// A machine in plain words: a virtual machine says so; a CPU not in
/// KNOWN goes by its family and core count.
fn plain(row: &Row) -> String {
    if row.virtual_machine {
        return format!("Linux in a virtual machine, {} CPUs", row.cpus);
    }
    KNOWN
        .iter()
        .find(|(m, _)| row.machine.starts_with(m))
        .map_or_else(
            || format!("{} machine, {} CPUs", family(row), row.cpus),
            |(_, w)| (*w).to_owned(),
        )
}

fn load_notes(row: &Row) -> String {
    row.load_notes
        .iter()
        .map(|note| format!("<span class=\"load-warning\">{}</span>", escape(note)))
        .collect()
}

const EVERYDAY: usize = 1;
const HEADLINE: usize = 4;

/// The first view: a row per machine, three bars (BLAKE3-Servil, BLAKE3
/// official, the fastest SHA-256). A bar's full length is the headline
/// case, the darker part inside it the everyday case; log scale, one for
/// every machine, so bars compare across rows. No figure on the page; the
/// figures and the cells' names under the pointer.
fn families_by_machine(all: &[Row], shown: &[usize]) -> String {
    let mut rows: Vec<(usize, &Row)> = all
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.stale && !r.cells[HEADLINE].is_empty() && !r.cells[EVERYDAY].is_empty())
        .collect();
    rows.sort_by_key(|(i, r)| {
        (
            FAMILIES.iter().position(|f| *f == family(r)),
            !shown.contains(i),
            *i,
        )
    });
    if rows.is_empty() {
        return String::new();
    }
    let all: Vec<f64> = rows
        .iter()
        .flat_map(|(_, r)| {
            r.cells[HEADLINE]
                .iter()
                .chain(&r.cells[EVERYDAY])
                .map(|b| b.gbps)
        })
        .collect();
    let lo_g = 10f64.powf(
        all.iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
            .log10()
            .floor(),
    );
    let hi_g = 10f64.powf(all.iter().cloned().fold(0.0, f64::max).log10().ceil());
    let width = |g: f64| 100.0 * (g.log10() - lo_g.log10()) / (hi_g.log10() - lo_g.log10());
    // The chart's own legend, drawn with it: a page with no such chart
    // (no machine with both cells) shows no key to it.
    let mut out = String::from(
        "<div class=\"fam-legend\"><span><i class=\"lo\"></i> one short message</span><span><i class=\"hi\"></i> a stream of messages, every core</span></div><div class=\"families\">",
    );
    for (i, r) in &rows {
        let _ = write!(
            out,
            "<div class=\"fam-row col{}\" data-m=\"{i}\" data-f=\"{}\"><div class=\"fam-machine\"><a href=\"{}/bench-hashes.map.html\"><b>{}</b></a><span>{}</span>{load}</div><div class=\"fam-bars\">",
            if shown.contains(i) { "" } else { " alt" },
            family(r),
            escape(&r.link),
            escape(&r.machine),
            escape(&plain(r)),
            load = load_notes(r)
        );
        // A bar a form, in the legend's order, each part from its own
        // form's cells, so every button shows or hides its form here as in
        // the grid (the page opens on servil multi-threaded and SHA-256).
        for (form, family) in [
            (Algorithm::Blake3ServilSt, "BLAKE3-Servil"),
            (Algorithm::Blake3ServilMt, "BLAKE3-Servil"),
            (Algorithm::Blake3, "BLAKE3 official"),
            (Algorithm::Blake3Rayon, "BLAKE3 official"),
            (Algorithm::Sha256, "SHA-256"),
        ] {
            let find = |cell: usize| r.cells[cell].iter().find(|b| b.id == form.key());
            let (Some(top), Some(inner)) = (find(HEADLINE), find(EVERYDAY)) else {
                continue;
            };
            let color = inner.color;
            let id = form.key();
            let p = paint(top.color, top.striped);
            let _ = write!(
                out,
                "<div class=\"fam-bar\" data-color=\"{color}\" data-head=\"{} · {}\" data-tip=\"{color}\t{}\t{}: {}\t{}\t{}: {}\"><i class=\"hi\" data-k=\"{id}\" data-g=\"{}\" style=\"width:{:.1}%;background:{p}\"></i><i class=\"lo\" data-k=\"{id}\" data-g=\"{}\" style=\"width:{:.1}%;background:{p}\"></i></div>",
                escape(&r.machine),
                escape(family),
                top.gbps,
                escape(&top.name),
                escape(PICKS[HEADLINE].situation),
                inner.gbps,
                escape(&inner.name),
                escape(PICKS[EVERYDAY].situation),
                top.gbps,
                width(top.gbps),
                inner.gbps,
                width(inner.gbps)
            );
        }
        out.push_str("</div></div>");
    }
    out.push_str("<div class=\"fam-row fam-scale\"><div class=\"fam-machine\"></div><div class=\"fam-bars\">");
    let mut e = lo_g.log10() as i32;
    while 10f64.powi(e) <= hi_g {
        let g = 10f64.powi(e);
        let _ = write!(
            out,
            "<span style=\"left:{:.1}%\">{}</span>",
            width(g),
            speed(g)
        );
        e += 1;
    }
    out.push_str("</div></div></div>");
    out
}

/// The door to every machine measured: a card each, grouped by family, in
/// plain words, with the headline contest at a glance (servil on every
/// core against SHA-256, messages nonstop); choosing one puts it in its
/// family's column.
fn door(rows: &[Row], shown: &[usize]) -> String {
    let mut out = format!(
        "<details class=\"more\" id=\"more\"><summary><b>All {} machines measured</b>: choose which one stands for each kind</summary><p class=\"glance-key\">Under each machine: BLAKE3-Servil against SHA-256, on a stream of messages. Click a machine to put it in the grid above.</p><div class=\"cards\">",
        rows.len()
    );
    for f in FAMILIES {
        let mut members: Vec<usize> = (0..rows.len()).filter(|&i| family(&rows[i]) == f).collect();
        if members.is_empty() {
            continue;
        }
        members.sort_by_key(|i| !shown.contains(i));
        let _ = write!(out, "<div class=\"fam-col\" data-f=\"{f}\"><h3>{f}</h3>");
        for i in members {
            let r = &rows[i];
            let [ours, _, sha] = trio(&r.cells[HEADLINE], Algorithm::Blake3ServilMt);
            let glance = match (ours, sha) {
                (Some(o), Some(h)) => {
                    let top = o.gbps.max(h.gbps);
                    format!(
                        "<span class=\"glance\" title=\"{}: BLAKE3-Servil, multi-threaded, {} against SHA-256, {}\"><i style=\"width:{:.0}%;background:{}\"></i><i style=\"width:{:.0}%;background:{}\"></i><em>{}</em></span>",
                        escape(PICKS[HEADLINE].column),
                        speed(o.gbps),
                        speed(h.gbps),
                        60.0 * o.gbps / top,
                        paint(o.color, o.striped),
                        60.0 * h.gbps / top,
                        h.color,
                        figure(o.gbps / h.gbps)
                    )
                }
                _ => String::new(),
            };
            let _ = write!(
                out,
                "<div class=\"card{}\" data-m=\"{i}\"><button type=\"button\" class=\"use\" data-m=\"{i}\"><b>{}</b><span>{} · {} CPUs, {} · {}</span>{load}{glance}<span class=\"state\">in the grid</span></button><a href=\"{}/bench-hashes.map.html\">its map \u{203A}</a></div>",
                if shown.contains(&i) { " on" } else { "" },
                escape(&plain(r)),
                escape(&r.machine),
                escape(&r.cpus),
                escape(&r.os),
                escape(&r.day),
                escape(&r.link),
                load = load_notes(r)
            );
        }
        out.push_str("</div>");
    }
    out.push_str("</div></details>\n");
    out
}

/// The page: a row per machine, a column per situation, and in each cell
/// a small bar chart in the map's colours, BLAKE3-Servil's bar first and
/// each competitor's under it, lengths in proportion to speed (the fastest
/// in the cell fills the width, its speed at the end as the scale). The
/// legend's buttons are the hashes drawn, each in its bar's colour, and
/// hide or show its bars; a pick under the pointer shows its figures in
/// the map's tooltip.
pub fn html(rows: &[Row]) -> String {
    // The legend: one button a hash, named for its first bar (b3sum's
    // bars join their library's multi-threaded form; the tooltip names
    // each bar exactly), in its bar's paint. A button hides or shows every
    // bar of its hash.
    // A library's form is named for the form, whichever bar of it the
    // page draws first (b3sum's bars share their library's
    // multi-threaded form's button).
    let form_name = |id: &str| {
        [
            Algorithm::Blake3ServilSt,
            Algorithm::Blake3ServilMt,
            Algorithm::Blake3,
            Algorithm::Blake3Rayon,
        ]
        .into_iter()
        .find(|a| a.key() == id)
        .map(plain_name)
    };
    let mut drawn: Vec<(&str, &str, &str, bool)> = Vec::new();
    for b in rows.iter().flat_map(|r| r.cells.iter().flatten()) {
        if !drawn.iter().any(|(_, id, _, _)| *id == b.id) {
            drawn.push((form_name(b.id).unwrap_or(&b.name), b.id, b.color, b.striped));
        }
    }
    // The release before's two forms first, then servil's two, official's
    // two, the SHA-256s, then the rest as drawn (sha1sum, md5sum).
    let order = [
        BEFORE_ST,
        BEFORE_MT,
        Algorithm::Blake3ServilSt.key(),
        Algorithm::Blake3ServilMt.key(),
        Algorithm::Blake3.key(),
        Algorithm::Blake3Rayon.key(),
        Algorithm::Sha256.key(),
        Algorithm::Sha256Ring.key(),
        Algorithm::Sha256CommonCrypto.key(),
    ];
    drawn.sort_by_key(|(_, id, _, _)| order.iter().position(|o| o == id).unwrap_or(usize::MAX));
    // A library's two forms, single- and multi-threaded, stand one above
    // the other: they are one contender in two forms, which SHA-256 is
    // related to in neither (Zooko, October 23). The release before is
    // such a contender too.
    let pairs = [
        (BEFORE_ST, BEFORE_MT),
        (
            Algorithm::Blake3ServilSt.key(),
            Algorithm::Blake3ServilMt.key(),
        ),
        (Algorithm::Blake3.key(), Algorithm::Blake3Rayon.key()),
    ];
    let button = |name: &str, id: &str, color: &str, striped: bool| {
        format!(
            "<button style=\"--c:{}\" data-k=\"{id}\">{}</button>",
            paint(color, striped),
            escape(name.split(" (").next().unwrap_or(name))
        )
    };
    let mut legend = String::new();
    let mut done: Vec<&str> = Vec::new();
    for (name, id, color, striped) in &drawn {
        if done.contains(id) {
            continue;
        }
        match pairs.iter().find(|(a, b)| a == id || b == id) {
            Some((a, b)) => {
                legend.push_str("<span class=\"pair\">");
                for form in [a, b] {
                    if let Some((n, k, c, st)) = drawn.iter().find(|(_, k, _, _)| k == form) {
                        legend.push_str(&button(n, k, c, *st));
                        done.push(k);
                    }
                }
                legend.push_str("</span>");
            }
            None => {
                legend.push_str(&button(name, id, color, *striped));
                done.push(id);
            }
        }
    }
    // Each cell on its own linear scale: its fastest bar fills the width,
    // the others in proportion, so every cell shows its contest at full
    // resolution and the bracket gives the factor (Zooko, October 24).
    // The grid, turned: a column per machine, a row per use case, so a
    // glance across a row compares machines. Machines of another release
    // get a column that names their last run and links its map.
    // A column a machine, in the families' order: each family's shown
    // machine visible, its others waiting (class "alt") until the door
    // swaps one in.
    let shown = shown(rows);
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by_key(|&i| {
        (
            FAMILIES.iter().position(|f| *f == family(&rows[i])),
            !shown.contains(&i),
            i,
        )
    });
    let col = |i: usize| {
        format!(
            "class=\"col{}\" data-m=\"{i}\" data-f=\"{}\"",
            if shown.contains(&i) { "" } else { " alt" },
            family(&rows[i])
        )
    };
    let mut body = String::new();
    body.push_str("<div class=\"grid\"><table><thead><tr><th class=\"situation\"></th>");
    for &i in &order {
        let row = &rows[i];
        let others = rows.iter().filter(|r| family(r) == family(row)).count() - 1;
        let _ = write!(
            body,
            "<th {}><a href=\"{}/bench-hashes.map.html\"><b>{}</b></a><span class=\"kind\">{}</span><span>{} CPUs, {} · {}</span>{load}{}{}</th>",
            col(i),
            escape(&row.link),
            escape(&row.machine),
            escape(&plain(row)),
            escape(&row.cpus),
            escape(&row.os),
            escape(&row.day),
            if row.stale {
                "<span class=\"stale\">earlier benchmark: its results remain on its map; a current run is awaited</span>"
            } else {
                ""
            },
            if others > 0 {
                format!(
                    "<a class=\"swap\" href=\"#more\" data-f=\"{0}\">{others} other {0} machine{1} \u{25BE}</a>",
                    family(row),
                    if others == 1 { "" } else { "s" }
                )
            } else {
                String::new()
            },
            load = load_notes(row)
        );
    }
    body.push_str("</tr></thead><tbody>\n");
    for (i, pick) in PICKS.iter().enumerate() {
        // A use no machine of the page measured under this release has no
        // row (a page of one crate's calls, `bench-hashes beside --without`).
        if rows.iter().all(|r| r.stale || r.cells[i].is_empty()) && rows.iter().any(|r| !r.stale) {
            continue;
        }
        {
            let column = pick.column;
            let _ = write!(
                body,
                "<tr><td class=\"situation\" title=\"{}\">{}</td>",
                escape(pick.situation),
                escape(column)
            );
            for &m in &order {
                let row = &rows[m];
                let class = format!(" {}", col(m));
                if row.stale {
                    let _ = write!(body, "<td{class}></td>");
                    continue;
                }
                let bars = &row.cells[i];
                let fastest = bars.iter().map(|b| b.gbps).fold(0.0f64, f64::max);
                let width = |g: f64| {
                    if fastest > 0.0 {
                        100.0 * g / fastest
                    } else {
                        0.0
                    }
                };
                let map = match &PICKS[i].cell {
                    Cell::Hashing { use_case, .. } => format!(
                        "{}/bench-hashes.map.html#{}",
                        row.link,
                        super::map::cell_key("solo", *use_case)
                    ),
                    Cell::B3sum { .. } => {
                        format!("{}/bench-hashes.map.html#1:solo|tree|cached", row.link)
                    }
                };
                if bars.is_empty() {
                    let _ = write!(
                        body,
                        "<td{class}><div class=\"pick none\">not in this run</div></td>"
                    );
                    continue;
                }
                let tip: Vec<String> = bars
                    .iter()
                    .map(|b| {
                        format!(
                            "{}\t{}\t{}\t{}",
                            paint(b.color, b.striped),
                            b.name,
                            b.note,
                            b.gbps
                        )
                    })
                    .collect();
                let _ = write!(
                    body,
                    "<td{class}><a class=\"pick\" href=\"{}\" data-head=\"{} · {}\" data-tip=\"{}\"><span class=\"bars\">",
                    escape(&map),
                    escape(&row.machine),
                    escape(column),

                    escape(&tip.join("\n"))
                );
                for b in bars {
                    let _ = write!(
                        body,
                        "<span class=\"bar\"><i data-k=\"{}\" data-color=\"{}\" data-g=\"{}\" style=\"width:{:.1}%;background:{}\"></i></span>",
                        b.id,
                        b.color,
                        b.gbps,
                        width(b.gbps),
                        paint(b.color, b.striped)
                    );
                }
                // The scale: the speed the full width stands for, under its end.
                let _ = write!(
                    body,
                    "</span><span class=\"scale\">{}</span></a></td>",
                    speed(fastest)
                );
            }
            body.push_str("</tr>\n");
        }
    }
    body.push_str("</tbody></table></div>\n");
    body.push_str(&door(rows, &shown));
    include_str!("overview.html")
        .replace("@FAMILIES@", &families_by_machine(rows, &shown))
        .replace("@LEGEND@", &legend)
        .replace("@BODY@", &body)
        .replace("@RECORDS@", &records(rows, &order))
}

/// The rows of the results directories under `results`, in the order they
/// are named (so the page is the same from any shell), linked from `base`'s
/// directory. A machine whose last run is another release's gets a row
/// with no cells (its samples are its release's to read), so one page
/// shows one release's cells and names every machine.
pub fn rows(results: &Path, base: &Path) -> Vec<Row> {
    let dirs = results_dirs(results);
    assert!(
        !dirs.is_empty(),
        "{} holds no results directory with a samples file",
        results.display()
    );
    let rows: Vec<Row> = dirs
        .iter()
        .map(|dir| {
            let link = dir
                .strip_prefix(base)
                .unwrap_or(dir)
                .to_str()
                .expect("a path in UTF-8")
                .to_owned();
            row(dir, link)
        })
        .collect();
    rows
}

/// The results directories under `results` that hold a samples file, in
/// name order.
fn results_dirs(results: &Path) -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<_> = std::fs::read_dir(results)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", results.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|p| {
            p.join("bench-hashes.samples.tsv").exists() || p.join("b3sum.samples.tsv").exists()
        })
        .collect();
    dirs.sort();
    dirs
}

/// After a run into `directory` (`benchmark-results/<machine>`): redraw
/// the overview beside it, `index.html` in the directory above the
/// results, so a run keeps the front page current. A machine whose last
/// run is another release's keeps its row, with its map and a note that
/// a run under this release is awaited, and no cells.
pub fn refresh(directory: &Path) {
    let Some(results) = directory.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return;
    };
    let base = results.parent().unwrap_or(Path::new(""));
    let index = base.join("index.html");
    if !index.exists() {
        return;
    }
    let rows: Vec<Row> = results_dirs(results)
        .iter()
        .map(|dir| {
            row(
                dir,
                dir.strip_prefix(base)
                    .unwrap_or(dir)
                    .to_str()
                    .expect("a path in UTF-8")
                    .to_owned(),
            )
        })
        .collect();
    std::fs::write(&index, html(&rows))
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", index.display()));
    println!("# Overview (HTML) is in \"{}\" .", index.display());
}

/// `bench-hashes overview OUT.html RESULTS`: write the page from every
/// results directory under RESULTS (`benchmark-results`), and print its
/// text. The test the_index_is_the_overview_of_the_published_runs keeps
/// the repository's index.html equal to this command's output.
pub fn command(args: &[String]) {
    let usage =
        "usage: bench-hashes overview OUT.html RESULTS (the directory of results directories)";
    let [out, results] = args else {
        panic!("{usage}")
    };
    let (out, results) = (Path::new(out), Path::new(results));
    let base = out.parent().unwrap_or(Path::new(""));
    let rows = rows(results, base);
    std::fs::write(out, html(&rows))
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    print!("{}", text(&rows));
    println!("# Overview (HTML) is in \"{}\" .", out.display());
}

#[cfg(test)]
mod tests {
    /// A machine's folder with `before/` beside its samples (bench-hashes
    /// beside) draws BLAKE3-Servil as of the release before as a bar of
    /// its own, just ahead of today's form, named by its version; and on a
    /// page without the pick's form of BLAKE3-Servil the other form stands
    /// for the row.
    #[test]
    fn the_release_before_is_a_bar_ahead_of_todays() {
        let dir =
            std::env::temp_dir().join(format!("bench-overview-before-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("before")).unwrap();
        let file = |version: &str, ns: u32, sha: bool| {
            let row = |who: &str, ns: u32| {
                format!("{who}\tsolo\tOneMessage\t128 MiB\tB\t{ns}/64,{ns}/64\t1,2\t-\t-\t-\t0,0\n")
            };
            format!(
                "{}\n# bench-hashes version: test\n# blake3-servil version: {version}\n# load: quiet\n# power: mains power\n{}\n{}{}",
                super::SAMPLES_VERSION,
                crate::SAMPLES_COLUMNS,
                row("blake3-servil-st", ns),
                if sha { row("sha256", 4 * ns) } else { String::new() }
            )
        };
        std::fs::write(
            dir.join("bench-hashes.samples.tsv"),
            file("0.9.0+dev.abc", 80, true),
        )
        .unwrap();
        std::fs::write(
            dir.join("before/bench-hashes.samples.tsv"),
            file("0.8.0+b3c5", 100, false),
        )
        .unwrap();
        let row = super::row(&dir, "x".to_owned());
        // The 128 MiB pick's own form is the multi-threaded one, which this page lacks.
        let pick = super::PICKS
            .iter()
            .position(|p| p.column == "128 MiB")
            .unwrap();
        let bars = &row.cells[pick];
        assert_eq!(
            bars.iter()
                .map(|b| (b.id, b.name.as_str()))
                .collect::<Vec<_>>(),
            [
                (super::BEFORE_ST, "BLAKE3-Servil 0.8.0, single-threaded"),
                ("blake3-servil-st", "BLAKE3-Servil, single-threaded"),
                ("sha256", "SHA-256"),
            ]
        );
        assert!(
            bars[1].gbps > bars[0].gbps,
            "today's 80 ns against the release before's 100"
        );
        assert_eq!(
            super::ours(bars, &super::PICKS[pick]).map(|b| b.id),
            Some("blake3-servil-st")
        );
        // This machine has the long cell alone, so the page draws no chart
        // of one short message inside a stream, and no key to one.
        assert_eq!(super::families_by_machine(&[row], &[0]), "");
        assert!(!include_str!("overview.html").contains("class=\"fam-legend\""));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    use super::*;

    #[test]
    fn each_runs_load_is_visible_without_changing_its_bars() {
        let dir = std::env::temp_dir().join(format!("overview-load-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |load: &str, rows: &str| {
            format!(
            "{SAMPLES_VERSION}\n# bench-hashes version: test\n# cpu type: Example\n# load: {load}\n# power: mains power\n{}\n{rows}",
            crate::SAMPLES_COLUMNS
        )
        };
        let hashing = "blake3-servil-st\tsolo\tOneMessage\t1 KiB\tB\t80/64,80/64\t1,2\t-\t-\t-\t0,0\nblake3-servil-mt\tsolo\tContinuousMessages\t64 KiB\tB\t80/64,80/64\t1,2\t-\t-\t-\t0,0\n";
        let files = "b3sum\twarm\tb3sum\tmixed tree, 1000 files, 74 MiB\tB\t160/64,160/64\t1,2\t-\t-\t-\t0,0\n";
        std::fs::write(
            dir.join("bench-hashes.samples.tsv"),
            file("busy: busy in 1 of 362 windows", hashing),
        )
        .unwrap();
        std::fs::write(dir.join("b3sum.samples.tsv"), file("not measured", files)).unwrap();
        let qualified = super::row(&dir, "example".to_owned());
        assert_eq!(
            qualified.load_notes,
            [
                "Library: other programs busy in 1 of 362 windows; times may be slower",
                "Files: background load unobserved; times need qualification"
            ]
        );
        let page = html(std::slice::from_ref(&qualified));
        assert_eq!(
            page.matches("class=\"load-warning\"").count(),
            6,
            "both warnings on the first view, grid header and machine card"
        );
        assert!(text(std::slice::from_ref(&qualified)).contains(&qualified.load_notes[1]));
        std::fs::write(dir.join("bench-hashes.samples.tsv"), file("quiet", hashing)).unwrap();
        std::fs::write(dir.join("b3sum.samples.tsv"), file("quiet", files)).unwrap();
        let quiet = super::row(&dir, "example".to_owned());
        assert!(quiet.load_notes.is_empty());
        assert!(!html(std::slice::from_ref(&quiet)).contains("class=\"load-warning\""));
        let speeds = |r: &Row| {
            r.cells
                .iter()
                .map(|bars| bars.iter().map(|b| b.gbps).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            speeds(&qualified),
            speeds(&quiet),
            "load qualification leaves arithmetic summaries unchanged"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_speed_is_bytes_over_the_shared_rule_s_mean() {
        // Each cell's mean is the shared rule's (clocks::summary::mean), the
        // map's: total time over total work, every sample counted. 1500 ns
        // over 500 bytes twice is 3 ns/B, 0.333 GB/s; 600 and 400 over 500
        // each is 1 ns/B, 1 GB/s. Independently fixed.
        let theirs = [Measured::new(1500, 500), Measured::new(1500, 500)];
        let ours = [Measured::new(600, 500), Measured::new(400, 500)];
        assert!((gbps(&theirs, 1.0) - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!(gbps(&ours, 1.0), 1.0);
        // A batch's unit is a message of 64 bytes: 64 ns a message is 1 GB/s.
        assert_eq!(gbps(&[Measured::new(6400, 100)], 64.0), 1.0);
    }

    #[test]
    fn figures_read_as_the_page_says_them() {
        assert_eq!(figure(0.925), "0.9×");
        assert_eq!(figure(1.0), "1.0×");
        assert_eq!(figure(9.96), "10.0×");
        assert_eq!(figure(12.4), "12×");
        assert_eq!(speed(0.1234), "0.12 GB/s");
        assert_eq!(speed(0.06834), "0.068 GB/s");
        assert_eq!(speed(3.61), "3.6 GB/s");
        assert_eq!(speed(23.46), "23 GB/s");
        assert_eq!(speed(159.4), "159 GB/s");
    }

    #[test]
    fn vendor_strings_lose_their_trademarks_generations_and_core_counts() {
        assert_eq!(
            short_name("12th Gen Intel(R) Core(TM) i7-12700K"),
            "Intel Core i7-12700K"
        );
        assert_eq!(
            short_name("AMD EPYC 9654 96-Core Processor"),
            "AMD EPYC 9654"
        );
        assert_eq!(
            short_name("AMD EPYC 9255 24-Core Processor"),
            "AMD EPYC 9255"
        );
        assert_eq!(short_name("Apple M4 Max"), "Apple M4 Max");
        assert_eq!(
            short_name("Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz"),
            "Intel Xeon E5-2680 v4 @ 2.40GHz"
        );
    }

    fn row(machine: &str, vm: bool, cells: Vec<Bars>) -> Row {
        Row {
            machine: machine.to_owned(),
            cpus: "8".to_owned(),
            os: "Linux".to_owned(),
            day: "2026-10-22".to_owned(),
            fork: "abc1234".to_owned(),
            version: "0.17.0".to_owned(),
            virtual_machine: vm,
            link: "benchmark-results/x".to_owned(),
            cells,
            load_notes: Vec::new(),
            records: Vec::new(),
            stale: false,
        }
    }

    /// A bar as the page's readers make it: its hash's key, colour and
    /// pattern from its name (b3sum's bars join their library's
    /// multi-threaded form).
    fn bar(name: &str, gbps: f64, note: &'static str) -> Bar {
        let (a, id) = match name {
            "BLAKE3-Servil st" => (Algorithm::Blake3ServilSt, Algorithm::Blake3ServilSt.key()),
            "BLAKE3-Servil mt" | "b3sum, BLAKE3-Servil's" => {
                (Algorithm::Blake3ServilMt, Algorithm::Blake3ServilMt.key())
            }
            "BLAKE3 official" => (Algorithm::Blake3, Algorithm::Blake3.key()),
            "b3sum, official BLAKE3's" => (Algorithm::Blake3Rayon, Algorithm::Blake3Rayon.key()),
            "SHA-256" => (Algorithm::Sha256, Algorithm::Sha256.key()),
            "md5sum" => {
                return Bar {
                    name: name.to_owned(),
                    id: "md5sum",
                    color: "#cfcbc3",
                    striped: false,
                    gbps,
                    note,
                };
            }
            other => panic!("no test bar named {other}"),
        };
        Bar {
            name: name.to_owned(),
            id,
            color: a.color(),
            striped: a.multithreaded(),
            gbps,
            note,
        }
    }

    #[test]
    fn the_page_draws_bars_in_proportion_and_a_legend_of_what_it_drew() {
        let mut cells: Vec<Bars> = PICKS.iter().map(|_| Vec::new()).collect();
        // Servil 0.9× official (official fills the width), 1.2× SHA-256.
        cells[0] = vec![
            bar("BLAKE3-Servil st", 0.9, ""),
            bar("BLAKE3 official", 1.0, ""),
            bar("SHA-256", 0.75, ""),
        ];
        cells[3] = vec![
            bar("BLAKE3-Servil mt", 12.0, ""),
            bar(
                "BLAKE3 official",
                1.0,
                "official on one thread: this run lacks its multithreaded form",
            ),
        ];
        cells[6] = vec![
            bar("b3sum, BLAKE3-Servil's", 30.0, ""),
            bar("b3sum, official BLAKE3's", 10.0, ""),
            bar("md5sum", 1.5, ""),
        ];
        let vm = row("virt", true, PICKS.iter().map(|_| Vec::new()).collect());
        let rows = [row("A", false, cells), vm];
        let page = html(&rows);
        // Each cell on its own linear scale: its fastest bar fills the width
        // (official's 1.0 GB/s in the first cell; servil's b3sum in the
        // last), the others in proportion.
        assert!(
            page.contains("width:90.0%;background:#7c3aed"),
            "servil's bar in proportion to the cell's fastest"
        );
        assert!(
            page.contains("width:100.0%;background:#3b82f6"),
            "the cell's fastest fills the width"
        );
        assert!(page.contains("width:75.0%;background:#e07a45"));
        assert!(
            page.contains("width:5.0%;background:#cfcbc3"),
            "md5sum's 1.5 of b3sum's 30"
        );
        assert!(
            page.contains("<th class=\"col alt\" data-m=\"1\""),
            "a virtual machine waits behind the door"
        );
        assert!(
            page.contains("<b>All 2 machines measured</b>"),
            "the door holds every machine"
        );
        // The legend: one button a colour, servil first, b3sum's sharing.
        let legend: Vec<&str> = page
            .split("<button style=\"--c:")
            .skip(1)
            .map(|s| &s[..s.find('"').unwrap()])
            .collect();
        // Each library's two forms in its one colour, the multi-threaded
        // striped; official's multi-threaded button is b3sum's bar's.
        assert_eq!(
            legend,
            [
                "#7c3aed".to_owned(),
                paint("#7c3aed", true),
                "#3b82f6".to_owned(),
                paint("#3b82f6", true),
                "#e07a45".to_owned(),
                "#cfcbc3".to_owned()
            ]
        );
        // A form's button is named for the form, though its first bar
        // drawn here is official's b3sum.
        assert!(
            page.contains(">BLAKE3 official, multi-threaded</button>"),
            "official's multi-threaded button named for its form"
        );
        assert!(!page.contains(">b3sum, official BLAKE3's</button>"));
        assert!(text(&rows).contains(
            "1000 files, b3sum:                   30 GB/s b3sum, BLAKE3-Servil's; 10 GB/s b3sum, official BLAKE3's (3.0×); 1.5 GB/s md5sum (20×)"
        ));
    }

    /// The front page is the command's output for the published runs, so it
    /// cannot drift from the data: a new or redrawn run means
    /// `bench-hashes overview index.html benchmark-results`.
    #[test]
    fn the_index_is_the_overview_of_the_published_runs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let want = html(&rows(&root.join("benchmark-results"), root));
        let have = std::fs::read_to_string(root.join("index.html")).expect("index.html");
        assert!(
            have == want,
            "index.html is stale: run `bench-hashes overview index.html benchmark-results`"
        );
    }

    /// A run redraws the overview above its results directory, and leaves
    /// a machine's row without cells where its run is another release's.
    #[test]
    fn a_run_refreshes_the_overview_above_its_results() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let site = std::env::temp_dir().join(format!("overview-refresh-{}", std::process::id()));
        let results = site.join("benchmark-results");
        std::fs::create_dir_all(&results).unwrap();
        let mut copied = Vec::new();
        for dir in results_dirs(&root.join("benchmark-results")) {
            let to = results.join(dir.file_name().unwrap());
            std::fs::create_dir(&to).unwrap();
            for file in ["bench-hashes.samples.tsv", "b3sum.samples.tsv"] {
                if dir.join(file).exists() {
                    std::fs::copy(dir.join(file), to.join(file)).unwrap();
                }
            }
            copied.push(to);
        }
        std::fs::write(site.join("index.html"), "stale").unwrap();
        refresh(&copied[0]);
        let drawn = std::fs::read_to_string(site.join("index.html")).unwrap();
        assert_eq!(
            drawn,
            html(&rows(&results, &site)),
            "the page above the results is redrawn"
        );
        // Another release among the results: its machine keeps a row
        // that says so, with its map, and no cells.
        for file in ["bench-hashes.samples.tsv", "b3sum.samples.tsv"] {
            let samples = copied[0].join(file);
            if samples.exists() {
                let text = std::fs::read_to_string(&samples)
                    .unwrap()
                    .replace(SAMPLES_VERSION, "# bench-hashes samples v0");
                std::fs::write(&samples, text).unwrap();
            }
        }
        refresh(&copied[0]);
        let drawn = std::fs::read_to_string(site.join("index.html")).unwrap();
        assert!(
            drawn.contains(
                "earlier benchmark: its results remain on its map; a current run is awaited"
            ),
            "the earlier machine's results keep their own renderer"
        );
        std::fs::remove_dir_all(&site).unwrap();
    }

    /// A column a family (Zooko, October 27): its chosen machine where
    /// measured, else its first real machine, else (only a virtual one
    /// measured) that one; the others wait behind the door.
    #[test]
    fn a_column_a_family_its_chosen_machine_first() {
        let none = || PICKS.iter().map(|_| Vec::new()).collect::<Vec<Bars>>();
        let rows = [
            row("AMD EPYC 9255", false, none()),
            row("AMD Ryzen 7 9700X", false, none()),
            row("aarch64", true, none()),
            row("Intel Xeon Gold 6330 @ 2.00GHz", false, none()),
        ];
        // Arm, AMD, Intel: the virtual machine alone; the chosen 9700X over
        // the EPYC named first; the Xeon, the family's only machine.
        assert_eq!(shown(&rows), [2, 1, 3]);
        let rows = [
            row("aarch64", true, none()),
            row("Apple M4 Max", false, none()),
            row("Intel Core i7-12700K", false, none()),
        ];
        assert_eq!(
            shown(&rows),
            [1, 2],
            "the chosen Mac over the virtual machine"
        );
        assert_eq!(plain(&rows[1]), "Apple laptop or desktop, 2024");
        assert_eq!(plain(&rows[0]), "Linux in a virtual machine, 8 CPUs");
        // A chosen machine whose run is another release's gives way to the
        // family's first machine measured under this one.
        let mut rows = [
            row("AMD EPYC 7443P 24-Core Processor", false, none()),
            row("AMD EPYC 9255 24-Core Processor", false, none()),
            row("AMD Ryzen 7 9700X", false, none()),
        ];
        rows[0].stale = true;
        rows[2].stale = true;
        assert_eq!(
            shown(&rows),
            [1],
            "the EPYC 9255, the 9700X's run being 0.18's"
        );
    }

    #[test]
    fn the_picks_keep_to_the_budget() {
        assert_eq!(PICKS.len(), 7, "the budget: at most eight cells");
        let mut labels: Vec<&str> = PICKS.iter().map(|p| p.column).collect();
        labels.dedup();
        assert_eq!(labels.len(), PICKS.len(), "each row its own label");
    }

    /// The three families of a cell: official's form follows servil's
    /// (its multi-threaded one where servil's is and the run has it, else
    /// its one thread).
    #[test]
    fn officials_form_follows_servils() {
        let bars = vec![
            bar("BLAKE3-Servil st", 2.0, ""),
            bar("BLAKE3-Servil mt", 9.0, ""),
            bar("BLAKE3 official", 1.0, ""),
            bar("b3sum, official BLAKE3's", 5.0, ""),
            bar("SHA-256", 0.5, ""),
        ];
        let ids = |form| trio(&bars, form).map(|b| b.map(|b| b.id));
        let (st, mt) = (Algorithm::Blake3ServilSt, Algorithm::Blake3ServilMt);
        let (official, rayon, sha) = (
            Algorithm::Blake3.key(),
            Algorithm::Blake3Rayon.key(),
            Algorithm::Sha256.key(),
        );
        assert_eq!(ids(mt), [Some(mt.key()), Some(rayon), Some(sha)]);
        assert_eq!(
            ids(st),
            [Some(st.key()), Some(official), Some(sha)],
            "one thread beside one thread"
        );
        let without_rayon: Vec<Bar> = bars.iter().filter(|b| b.id != rayon).cloned().collect();
        assert_eq!(
            trio(&without_rayon, mt).map(|b| b.map(|b| b.id)),
            [Some(mt.key()), Some(official), Some(sha)],
            "a run without official's multi-threaded form"
        );
    }

    /// Each form of the release before stands just ahead of today's form in
    /// a cell's bars; in the legend its two forms are a pair ahead of
    /// today's pair, whatever order the page met them in.
    #[test]
    fn each_release_before_stands_just_ahead_of_todays_form() {
        let dir = std::env::temp_dir().join(format!("bench-overview-both-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("before")).unwrap();
        let file = |version: &str, rows: &[(&str, u32)]| {
            let rows: String = rows
                .iter()
                .map(|(who, ns)| {
                    format!(
                        "{who}\tsolo\tOneMessage\t128 MiB\tB\t{ns}/64,{ns}/64\t1,2\t-\t-\t-\t0,0\n"
                    )
                })
                .collect();
            format!(
                "{}\n# bench-hashes version: test\n# blake3-servil version: {version}\n# load: quiet\n# power: mains power\n{}\n{rows}",
                super::SAMPLES_VERSION,
                crate::SAMPLES_COLUMNS
            )
        };
        std::fs::write(
            dir.join("bench-hashes.samples.tsv"),
            file(
                "0.9.0+dev.abc",
                &[
                    ("blake3-servil-st", 80),
                    ("blake3-servil-mt", 20),
                    ("blake3-official", 90),
                    ("sha256", 400),
                ],
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("before/bench-hashes.samples.tsv"),
            file(
                "0.8.0+b3c5",
                &[("blake3-servil-st", 100), ("blake3-servil-mt", 25)],
            ),
        )
        .unwrap();
        let row = super::row(&dir, "x".to_owned());
        let pick = PICKS.iter().position(|p| p.column == "128 MiB").unwrap();
        let ids: Vec<&str> = row.cells[pick].iter().map(|b| b.id).collect();
        assert_eq!(
            ids,
            [
                BEFORE_ST,
                "blake3-servil-st",
                BEFORE_MT,
                "blake3-servil-mt",
                "blake3-official",
                "sha256"
            ]
        );
        // The legend: the same order, though the page meets the bars in another.
        let mut scrambled = row.cells[pick].clone();
        scrambled.reverse();
        let mut cells: Vec<Bars> = PICKS.iter().map(|_| Vec::new()).collect();
        cells[pick] = scrambled;
        let page = html(&[self::row("AMD Ryzen 7 9700X", false, cells)]);
        let legend = &page[page.find("<div class=\"legend\">").unwrap()..];
        let legend = &legend[..legend.find("</div>").unwrap()];
        let keys: Vec<&str> = legend
            .split("data-k=\"")
            .skip(1)
            .map(|rest| &rest[..rest.find('"').unwrap()])
            .collect();
        assert_eq!(
            keys,
            [
                BEFORE_ST,
                BEFORE_MT,
                "blake3-servil-st",
                "blake3-servil-mt",
                "blake3-official",
                "sha256"
            ],
            "the release before's two forms as a pair, ahead of today's pair"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A use no machine of the page measured under this release has no row;
    /// a page whose machines all await a run under this release keeps every
    /// row, empty.
    #[test]
    fn a_row_no_fresh_machine_measured_is_left_out() {
        let label = |pick: usize| format!(">{}<", PICKS[pick].column);
        let mut cells: Vec<Bars> = PICKS.iter().map(|_| Vec::new()).collect();
        cells[0] = vec![bar("BLAKE3-Servil st", 2.0, ""), bar("SHA-256", 1.0, "")];
        let fresh = row("AMD Ryzen 7 9700X", false, cells);
        let page = html(std::slice::from_ref(&fresh));
        assert!(page.contains(&label(0)));
        assert!(!page.contains(&label(1)), "nothing measured the second row");
        let mut awaited = row(
            "Apple M4 Max",
            false,
            PICKS.iter().map(|_| Vec::new()).collect(),
        );
        awaited.stale = true;
        let page = html(&[awaited]);
        assert!(
            page.contains(&label(0)) && page.contains(&label(1)),
            "every row, awaiting its run"
        );
    }

    /// The first chart draws a machine that has both of its cells, the
    /// headline and the everyday one; a machine with one alone has no row there.
    #[test]
    fn the_first_chart_needs_both_of_its_cells() {
        let both = vec![bar("BLAKE3-Servil mt", 9.0, ""), bar("SHA-256", 1.0, "")];
        for present in [HEADLINE, EVERYDAY] {
            let mut cells: Vec<Bars> = PICKS.iter().map(|_| Vec::new()).collect();
            cells[present] = both.clone();
            assert_eq!(
                families_by_machine(&[row("AMD Ryzen 7 9700X", false, cells)], &[0]),
                ""
            );
        }
        let mut cells: Vec<Bars> = PICKS.iter().map(|_| Vec::new()).collect();
        cells[HEADLINE] = both.clone();
        cells[EVERYDAY] = both;
        assert!(
            families_by_machine(&[row("AMD Ryzen 7 9700X", false, cells)], &[0])
                .contains("fam-row")
        );
    }

    /// A cell whose bars all measure zero draws bars of no width, never a
    /// width that is not a number.
    #[test]
    fn bars_of_zero_speed_have_no_width() {
        let mut cells: Vec<Bars> = PICKS.iter().map(|_| Vec::new()).collect();
        cells[0] = vec![bar("BLAKE3-Servil st", 0.0, ""), bar("SHA-256", 0.0, "")];
        let page = html(&[row("AMD Ryzen 7 9700X", false, cells)]);
        assert!(page.contains("width:0.0%"));
        assert!(!page.contains("width:NaN") && !page.contains("width:inf"));
    }

    /// A generation or a core count is dropped as the pair of words it is:
    /// a number alone, or "Gen" alone, stays.
    #[test]
    fn a_number_or_gen_alone_stays_in_the_name() {
        assert_eq!(short_name("Foo 9th Wonder"), "Foo 9th Wonder");
        assert_eq!(short_name("Foo Bar Gen Baz"), "Foo Bar Gen Baz");
    }

    /// A machine is virtual by any one of its three signs.
    #[test]
    fn a_virtual_machine_is_known_by_any_one_sign() {
        let with = |os: &str, identity: &str| {
            let headers = vec![
                ("os type".to_owned(), os.to_owned()),
                ("cpu identity".to_owned(), identity.to_owned()),
                ("cpu type".to_owned(), "Some CPU".to_owned()),
            ];
            described(&headers, "x".to_owned(), Vec::new(), false).virtual_machine
        };
        assert!(with("linux 7.0 virt", "GenuineIntel"));
        assert!(with("linux 7.0", "a virtual CPU"));
        assert!(with("linux 7.0", "QEMU TCG"));
        assert!(!with("linux 7.0", "GenuineIntel"));
    }

    /// A folder is a machine's results by either of its samples files, and
    /// a folder with one file of this release and one of another reads the
    /// first alone.
    #[test]
    fn a_results_folder_holds_either_samples_file() {
        let dir = std::env::temp_dir().join(format!("bench-overview-dirs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (name, files) in [
            ("a-hashing", &["bench-hashes.samples.tsv"][..]),
            ("b-b3sum", &["b3sum.samples.tsv"][..]),
            ("c-neither", &["notes.txt"][..]),
        ] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
            for file in files {
                std::fs::write(dir.join(name).join(file), "x").unwrap();
            }
        }
        let names: Vec<String> = results_dirs(&dir)
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["a-hashing", "b-b3sum"]);
        // One file of this release, the other of an older one: the row is
        // this release's, read from the first, with the older file left alone.
        let both = dir.join("d-both");
        std::fs::create_dir_all(&both).unwrap();
        std::fs::write(
            both.join("bench-hashes.samples.tsv"),
            format!(
                "{}\n# bench-hashes version: test\n# load: quiet\n# power: mains power\n{}\nblake3-servil-st\tsolo\tOneMessage\t64 B\tB\t80/64,80/64\t1,2\t-\t-\t-\t0,0\n",
                SAMPLES_VERSION,
                crate::SAMPLES_COLUMNS
            ),
        )
        .unwrap();
        std::fs::write(
            both.join("b3sum.samples.tsv"),
            "# an older release's file\nnot this format\n",
        )
        .unwrap();
        let row = super::row(&both, "x".to_owned());
        assert!(!row.stale);
        assert_eq!(
            row.cells[0].len(),
            1,
            "the 64 B cell from this release's file"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Older headers stay available while their measurement rows remain
    /// outside this format's reader, even when those rows cannot be parsed here.
    #[test]
    fn earlier_records_show_headers_without_reading_their_samples() {
        let dir =
            std::env::temp_dir().join(format!("bench-overview-old-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let source = "blake3 1.0.0; crate archive SHA-256 old-fixed-source";
        std::fs::write(
            dir.join("bench-hashes.samples.tsv"),
            format!("# bench-hashes samples v0\n# cpu type: Earlier machine\n# source blake3-official: {source}\nthese are not current-format measurement rows\n"),
        ).unwrap();
        let row = super::row(&dir, "benchmark-results/earlier".to_owned());
        assert!(row.stale);
        assert!(row.cells.iter().all(Vec::is_empty));
        assert_eq!(row.records.len(), 1);
        assert!(row.records[0]
            .details
            .iter()
            .any(|(_, value, _)| value == source));
        let page = html(&[row]);
        assert!(page.contains(source));
        assert!(page.contains("benchmark-results/earlier/bench-hashes.samples.tsv"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A batch cell's speed counts its own messages' length: a batch of 16
    /// messages of 4 KiB, whose label a batch of 16 messages of 64 B shares.
    #[test]
    fn a_batch_cells_speed_is_by_its_own_messages_length() {
        let dir = std::env::temp_dir().join(format!("bench-overview-long-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bench-hashes.samples.tsv");
        // 4,096 ns a message of 4,096 bytes: 1 GB/s.
        std::fs::write(
            &path,
            format!(
                "{}\n# bench-hashes version: test\n# load: quiet\n# power: mains power\n{}\nblake3-servil-st\tsolo\tManyLongMessages\t16\tB\t409600/100,409600/100\t1,2\t-\t-\t-\t0,0\n",
                SAMPLES_VERSION,
                crate::SAMPLES_COLUMNS
            ),
        )
        .unwrap();
        let file = read_samples(path.to_str().unwrap());
        let bars = hashing_bars(
            &file,
            None,
            UseCase::ManyLongMessages,
            "16",
            Algorithm::Blake3ServilSt,
        );
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].gbps, 1.0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A folder with b3sum's samples alone is a row: the hashing file it
    /// lacks is not looked into.
    #[test]
    fn a_folder_of_b3sums_samples_alone_is_a_row() {
        let dir = std::env::temp_dir().join(format!("bench-overview-b3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("b3sum.samples.tsv"),
            format!(
                "{}\n# bench-hashes version: test\n# cpu type: AMD Ryzen 7 9700X 8-Core Processor\n# load: quiet\n# power: mains power\n{}\n",
                SAMPLES_VERSION,
                crate::SAMPLES_COLUMNS
            ),
        )
        .unwrap();
        let row = super::row(&dir, "x".to_owned());
        assert!(!row.stale);
        assert_eq!(row.machine, "AMD Ryzen 7 9700X");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Each run carries its own identities, including file tools and the
    /// earlier library. The overview never substitutes its renderer's build.
    #[test]
    fn provenance_keeps_the_library_file_and_earlier_records_separate() {
        let dir =
            std::env::temp_dir().join(format!("bench-overview-records-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("before")).unwrap();
        let bench = "0.22.0+1111111111111111111111111111111111111111";
        let source = |commit: &str| {
            format!(
                "blake3-servil (local checkout); source https://example.test/library; commit {commit}; clean"
            )
        };
        let library = source("2222222222222222222222222222222222222222");
        let earlier = source("3333333333333333333333333333333333333333");
        let tool = "coreutils 9.9; source https://example.test/tools; flags <fast> & \"exact\"";
        let command =
            "/recorded/tools/b3sum --outboard /recorded/output (blake3 abcdef; b3sums 0.9.0)";
        let fixture = |key: &str, value: &str, day: &str, cells: &str| {
            format!(
                "{SAMPLES_VERSION}\n# timestamp: {day}\n# cpu type: AMD Ryzen 7 9700X\n# cpu count: 16\n# os type: linux\n# bench-hashes version: {bench}\n# bench-hashes source: https://example.test/bench\n# git commit: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n# git clean status: clean\n# source {key}: {value}\n# load: quiet\n# power: mains power\n{}\n{cells}",
                crate::SAMPLES_COLUMNS,
            )
        };
        let cells = "blake3-servil-st\tsolo\tOneMessage\t64 B\tB\t80/64,80/64\t1,2\t-\t-\t-\t0,0\n";
        for (path, key, value, day, cells) in [
            (
                "bench-hashes.samples.tsv",
                "blake3-servil-st",
                library.as_str(),
                "2026-11-14 01:00:00 UTC",
                cells,
            ),
            (
                "b3sum.samples.tsv",
                "sha256sum",
                tool,
                "2026-11-14 02:00:00 UTC",
                "",
            ),
            (
                "before/bench-hashes.samples.tsv",
                "blake3-servil-st",
                earlier.as_str(),
                "2026-11-13 03:00:00 UTC",
                cells,
            ),
        ] {
            let mut text = fixture(key, value, day, cells);
            if path == "b3sum.samples.tsv" {
                text = text.replace(
                    "# load: quiet",
                    &format!("# contender: servil: {command}\n# load: quiet"),
                );
            }
            std::fs::write(dir.join(path), text).unwrap();
        }
        let row = super::row(&dir, "benchmark-results/example".to_owned());
        assert_eq!(row.records.len(), 3);
        assert!(row.records[1]
            .details
            .iter()
            .any(|(label, value, _)| label == "b3sum servil" && value == command));
        for (record, label, source, path) in [
            (
                &row.records[0],
                "Library run",
                library.as_str(),
                "bench-hashes.samples.tsv",
            ),
            (&row.records[1], "File-tool run", tool, "b3sum.samples.tsv"),
            (
                &row.records[2],
                "Earlier library run",
                earlier.as_str(),
                "before/bench-hashes.samples.tsv",
            ),
        ] {
            assert_eq!(record.label, label);
            assert!(record.details.iter().any(|(_, text, _)| text == source));
            assert!(record
                .details
                .iter()
                .any(|(name, text, _)| name == "bench-hashes" && text == bench));
            assert!(record
                .details
                .iter()
                .any(|(name, _, link)| name == "Samples"
                    && link == &format!("benchmark-results/example/{path}")));
        }
        let page = html(&[row]);
        let (results, about) = page
            .split_once("<details class=\"door\" id=\"hashes\">")
            .unwrap();
        assert!(
            !results.contains(bench),
            "the results stay free of the long version line"
        );
        assert!(about.starts_with("<summary>About the hashes compared</summary>"));
        assert!(about.contains(bench));
        assert!(about.contains(
            "https://example.test/library/tree/2222222222222222222222222222222222222222"
        ));
        assert!(about.contains(
            "https://example.test/library/tree/3333333333333333333333333333333333333333"
        ));
        assert!(about.contains("flags &lt;fast&gt; &amp; &quot;exact&quot;"));
        assert!(!about.contains("flags <fast>"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The command writes the page it names from the results it is given.
    #[test]
    fn the_command_writes_the_page() {
        let dir =
            std::env::temp_dir().join(format!("bench-overview-command-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let machine = dir.join("benchmark-results/m");
        std::fs::create_dir_all(&machine).unwrap();
        std::fs::write(
            machine.join("bench-hashes.samples.tsv"),
            format!(
                "{}\n# bench-hashes version: test\n# cpu type: AMD Ryzen 7 9700X 8-Core Processor\n# load: quiet\n# power: mains power\n{}\nblake3-servil-st\tsolo\tOneMessage\t64 B\tB\t80/64,80/64\t1,2\t-\t-\t-\t0,0\n",
                SAMPLES_VERSION,
                crate::SAMPLES_COLUMNS
            ),
        )
        .unwrap();
        let out = dir.join("index.html");
        command(&[
            out.display().to_string(),
            dir.join("benchmark-results").display().to_string(),
        ]);
        let page = std::fs::read_to_string(&out).unwrap();
        assert!(page.contains("AMD Ryzen 7 9700X"), "the machine's name");
        assert!(
            page.contains("benchmark-results/m/bench-hashes.map.html"),
            "its map, by a path from the page"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
