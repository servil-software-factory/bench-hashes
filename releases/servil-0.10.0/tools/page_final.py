#!/usr/bin/env python3
"""The 0.9.1 -> 0.10.0 announcement page for one 0.10.0 commit.

    page_final.py COMMIT OUT_DIR [--mac-check JOB] [--mac-b3sum JOB] [--mac-b3sum-against JOB] [--publish] [--preview]

Front page stays short and scannable (6 representative workloads, plain-English
"faster"/"slower"/"same" cells, zero bare time ratios). Full 10-workload tables,
wall times, 3%-5% bands, known costs, machine specs, and raw evidence live behind
labelled links in b3sum/, library/, known-costs.md, machines.md, and evidence/.
"""
from pathlib import Path
import re, shutil, sys
sys.path.insert(0, str(Path(__file__).parent))
import evolve, page
from page import INBOX, HOSTS, MAC, REPORT, QUICK_REPORT, NEW, field, sections, b3sum_host_page

# The reader that matches the checks' samples: bench-hashes 0.22 at its tag (97c0fbcb).
evolve.X = "/home/agent/bench-022-plain/target/release/bench-hashes"
BASE_REPORT = "b3sum-base-0100.stdout.txt"
MARGIN = 1.05  # Zooko, November 18, 2026: a cell counts beyond 5%.
NAMES = {
    "x86-epyc-9275f": "EPYC 9275F",
    "x86-epyc-9255": "EPYC 9255",
    "x86-9700x": "Ryzen 9700X",
    "x86-icelake": "2× Xeon Gold 6330 (a)",
    "x86-icelake-2": "2× Xeon Gold 6330 (b)",
    "x86-avx2": "2× EPYC 7443",
    MAC: "Apple M6",
}

# Six representative workloads for the front page (all 10 live in b3sum/).
HIGHLIGHT_INPUTS = [
    ("4 KiB", "1 small file (4 KiB)"),
    ("1 MiB", "1 medium file (1 MiB)"),
    ("1 GiB", "1 large file (1 GiB)"),
    ("1000 x 16 KiB", "1,000 small files (16 KiB each)"),
    ("mixed tree, 1000 files, 74 MiB", "Mixed directory (1,000 files, 74 MiB)"),
    ("16 x 64 MiB", "16 large files (64 MiB each)"),
]


def flag(args, name):
    return args[args.index(name) + 1] if name in args else None


COL_LABELS = {
    "official": "Official b3sum 1.8.7",
    "official-1-thread": "Official b3sum (1 thread)",
    "b3sum": "BLAKE3-Servil 0.10.0",
    "b3sum-1-thread": "BLAKE3-Servil 0.10.0 (1 thread)",
    "b3sum-0.10.0": "BLAKE3-Servil 0.10.0",
    "b3sum-0.9.1": "BLAKE3-Servil 0.9.1",
    "b3sum-0.9.0": "BLAKE3-Servil 0.9.0",
    "b3sum-0.8.0": "BLAKE3-Servil 0.8.0",
}


def fmt_host_cell(cell, is_base=False):
    """Mean wall time plus unmistakable faster/slower/same tag against the base column."""
    m = re.match(r"\s*([0-9]+(?:\.[0-9]+)?\s*(?:ns|μs|us|ms|s))\b", cell)
    time_s = m.group(1) if m else cell.strip()
    if is_base or " x" not in cell:
        return time_s
    r_str, label = page.ratio(cell)
    r = float(r_str)
    if label == "faster":
        sp = 1.0 / r
        tag = f"**{fmt_x(sp)} faster**" if sp >= 1.5 else f"**{round((1 - r) * 100)}% faster**"
        return f"{time_s} ({tag})"
    if label == "slower":
        tag = f"{fmt_x(r)} slower" if r >= 1.5 else f"{round((r - 1) * 100)}% slower"
        return f"{time_s} ({tag})"
    return f"{time_s} (same)"


def render_b3sum_host(host, text, heading, columns=None):
    out = [
        f"# {heading}",
        "",
        f"Host `{host}` ({field(text, 'machine')}); files on {field(text, 'files on')}. "
        f"Load: {field(text, 'load')}. Runs: {field(text, 'runs')}.",
        "",
    ]
    for title, names, rows in sections(text):
        keep = [n for n in (columns or names) if n in names]
        base = names[0]
        base_label = COL_LABELS.get(base, base)
        short_title = "In memory (page cache)" if "warm" in title.lower() else "From disk (evicted before each run)"
        out += [
            f"## {short_title}",
            "",
            f"*Wall time per run; parenthesized comparison is against {base_label}.*",
            "",
            "| Workload | " + " | ".join(COL_LABELS.get(n, n) for n in keep) + " |",
            "|---|" + "---:|" * len(keep),
        ]
        for inp, cells in rows.items():
            rendered = [fmt_host_cell(cells[names.index(n)], is_base=(n == base)) for n in keep]
            out.append(f"| {inp} | " + " | ".join(rendered) + " |")
        out.append("")
    return "\n".join(out)


def b3sum_run(host, short):
    for run in (f"b3sum-page-{short}", f"b3sum-0100-{short}"):
        d = INBOX / host / run
        if (d / REPORT).exists() and "_EXIT=0" in (d / "stdout.txt").read_text():
            return d
    return None


def parse_time_us(cell):
    """Extract the mean wall time in microseconds from a b3sum table cell."""
    m = re.match(r"\s*([0-9]+(?:\.[0-9]+)?)\s*(ns|μs|us|ms|s)\b", cell)
    if not m:
        raise ValueError(f"no time in cell: {cell!r}")
    val, unit = float(m.group(1)), m.group(2)
    scale = {"ns": 1e-3, "μs": 1.0, "us": 1.0, "ms": 1e3, "s": 1e6}[unit]
    return val * scale


def fmt_x(speedup):
    """Format a >=1 speedup factor: 1.2×, 7.4×, 10×, 27×."""
    return f"{speedup:.0f}×" if speedup >= 9.95 else f"{speedup:.1f}×"


def fmt_official_one(ratio_new_over_off, label):
    """Single-machine (Apple M6) cell: 0.10.0 vs official b3sum 1.8.7."""
    if label == "faster":
        speedup = 1.0 / ratio_new_over_off
        if speedup >= 1.15:
            return f"**{fmt_x(speedup)} faster**"
        pct = round((1.0 - ratio_new_over_off) * 100)
        return f"**{pct}% faster**"
    if label == "slower":
        pct = round((ratio_new_over_off - 1.0) * 100)
        return f"{pct}% slower"
    return "same"


def fmt_official_fleet(speedups, faster_count, slower_count, n):
    """Multi-machine Linux cell: 0.10.0 vs official b3sum 1.8.7 (speedup = official / 0.10.0)."""
    lo, hi = min(speedups), max(speedups)
    if faster_count == n and slower_count == 0:
        if fmt_x(lo) == fmt_x(hi):
            return f"**{fmt_x(lo)} faster**"
        return f"**{fmt_x(lo)}–{fmt_x(hi)} faster**"
    if faster_count > 0 and slower_count == 0:
        return f"{fmt_x(lo)}–{fmt_x(hi)} faster ({faster_count} of {n})"
    if slower_count > 0 and faster_count == 0:
        return f"slower ({slower_count} of {n})"
    if faster_count == 0 and slower_count == 0:
        return "same"
    return f"{faster_count} faster, {slower_count} slower (of {n})"


def fmt_prev_one(ratio_new_over_old, label):
    """Single-machine (Apple M6) cell: 0.9.x -> 0.10.0."""
    if label == "faster":
        sp = 1.0 / ratio_new_over_old
        if sp >= 1.5:
            return f"**{fmt_x(sp)} faster**"
        return f"**{round((1.0 - ratio_new_over_old) * 100)}% faster**"
    if label == "slower":
        if ratio_new_over_old >= 1.5:
            return f"{fmt_x(ratio_new_over_old)} slower"
        return f"{round((ratio_new_over_old - 1.0) * 100)}% slower"
    return "same"


def fmt_prev_fleet(pairs):
    """Multi-machine Linux cell: 0.10.0 vs 0.9.x (ratio = 0.10.0 / 0.9.x, <1 is faster)."""
    n = len(pairs)
    faster = [(1.0 - r) * 100.0 for r, l in pairs if l == "faster"]
    slower = [(r - 1.0) * 100.0 for r, l in pairs if l == "slower"]
    if len(faster) == n and not slower:
        lo, hi = round(min(faster)), round(max(faster))
        return f"**{lo}% faster** (all {n})" if lo == hi else f"**{lo}%–{hi}% faster** (all {n})"
    if faster and not slower:
        lo, hi = round(min(faster)), round(max(faster))
        if len(faster) == 1:
            return f"{hi}% faster (1 of {n})"
        return f"up to {hi}% faster ({len(faster)} of {n})" if lo < 5 else f"{lo}%–{hi}% faster ({len(faster)} of {n})"
    if slower and not faster:
        lo, hi = round(min(slower)), round(max(slower))
        if len(slower) == 1:
            return f"{hi}% slower (1 of {n})"
        return f"{lo}% slower ({len(slower)} of {n})" if lo == hi else f"{lo}%–{hi}% slower ({len(slower)} of {n})"
    if faster and slower:
        f_hi = round(max(faster))
        s_hi = round(max(slower))
        return f"up to {f_hi}% faster ({len(faster)} of {n}); {s_hi}% slower on {len(slower)}"
    return "same"


def extract_official_vs_new(full_reports, official_reports):
    """Per-input, per-state (0=warm, 1=cold) list of (speedup = official/0.10, is_faster, is_slower) across hosts.
    Uses direct `b3sum-base-0100.stdout.txt` where available; falls back to `b3sum-full.stdout.txt`
    ratios when rendering a preview before all hosts have the third suite."""
    hosts = list(full_reports)
    used_direct = len(official_reports) == len(hosts)
    data = {0: {}, 1: {}}
    first = sections(full_reports[hosts[0]])
    for idx in (0, 1):
        for inp in first[idx][2]:
            entries = []
            for h in hosts:
                if used_direct:
                    _t, names, rows = sections(official_reports[h])[idx]
                    r_str, label = page.ratio(rows[inp][names.index("official")])
                    # Base is 0.10.0, judged column is official: label=="slower" means official took longer => 0.10.0 is faster.
                    entries.append((float(r_str), label == "slower", label == "faster"))
                else:
                    _t, names, rows = sections(full_reports[h])[idx]
                    t_new = parse_time_us(rows[inp][names.index(NEW)])
                    t_off = parse_time_us(rows[inp][names.index("official")])
                    speedup = t_off / t_new
                    # Conservative directional tag for preview when direct base-0100 suite wasn't run:
                    entries.append((speedup, speedup >= 1.05, speedup <= 1.0 / 1.05))
            data[idx][inp] = entries
    return data, used_direct


def library(host, folder, commit):
    """(host_md, {form: [cells, faster, slower, slower_beyond_5pct, best_gain_text]}, slower_rows)."""
    sides, rows, _pairs = evolve.compare(folder)
    assert any(c[:12] in s for s in sides for c in (commit, CHECKS_COMMIT or commit)), (sides, commit)
    forms = {}
    for r in rows:
        form = evolve.NAMES.get(r["key"].split("|", 1)[0], r["key"].split("|", 1)[0])
        c = forms.setdefault(form, [0, 0, 0, 0])
        label, ratio = r["label"].lower(), float(r["ratio"])
        c[0] += 1
        c[1] += label == "faster"
        c[2] += label == "slower"
        c[3] += label == "slower" and ratio > MARGIN
    for form, c in forms.items():
        best = min(
            (r for r in rows if r["label"].lower() == "faster" and evolve.NAMES.get(r["key"].split("|", 1)[0]) == form),
            key=lambda r: float(r["ratio"]),
            default=None,
        )
        if best:
            sp = 1.0 / float(best["ratio"])
            c.append(f"{' · '.join(best['key'].split('|')[1:])} ({fmt_x(sp)} faster)")
        else:
            c.append("")
    _sides, head, body = evolve.table(folder)
    slower = sorted((r for r in rows if r["label"].lower() == "slower"), key=lambda r: -float(r["ratio"]))
    faster = sorted((r for r in rows if r["label"].lower() == "faster"), key=lambda r: float(r["ratio"]))[:10]
    out = [
        f"# Library cells, 0.9.1 → 0.10.0: {NAMES[host]} (`{host}`)",
        "",
        "One bench-hashes (0.22) built both libraries and measured them in alternating pairs. "
        "Verdicts come from `bench-hashes compare` over the check's kept runs (3% bench mark; 5% project margin).",
        "",
        "`" + "`; `".join(s.removeprefix("compare: ") for s in sides) + "`",
        "",
        head,
        "",
        body,
        "",
    ]
    if slower:
        out += ["Slower cells (beyond 5% in bold):", ""] + [
            f"- `{r['key']}`: "
            + (f"**+{round((float(r['ratio']) - 1) * 100)}% slower**" if float(r["ratio"]) > MARGIN else f"+{round((float(r['ratio']) - 1) * 100)}% slower")
            + f" ({r['pairs']} pairs)"
            for r in slower
        ] + [""]
    else:
        out += ["No cell reads slower.", ""]
    if faster:
        out += ["Ten largest gains:", ""] + [
            f"- `{r['key']}`: {fmt_x(1.0 / float(r['ratio']))} faster" for r in faster
        ] + [""]
    return "\n".join(out), forms, slower


def main(commit, out_dir, mac_check, mac_b3sum, mac_b3sum_against=None, preview=False):
    short, out = commit[:8], Path(out_dir)
    for d in ("b3sum", "library", "tables"):
        (out / d).mkdir(parents=True, exist_ok=True)
    parts = {"COMMIT": commit, "SHORT": short}

    # --- b3sum ---
    runs = {h: b3sum_run(h, short) for h in HOSTS}
    runs = {h: d for h, d in runs.items() if d}
    full = {h: (d / REPORT).read_text() for h, d in runs.items()}
    quick = {h: (d / QUICK_REPORT).read_text() for h, d in runs.items() if (d / QUICK_REPORT).exists()}
    official = {h: (d / BASE_REPORT).read_text() for h, d in runs.items() if (d / BASE_REPORT).exists()}
    base_label = "0.9.1" if any("b3sum-0.9.1" in t for t in full.values()) else "0.9.0"
    parts["BASE"] = base_label

    # Per-host b3sum detail pages (behind the b3sum/ door)
    for h, text in full.items():
        md = render_b3sum_host(h, text, f"b3sum on {NAMES[h]} (all 10 workloads)")
        if h in quick:
            md += "\n---\n\n" + render_b3sum_host(h, quick[h], f"Quick suite on {NAMES[h]} (5 rounds, includes 100 x 16 KiB)")
        if h in official:
            md += "\n---\n\n" + render_b3sum_host(h, official[h], f"Official b3sum 1.8.7 vs BLAKE3-Servil 0.10.0 on {NAMES[h]}")
        (out / "b3sum" / f"{h}.md").write_text(md)

    mac_data = {0: {}, 1: {}}
    mac_wins = 0
    mac_total = 0
    mac_slower = []
    if mac_b3sum:
        found = sorted(page.MAC_RESULTS.glob(f"{mac_b3sum}-*-mac-m6-timing"))
        mac_txt = (found[-1] / "benchmark-results/AppleM6.darwin27/b3sum.result.txt").read_text()
        mac_cols = ["official", "b3sum", "official-1-thread", "b3sum-1-thread", "sha256sum", "sha1sum", "md5sum"]
        (out / "b3sum" / f"{MAC}.md").write_text(render_b3sum_host(MAC, mac_txt, f"b3sum on Apple M6 (job {mac_b3sum})", columns=mac_cols))
        for idx, state in ((0, "in memory"), (1, "from disk")):
            _t, names, rows = sections(mac_txt)[idx]
            for inp, cells in rows.items():
                r_str, lbl = page.ratio(cells[names.index("b3sum")])
                mac_data[idx][inp] = (float(r_str), lbl)
                mac_total += 1
                if lbl == "faster":
                    mac_wins += 1
                elif lbl == "slower":
                    mac_slower.append(f"{inp} {state}, {round((float(r_str) - 1) * 100)}% slower")

    # Front-page Table 1: 0.10.0 vs Official b3sum 1.8.7 (6 highlight rows)
    off_fleet, _used_direct = extract_official_vs_new(full, official)
    all_linux_speedups = [sp for idx in (0, 1) for entries in off_fleet[idx].values() for sp, _, _ in entries]
    linux_all_faster = all(f and not s for idx in (0, 1) for entries in off_fleet[idx].values() for _, f, s in entries)

    t1 = [
        "| Workload | Linux in memory (6 hosts) | Linux from disk (6 hosts) | Apple M6 in memory | Apple M6 from disk |",
        "|---|---:|---:|---:|---:|",
    ]
    for key, label in HIGHLIGHT_INPUTS:
        l_warm = off_fleet[0][key]
        l_cold = off_fleet[1][key]
        lw_cell = fmt_official_fleet([x[0] for x in l_warm], sum(x[1] for x in l_warm), sum(x[2] for x in l_warm), len(l_warm))
        lc_cell = fmt_official_fleet([x[0] for x in l_cold], sum(x[1] for x in l_cold), sum(x[2] for x in l_cold), len(l_cold))
        mw_cell = fmt_official_one(*mac_data[0][key]) if key in mac_data[0] else "—"
        mc_cell = fmt_official_one(*mac_data[1][key]) if key in mac_data[1] else "—"
        t1.append(f"| {label} | {lw_cell} | {lc_cell} | {mw_cell} | {mc_cell} |")
    parts["TABLE-OFFICIAL"] = "\n".join(t1)

    mac_prev_data = {0: {}, 1: {}}
    if mac_b3sum_against:
        found = sorted(page.MAC_RESULTS.glob(f"{mac_b3sum_against}-*-mac-m6-timing"))
        mac_prev_txt = (found[-1] / "benchmark-results/AppleM6.darwin27/b3sum.result.txt").read_text()
        (out / "b3sum" / f"{MAC}-vs-{base_label}.md").write_text(
            render_b3sum_host(MAC, mac_prev_txt, f"b3sum {base_label} → 0.10.0 on Apple M6 (job {mac_b3sum_against})")
        )
        for idx in (0, 1):
            _t, names, rows = sections(mac_prev_txt)[idx]
            new_col = next((n for n in names if n.startswith("b3sum-") and commit.startswith(n.removeprefix("b3sum-"))), None)
            if new_col is None and "b3sum-0.10.0" in names:
                new_col = "b3sum-0.10.0"
            assert new_col is not None, f"no 0.10.0 column in Mac against job {mac_b3sum_against}: {names}"
            for inp, cells in rows.items():
                r_str, lbl = page.ratio(cells[names.index(new_col)])
                mac_prev_data[idx][inp] = (float(r_str), lbl)

    # Front-page Table 2: 0.9.x -> 0.10.0 (Linux + Apple M6 when available)
    has_mac_prev = bool(mac_prev_data[0])
    if has_mac_prev:
        t2 = [
            "| Workload | Linux in memory (6 hosts) | Linux from disk (6 hosts) | Apple M6 in memory | Apple M6 from disk |",
            "|---|---|---|---:|---:|",
        ]
    else:
        t2 = [
            "| Workload | Linux in memory (6 hosts) | Linux from disk (6 hosts) |",
            "|---|---|---|",
        ]
    hosts = list(full)
    for key, label in HIGHLIGHT_INPUTS:
        w_pairs = []
        c_pairs = []
        for h in hosts:
            _, names_w, rows_w = sections(full[h])[0]
            _, names_c, rows_c = sections(full[h])[1]
            rw, lw = page.ratio(rows_w[key][names_w.index(NEW)])
            rc, lc = page.ratio(rows_c[key][names_c.index(NEW)])
            w_pairs.append((float(rw), lw))
            c_pairs.append((float(rc), lc))
        if has_mac_prev:
            mw = fmt_prev_one(*mac_prev_data[0][key]) if key in mac_prev_data[0] else "—"
            mc = fmt_prev_one(*mac_prev_data[1][key]) if key in mac_prev_data[1] else "—"
            t2.append(f"| {label} | {fmt_prev_fleet(w_pairs)} | {fmt_prev_fleet(c_pairs)} | {mw} | {mc} |")
        else:
            t2.append(f"| {label} | {fmt_prev_fleet(w_pairs)} | {fmt_prev_fleet(c_pairs)} |")
    parts["TABLE-PREV"] = "\n".join(t2)

    # Compute headline cold large-file and 1000-file numbers dynamically from data
    cold_1g = [float(page.ratio(sections(full[h])[1][2]["1 GiB"][sections(full[h])[1][1].index(NEW)])[0]) for h in hosts]
    warm_1k = [float(page.ratio(sections(full[h])[0][2]["1000 x 16 KiB"][sections(full[h])[0][1].index(NEW)])[0]) for h in hosts]
    cold_lo, cold_hi = round((1 - max(cold_1g)) * 100), round((1 - min(cold_1g)) * 100)
    warm_lo, warm_hi = round((1 - max(warm_1k)) * 100), round((1 - min(warm_1k)) * 100)

    headline = []
    if preview:
        headline.append(f"> **Layout preview** using pre-release commit `{short}` (final `0.10.0` numbers will replace these once the tag is cut).\n")
    linux_line = (
        f"**`b3sum` from BLAKE3-Servil 0.10.0 beats official `b3sum` 1.8.7 on every workload tested on Linux** "
        f"(from **{fmt_x(min(all_linux_speedups))}** to **{fmt_x(max(all_linux_speedups))} faster** across six x86-64 machines)"
        if linux_all_faster
        else f"**`b3sum` from BLAKE3-Servil 0.10.0 reaches up to {fmt_x(max(all_linux_speedups))} faster than official `b3sum` 1.8.7 on Linux**"
    )
    if mac_total:
        slower_note = f", with {len(mac_slower)} slower ({'; '.join(mac_slower)})" if mac_slower else ""
        linux_line += f", and wins **{mac_wins} of {mac_total}** workloads on an Apple M6{slower_note}."
    else:
        linux_line += "."
    headline.append(linux_line)
    headline.append("")
    headline.append(
        f"Compared with BLAKE3-Servil {base_label} on Linux, `b3sum` 0.10.0 hashes a cold 1 GiB file "
        f"**{cold_lo}%–{cold_hi}% faster** and hashes 1,000 in-memory 16 KiB files **{warm_lo}%–{warm_hi}% faster**."
    )
    parts["HEADLINE"] = "\n".join(headline)

    # --- Library summary (concise on front page; full tables in library/) ---
    checks = {h: INBOX / h / f"release-0100-onebench-{(CHECKS_COMMIT or commit)[:8]}" / "kept-release" for h in HOSTS}
    if mac_check:
        checks[MAC] = page.mac_job(mac_check)
    lib_faster_counts = []
    beyond_items = []
    machine_count = 0
    for h, folder in checks.items():
        if not folder.is_dir():
            continue
        machine_count += 1
        text, forms, slower = library(h, folder, commit)
        again = [(k, v) for (host, k), v in REPEATED.items() if host == h]
        if again:
            text += "\n\nRepeated by name with 16 or more link layouts a side, after the check above marked them from four pairs:\n\n" + "\n".join(f"- `{k}`: {v}" for k, v in again) + "\n"
        (out / "library" / f"{h}.md").write_text(text)
        lib_faster_counts.append(sum(c[1] for c in forms.values()))
        far = [r for r in slower if float(r["ratio"]) > MARGIN and (h, r["key"]) not in REPEATED]
        for r in far:
            form_name = evolve.NAMES.get(r["key"].split("|", 1)[0], r["key"].split("|", 1)[0])
            cell_name = " · ".join(r["key"].split("|")[1:])
            pct = round((float(r["ratio"]) - 1.0) * 100)
            beyond_items.append(f"**{NAMES[h]}** — `{cell_name}` ({form_name}): {pct}% slower")

    if lib_faster_counts:
        f_lo, f_hi = min(lib_faster_counts), max(lib_faster_counts)
        hosts_with_slow = sorted({item.split(" — ")[0] for item in beyond_items})
        lib_lines = [
            f"Across {machine_count} machines ({machine_count - (1 if mac_check else 0)} x86-64 Linux" + (", one Apple M6" if mac_check else "") + "; one benchmark binary on both sides, 5% margin), "
            f"BLAKE3-Servil 0.10.0 improved on **{f_lo}–{f_hi} workloads per machine** relative to {base_label}."
        ]
        if beyond_items:
            pcts = [int(re.search(r"(\d+)% slower", item).group(1)) for item in beyond_items]
            clean_hosts = machine_count - len(hosts_with_slow)
            lib_lines += [
                "",
                f"- **Known cost (> 5%):** {len(beyond_items)} multithreaded workloads were **{min(pcts)}%–{max(pcts)}% slower** "
                f"across {len(hosts_with_slow)} machines ({', '.join(hosts_with_slow)}), while {clean_hosts} had none beyond 5%. "
                f"See every cell by name in [library/](library/) and the tracked causes in [known-costs.md](known-costs.md).",
            ]
        else:
            lib_lines += ["", "**No workload was slower beyond 5% on any machine.**"]
        if CHECKS_COMMIT and CHECKS_COMMIT != commit:
            lib_lines += ["", f"*The x86-64 library checks ran on `{CHECKS_COMMIT[:8]}`, the candidate stamped earlier the same day; the release commit is `{short}`. The two differ in documents, release tools and one test, and in no shipped code.*"]
        parts["LIBRARY-SUMMARY"] = "\n".join(lib_lines)
    else:
        parts["LIBRARY-SUMMARY"] = "*(Library checks pending.)*"

    parts["B3SUM-PAGES"] = ", ".join(
        f"[{NAMES.get(p.stem, p.stem)}](b3sum/{p.stem}.md)" for p in sorted((out / "b3sum").glob("*.md"))
    )
    parts["LIBRARY-PAGES"] = ", ".join(f"[{NAMES[p.stem]}](library/{p.stem}.md)" for p in sorted((out / "library").glob("*.md")))

    # Write known-costs.md and machines.md behind their own doors
    costs_raw = (Path(__file__).parent / "costs-final.md").read_text()
    costs_clean = "\n".join(ln for ln in costs_raw.splitlines() if not (ln.startswith("@@") and ln.endswith("@@"))).strip()
    costs_clean = costs_clean.replace("@@COMMIT@@", commit).replace("@@BASE@@", base_label)
    (out / "known-costs.md").write_text(f"# Known costs and owners (0.10.0)\n\n{costs_clean}\n")

    hosts_md = [
        f"# Machines, storage, and loads (`{commit}`)",
        "",
        "Two-socket Linux machines ran bound to memory node 0 (`numactl --cpunodebind=0 --membind=0`).",
        "",
    ]
    for h, t in full.items():
        hosts_md.append(f"- **{NAMES[h]}** (`{h}`): {field(t, 'machine')}; files on {field(t, 'files on')}; {field(t, 'load')}; {field(t, 'runs')}")
    (out / "machines.md").write_text("\n".join(hosts_md) + "\n")

    # Render README.md
    text = (Path(__file__).parent / "README.final.in.md").read_text()
    for _ in range(2):
        for name, value in parts.items():
            text = text.replace(f"@@{name}@@", value)
    assert "@@" not in text, [l for l in text.splitlines() if "@@" in l]
    (out / "README.md").write_text(text)

    # Also build b3sum/README.md and library/README.md index pages so directory links read cleanly on GitHub
    (out / "b3sum" / "README.md").write_text(
        "# Per-machine `b3sum` results\n\n"
        + "\n".join(f"- [{NAMES.get(p.stem, p.stem)}]({p.name})" for p in sorted((out / "b3sum").glob("*.md")) if p.name != "README.md")
        + "\n"
    )
    lib_index = ["# Per-machine library results", ""]
    lib_index += [f"- [{NAMES[p.stem]}]({p.name})" for p in sorted((out / "library").glob("*.md")) if p.name != "README.md"]
    if beyond_items:
        lib_index += ["", "## Workloads slower beyond 5%", ""] + [f"- {item}" for item in beyond_items]
    (out / "library" / "README.md").write_text("\n".join(lib_index) + "\n")
    return runs, checks


def evidence(out_dir, runs, checks, mac_check, mac_b3sum, mac_b3sum_against=None):
    out, here = Path(out_dir), Path(__file__).parent

    def copy(src, dst):
        for p in src.rglob("*"):
            if p.is_file() and p.suffix not in {".html", ".svg"}:
                q = dst / p.relative_to(src)
                q.parent.mkdir(parents=True, exist_ok=True)
                data = p.read_bytes()
                q.write_bytes(data if p.suffix == ".tsv" else data.replace(b"/home/devon", b"<worker-home>").replace(b"/Users/benchrunner", b"<runner-home>"))

    for h, d in runs.items():
        copy(d, out / "evidence" / h / d.name)
    for h, d in checks.items():
        if d.is_dir() and h != MAC:
            copy(d.parent, out / "evidence" / h / d.parent.name)
    for job in (mac_check, mac_b3sum, mac_b3sum_against):
        if job:
            found = sorted(page.MAC_RESULTS.glob(f"{job}-*-mac-m6-timing"))
            copy(found[-1], out / "evidence" / MAC / found[-1].name.split(".")[0])
    (out / "tools").mkdir(exist_ok=True)
    for name in ("b3sum-page.sh", "onebench-x86-final.sh", "evolve.py", "page.py", "page_final.py", "README.final.in.md", "costs-final.md"):
        shutil.copyfile(here / name, out / "tools" / name)


# Cells the first check marked from four pairs and a repeat by name (16 layouts a side) then read: what the repeat found.
# The repeat's reading is the one the page counts (BLAKE3 #433, November 20; run `repeat-cells-d3ed18d1` on each machine).
REPEATED = {
    ("x86-avx2", "blake3-servil-mt|shared-one-cache|MerkleTree|1048576 x 16 B"): "3.6% slower over 16 pairs (7.1% from four)",
    ("x86-icelake", "blake3-servil-mt|solo|IdleOneMessage|32 KiB"): "level over 16 pairs (5.4% slower from four)",
    ("x86-icelake-2", "blake3-servil-st|solo|ManyMessages|3"): "level over 16 pairs (6.0% slower from four)",
    ("x86-icelake-2", "blake3-servil-mt|solo|ManyMessages|4"): "level over 64 pairs (5.0% slower from four)",
    ("apple-m6", "blake3-servil-mt|solo|ManyMessages|1"): "level over 64 pairs (6.6% slower from four; Mac job 3545)",
}

CHECKS_COMMIT = None  # the x86 library checks' commit, where it is another than the page's (--checks-commit)

if __name__ == "__main__":
    args = sys.argv[1:]
    CHECKS_COMMIT = flag(args, "--checks-commit")
    mac_check = flag(args, "--mac-check")
    mac_b3sum = flag(args, "--mac-b3sum")
    mac_b3sum_against = flag(args, "--mac-b3sum-against")
    preview = "--preview" in args
    runs, checks = main(args[0], args[1], mac_check, mac_b3sum, mac_b3sum_against=mac_b3sum_against, preview=preview)
    if "--publish" in args:
        evidence(args[1], runs, checks, mac_check, mac_b3sum, mac_b3sum_against=mac_b3sum_against)
