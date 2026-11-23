#!/usr/bin/env python3
"""The dependency policy's checks, as CI runs them (BLAKE3 #479).

    python3 tools/dependency_policy.py

1. Every package without a source in `cargo metadata` must be one of the two crates this repository
   holds (`OURS`), matching both its name and its manifest path (`strangers`). cargo-deny checks
   registry and Git sources, and cargo-vet asks an audit of crates.io and Git packages; a package
   taken by path is first-party to both, so a vendored library would pass them unaudited.
2. `cargo deny --locked check -D index-failure`: advisories (including transitive unsound and
   unmaintained, yanked versions, and unreadable yank statuses), bans, licences (including dev
   dependencies), and sources.
3. `cargo vet --locked`: every third-party version has an imported audit or a recorded exemption.
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# The two crates this repository holds, by package name and folder inside ROOT.
OURS = {
    "bench-hashes": ".",
    "clocks": "clocks",
}


def strangers(metadata, root=ROOT, ours=OURS):
    """What is wrong with the packages that have no source, in `cargo metadata`'s account:
    Cargo's `workspace_members` must resolve to `ours`, and every source-less package must match
    one of `ours` by both name and manifest path."""
    root = Path(root).resolve()
    expected = {name: (root / folder / "Cargo.toml").resolve() for name, folder in ours.items()}
    seen = set()
    wrong = []
    by_id = {p["id"]: p for p in metadata["packages"]}
    actual_members = {by_id[mid]["name"] if mid in by_id else mid for mid in metadata.get("workspace_members", [])}
    if actual_members != set(ours):
        wrong.append(f"Cargo workspace_members are {sorted(actual_members)}; expected {sorted(ours)}")
    for package in sorted((p for p in metadata["packages"] if p["source"] is None), key=lambda p: (p["name"], p["manifest_path"])):
        name = package["name"]
        path = Path(package["manifest_path"]).resolve()
        if expected.get(name) == path:
            seen.add(name)
            continue
        try:
            where = f"{path.parent.relative_to(root)}/"
        except ValueError:
            where = str(path.parent)
        wrong.append(f"{name} {package['version']} is taken by path, from {where}, and matches neither crate of this "
                     "repository by name and path: a dependency comes from crates.io or an allowed Git source")
    for name in sorted(set(ours) - seen):
        wrong.append(f"expected crate {name} at {ours[name]}/Cargo.toml is absent from the graph")
    return wrong


def main():
    metadata = subprocess.run(["cargo", "metadata", "--locked", "--all-features", "--format-version", "1"],
                              cwd=ROOT, check=True, stdout=subprocess.PIPE, text=True)
    wrong = strangers(json.loads(metadata.stdout))
    for line in wrong:
        print(f"dependency policy: {line}", file=sys.stderr)
    deny = subprocess.run(["cargo", "deny", "--color", "never", "--locked", "check", "-D", "index-failure"], cwd=ROOT)
    vet = subprocess.run(["cargo", "vet", "--locked"], cwd=ROOT)
    return 1 if wrong or deny.returncode or vet.returncode else 0


if __name__ == "__main__":
    sys.exit(main())
