#!/usr/bin/env python3
"""Tables for the 0.8 -> 0.9 -> 0.10 page, from kept check runs.

Every ratio and verdict is `bench-hashes compare`'s own line (the Rust reader and
rule); this script groups those lines by use case and formats them. It reads no
sample itself.
"""
from pathlib import Path
import re, statistics, subprocess, sys

X = "/home/agent/bench-pin-move/target/release/bench-hashes"
LINE = re.compile(r"^(?P<key>[^:]+\|[^:]+): (?P<old>[\d.]+) -> (?P<new>[\d.]+) ns/unit\s+x(?P<ratio>[\d.]+) (?P<label>\w+)")
NAMES = {"blake3-servil-st": "one thread", "blake3-servil-mt": "every core"}

def one_stage(folder):
    olds = sorted(map(str, Path(folder).glob("run-*-old.tsv")))
    news = sorted(map(str, Path(folder).glob("run-*-new.tsv")))
    assert olds and len(olds) == len(news), folder
    out = subprocess.run([X, "compare", *olds, "--", *news], capture_output=True, text=True, check=True).stdout
    sides = [l for l in out.splitlines() if l.startswith("compare:")]
    rows = [m.groupdict() for m in map(LINE.match, out.splitlines()) if m]
    assert rows, folder
    return sides, rows, len(olds)

def compare(folder):
    """Each cell's reading: the check's first look over every cell, and for the cells
    it measured again (`second/`, more pairs over more link layouts) the second
    look's. The second look measures the check's suspects and the other cells at
    their points; the check's verdict judges the suspects alone. So a cell beside a
    suspect can read slower here where the check said "no cell slower" (EPYC 7443
    pair, release-0100-follow: two such cells over 64 pairs)."""
    sides, rows, pairs = one_stage(folder)
    second = Path(folder) / "second"
    if second.is_dir():
        sides2, rows2, pairs2 = one_stage(second)
        assert sides2 == sides, (sides, sides2)
        again = {r["key"]: r for r in rows2}
        assert set(again) <= {r["key"] for r in rows}
        rows = [dict(again.get(r["key"], r), pairs=pairs2 if r["key"] in again else pairs) for r in rows]
    else:
        rows = [dict(r, pairs=pairs) for r in rows]
    return sides, rows, pairs

def table(folder, skip=frozenset()):
    """The folder's cells by use case; `skip` names cells left out (the caller reports them)."""
    sides, rows, pairs = compare(folder)
    rows = [r for r in rows if r["key"] not in skip]
    groups = {}
    for r in rows:
        contender, scenario, use_case, _point = r["key"].split("|", 3)
        groups.setdefault((contender, scenario, use_case), []).append(r)
    lines = ["| form | scenario | use case | cells | faster | slower | median time ratio | range |", "|---|---|---|---:|---:|---:|---:|---|"]
    total = [0, 0, 0]
    for (contender, scenario, use_case), rs in groups.items():
        ratios = [float(r["ratio"]) for r in rs]
        faster = sum(r["label"].lower() == "faster" for r in rs)
        slower = sum(r["label"].lower() == "slower" for r in rs)
        total[0] += len(rs); total[1] += faster; total[2] += slower
        lines.append(f"| {NAMES.get(contender, contender)} | {scenario} | {use_case} | {len(rs)} | {faster} | {slower} | ×{statistics.median(ratios):.3f} | ×{min(ratios):.3f}–×{max(ratios):.3f} |")
    again = sum(r["pairs"] != pairs for r in rows)
    head = f"{total[0]} cells over {pairs} pairs" + (f" ({again} of them measured again over more pairs and link layouts)" if again else "") + f": **{total[1]} faster, {total[2]} slower**, the rest level."
    return sides, head, "\n".join(lines)

if __name__ == "__main__":
    for folder in sys.argv[1:]:
        sides, head, body = table(folder)
        print("\n".join(sides)); print(head); print(body); print()
