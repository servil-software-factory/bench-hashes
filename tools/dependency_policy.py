#!/usr/bin/env python3
"""The dependency policy's checks, as CI runs them (BLAKE3 #479).

    python3 tools/dependency_policy.py            the checks
    python3 tools/dependency_policy.py controls   planted faults, each of which the checks must fail

1. Every package without a source in `cargo metadata` must be one of the two crates this repository
   holds (`OURS`), matching both its name and its manifest path, and Cargo's `workspace_members`
   must resolve to `OURS` (`strangers`). cargo-deny checks registry and Git sources, and cargo-vet
   asks an audit of crates.io and Git packages; a package taken by path is first-party to both, so a
   vendored library would pass them unaudited.
2. `cargo deny --color never --locked check -D index-failure`: advisories (including transitive
   unsound and unmaintained, yanked versions, and unreadable yank statuses), bans, licences
   (including dev dependencies), and sources.
3. `cargo vet --locked`: every third-party version has an imported/local audit or a recorded
   exemption.
"""
from contextlib import contextmanager
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# The two crates this repository holds, by package name and folder inside ROOT.
OURS = {
    "bench-hashes": ".",
    "clocks": "clocks",
}


def _copy_tracked(dest, tracked_files):
    for name in tracked_files:
        src = ROOT / name
        if src.is_file() or src.is_symlink():
            target = dest / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, target, follow_symlinks=False)


@contextmanager
def staged():
    """A private temporary copy of the tracked files under target/ so planted faults never touch the
    working tree and concurrent runs never collide."""
    target_dir = ROOT / "target"
    target_dir.mkdir(exist_ok=True)
    tracked = [n for n in subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, check=True,
                                         capture_output=True, text=True).stdout.split("\0") if n]
    with tempfile.TemporaryDirectory(prefix="dependency-policy-", dir=target_dir) as tmp:
        stage = Path(tmp)
        _copy_tracked(stage / "repo", tracked)
        yield stage


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


def failed(repo, say=print, environment=None):
    """Run all checks on `repo`; return the list of failed check-group names."""
    def run(*command):
        done = subprocess.run(command, cwd=repo, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        say(done.stderr + (done.stdout if command[1] != "metadata" else ""), end="")
        return done

    metadata = run("cargo", "metadata", "--locked", "--all-features", "--format-version", "1")
    if metadata.returncode:
        return ["the lock"]
    wrong = strangers(json.loads(metadata.stdout), root=repo)
    for line in wrong:
        say(f"dependency policy: {line}")
    names = ["by path"] if wrong else []
    deny = run("cargo", "deny", "--color", "never", "--locked", "check", "-D", "index-failure")
    if deny.returncode:
        names += re.findall(r"(\w+) FAILED", deny.stdout + deny.stderr) or ["cargo-deny"]
    vet = run("cargo", "vet", "--locked")
    return names + (["audits"] if vet.returncode else [])


def _header_matches_crate(header_line, crate):
    inner = header_line.strip()
    if inner.startswith("[[") and inner.endswith("]]"):
        inner = inner[2:-2].strip()
    elif inner.startswith("[") and inner.endswith("]"):
        inner = inner[1:-1].strip()
    else:
        return False
    parts = [p.strip().strip('"').strip("'") for p in re.findall(r'"[^"]*"|\'[^\']*\'|[^.\s]+', inner)]
    return len(parts) >= 2 and parts[-1] == crate and parts[-2] in ("audits", "exemptions", "trusted", "wildcard-audits")


def strip_toml_crate_sections(text, crate):
    """Remove every TOML section whose header records audit/exemption/trust evidence for `crate`."""
    out = []
    skip = False
    in_triple = None
    for line in text.splitlines(keepends=True):
        if in_triple is not None:
            if not skip:
                out.append(line)
            if in_triple in line:
                in_triple = None
            continue
        stripped = line.lstrip()
        if stripped.startswith("["):
            skip = _header_matches_crate(stripped, crate)
        if not skip:
            out.append(line)
        for quote in ('"""', "'''"):
            if line.count(quote) % 2 == 1:
                in_triple = quote
                break
    return "".join(out)


def strip_audit_evidence(repo, crate, replace_with=()):
    """Strip all audit, exemption, trust and wildcard records for `crate` across the staged store,
    append `(version, criteria)` exemptions if given, and run `cargo vet fmt`."""
    store = repo / "supply-chain"
    for name in ("config.toml", "audits.toml", "imports.lock"):
        path = store / name
        if path.exists():
            cleaned = strip_toml_crate_sections(path.read_text(), crate)
            if name == "audits.toml" and "audits" not in tomllib.loads(cleaned):
                cleaned = cleaned.rstrip() + "\n\n[audits]\n"
            path.write_text(cleaned)
    if replace_with:
        key = f'"{crate}"' if not re.fullmatch(r"[A-Za-z0-9_-]+", crate) else crate
        extra = "".join(f'\n[[exemptions.{key}]]\nversion = "{version}"\ncriteria = "{criteria}"\n'
                        for version, criteria in replace_with)
        config = store / "config.toml"
        config.write_text(config.read_text() + extra)
    subprocess.run(["cargo", "vet", "fmt"], cwd=repo, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def edit(path, pattern, replacement):
    text, count = re.subn(pattern, replacement, path.read_text(), count=1, flags=re.M)
    assert count == 1, f"{path}: nothing matches {pattern}; this control needs another fault"
    path.write_text(text)


def _write_path_crate(crate_dir, name, licence):
    (crate_dir / "src").mkdir(parents=True, exist_ok=True)
    (crate_dir / "src" / "lib.rs").write_text("")
    (crate_dir / "Cargo.toml").write_text(f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\nlicense = "{licence}"\n')


def a_vulnerable_version(stage):
    # RUSTSEC-2025-0009 (ring 0.17.11, unyanked). Replace audit evidence for ring so only advisories fail.
    repo = stage / "repo"
    edit(repo / "Cargo.lock", r'(name = "ring"\nversion = ")[^"]*("\nsource = "[^"]*"\nchecksum = ")[0-9a-f]*',
         r"\g<1>0.17.11\g<2>da5349ae27d3887ca812fb375b45a4fbb36d8d12d2df394968cd86e35683fe73")
    strip_audit_evidence(repo, "ring", replace_with=[("0.17.11", "safe-to-deploy")])


def a_disallowed_licence(stage):
    edit(stage / "repo" / "deny.toml", r'^    "ISC",\n', "")


def a_disallowed_git_source(stage):
    edit(stage / "repo" / "deny.toml", r'^allow-git = \[.*\]\n', "allow-git = []\n")


def a_crate_taken_by_path(stage):
    repo = stage / "repo"
    _write_path_crate(repo / "copied", "copied", "MIT")
    edit(repo / "Cargo.toml", r'^\[dependencies\.wyhash\]\n', '[dependencies.copied]\npath = "copied"\n\n[dependencies.wyhash]\n')
    subprocess.run(["cargo", "metadata", "--format-version", "1"], cwd=repo, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def a_dev_only_gpl_crate_outside(stage):
    # Placed outside the workspace root so Cargo does not pull it into workspace_members,
    # proving include-dev = true checks dev-dependency licences.
    repo = stage / "repo"
    outside = stage / "outside-dev-gpl"
    _write_path_crate(outside, "outside-dev-gpl", "GPL-3.0-only")
    edit(repo / "Cargo.toml", r'^\[dev-dependencies\.sigilcode\]\n',
         f'[dev-dependencies.outside-dev-gpl]\npath = "{outside.resolve()}"\n\n[dev-dependencies.sigilcode]\n')
    subprocess.run(["cargo", "metadata", "--format-version", "1"], cwd=repo, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def an_unaudited_crate(stage):
    strip_audit_evidence(stage / "repo", "wyhash")


def unreadable_yank_status(stage):
    # Vendor the locked graph beside the manifest and run with an empty CARGO_HOME (no crates.io index):
    # cargo metadata and cargo vet still pass, and -D index-failure makes cargo-deny fail advisories.
    repo = stage / "repo"
    vendored = subprocess.run(["cargo", "vendor", "--locked", "vendor"], cwd=repo, check=True, capture_output=True, text=True)
    (repo / ".cargo").mkdir(exist_ok=True)
    (repo / ".cargo" / "config.toml").write_text(vendored.stdout)
    cargo_home = stage / "cargo-home"
    cargo_home.mkdir()
    home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    if (home / "advisory-dbs").is_dir():
        shutil.copytree(home / "advisory-dbs", cargo_home / "advisory-dbs")
    return dict(os.environ, CARGO_HOME=str(cargo_home), PATH=f"{home / 'bin'}{os.pathsep}{os.environ['PATH']}",
                RUSTUP_HOME=os.environ.get("RUSTUP_HOME", str(Path.home() / ".rustup")))


CONTROLS = [
    ("a version with a security advisory (ring 0.17.11, RUSTSEC-2025-0009)", a_vulnerable_version, ["advisories"]),
    ("a licence outside the list (ISC removed from deny.toml)", a_disallowed_licence, ["licenses"]),
    ("a Git dependency outside allow-git", a_disallowed_git_source, ["sources"]),
    ("a crate copied into the checkout and taken by path", a_crate_taken_by_path, ["by path"]),
    ("a dev-only GPL crate outside the workspace (tests include-dev)", a_dev_only_gpl_crate_outside, ["by path", "licenses"]),
    ("a shipped crate with all audit evidence stripped", an_unaudited_crate, ["audits"]),
    ("versions whose yank status cannot be read (-D index-failure)", unreadable_yank_status, ["advisories"]),
    ("nothing planted: every check passes", lambda _stage: None, []),
]


def controls():
    ok = True
    for name, plant, expected in CONTROLS:
        output = []
        with staged() as stage:
            env = plant(stage)
            got = failed(stage / "repo", say=lambda *parts, end="\n": output.append(" ".join(map(str, parts)) + end), environment=env)
        if sorted(got) == sorted(expected):
            print(f"as it should: {name}: {', '.join(got) + ' failed' if got else 'every check passed'}")
        else:
            ok = False
            print(f"WRONG: {name}: expected {expected}, got {got}", file=sys.stderr)
            print("".join(output), file=sys.stderr)
    return 0 if ok else 1


def main():
    if sys.argv[1:] == ["controls"]:
        return controls()
    if sys.argv[1:]:
        print("usage: python3 tools/dependency_policy.py [controls]", file=sys.stderr)
        return 2
    got = failed(ROOT)
    if got:
        print(f"dependency policy: failed: {', '.join(got)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
