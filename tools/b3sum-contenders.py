"""Build b3sum contenders for `bench-hashes b3sum` into DIR, each named for what it is:
    python3 tools/b3sum-contenders.py [--outboard] FORK DIR [COMMIT...]
With --outboard, each BLAKE3-Servil build also has an --outboard {OUT} form,
beside its unchanged hash-only form (the added-cost baseline).
FORK is a checkout of github.com/servil-software-factory/BLAKE3.
  DIR/b3sum-official-1.8.7  official BLAKE3's b3sum from crates.io (also as
                            official-1-thread, the same build with
                            --num-threads 1)
  DIR/b3sum-COMMIT          BLAKE3-Servil's b3sum at each COMMIT (default: FORK's HEAD),
                            built from a clean worktree of that commit as its
                            README recommends: statically linked on Linux;
                            named b3sum where one COMMIT is given (with
                            b3sum-1-thread, the same build with --num-threads 1:
                            the names the README's chart reads), b3sum-COMMIT
                            where several are
  sha256sum, sha1sum, md5sum  the system's own, where installed: the programs
                            a user chooses between
Each b3sum is built as its own project recommends (official's from crates.io,
as `cargo install b3sum` builds it); on Linux, BLAKE3-Servil's static link alone
makes each of its runs about 240 μs shorter (METHODOLOGY, b3sum).
With LAYOUTS=N (default 1), each b3sum is also relinked N times, its functions
in a random order each time (lld's --symbol-ordering-file, as BLAKE3-Servil's check
relinks the library), DIR/...-L0 to -L(N-1): a binary's layout alone moves its
times by several percent, so a program's cells should not rest on one layout's
luck. Layout i's seed is the build's name's checksum plus i: each build's
layouts are its own (two builds never share a seed, as check gives each side
its own), and the same on every invocation, so a layout built before is the one
asked for. bench-hashes b3sum runs a contender named more than once one build a
round.
Prints the contenders for bench-hashes b3sum, one NAME=COMMAND a line, official
first; pass each line as one argument (a command may hold flags):
    python3 tools/b3sum-contenders.py FORK DIR | tr '\\n' '\\0' | xargs -0 cargo run --release -- b3sum --files FILES
"""

import glob
import os
import random
import shutil
import subprocess
import sys
import zlib


def out(*cmd, **kw):
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw).stdout.strip()


def run(*cmd, env=None):
    subprocess.run(cmd, check=True, stdout=sys.stderr, env=None if env is None else {**os.environ, **env})


def relink(name, manifest, built, flags="", *, target, host, layouts, binary="b3sum"):
    """`built` and, with LAYOUTS above 1, BUILT-L0... beside it, each relinked
    with its functions in a random order (seed: the name's checksum plus i),
    as NAME=PATH contenders."""
    if layouts <= 1:
        return [f"{name}={built}"]
    lld = os.path.join(target, "lld")
    os.makedirs(lld, exist_ok=True)
    if not os.path.exists(os.path.join(lld, "ld.lld")):
        newest = max(glob.glob("/usr/bin/ld.lld-*"), key=lambda p: int(p.rsplit("-", 1)[1]))
        os.symlink(newest, os.path.join(lld, "ld.lld"))
    base = os.path.basename(built)
    symbols = sorted({f[2] for f in (l.split() for l in out("nm", built).splitlines()) if len(f) == 3 and f[1] in "tT"})
    seed = zlib.crc32(base.encode())
    contenders = []
    for i in range(layouts):
        path = f"{built}-L{i}"
        if not os.access(path, os.X_OK):
            order = os.path.join(target, f"order-{base}-{i}")
            shuffled = list(symbols)
            random.Random(seed + i).shuffle(shuffled)
            with open(order, "w") as f:
                f.writelines(s + "\n" for s in shuffled)
            build = os.path.join(target, f"relink-{base}")
            run("cargo", "rustc", "--quiet", "--release", "--locked", "--manifest-path", manifest, "--bin", binary,
                "--target", host, "--", "-Clink-arg=-fuse-ld=lld", f"-Clink-arg=-B{lld}",
                f"-Clink-arg=-Wl,--symbol-ordering-file={order}", "-Clink-arg=-Wl,--no-warn-symbol-ordering",
                env={"RUSTFLAGS": flags, "CARGO_TARGET_DIR": build})
            shutil.copy(os.path.join(build, host, "release", binary), path)
        contenders.append(f"{name}={path}")
    return contenders


def servil_forms(builds, one, outboard):
    """A Servil build's hash-only and optional outboard contenders."""
    forms = list(builds)
    if one:
        forms += [c.replace("=", "-1-thread=", 1) + " --num-threads 1" for c in builds]
    if outboard:
        forms += [c.replace("=", "-outboard=", 1) + " --outboard {OUT}" for c in builds]
        if one:
            forms += [c.replace("=", "-outboard-1-thread=", 1) + " --num-threads 1 --outboard {OUT}" for c in builds]
    return forms


def main():
    args = sys.argv[1:]
    outboard = "--outboard" in args
    if outboard:
        args.remove("--outboard")
    fork = out("git", "-C", args[0], "rev-parse", "--show-toplevel")
    os.makedirs(args[1], exist_ok=True)
    dir = os.path.abspath(args[1])
    revs = args[2:] or ["HEAD"]
    target = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(fork, "target")), "b3sum-contenders")
    host = next(l.split(": ", 1)[1] for l in out("rustc", "-vV").splitlines() if l.startswith("host: "))
    layouts = int(os.environ.get("LAYOUTS", "1"))
    # BLAKE3-Servil's b3sum as its README recommends: statically linked on Linux.
    static = "-C target-feature=+crt-static" if host.endswith("-linux-gnu") else ""
    layout = dict(target=target, host=host, layouts=layouts)

    # Official BLAKE3 1.8.7, the release BLAKE3-Servil comes from and the one the
    # bench's hash cells measure (Cargo.lock): one reference point (Zooko,
    # October 25).
    OFFICIAL = "1.8.7"
    official = os.path.join(dir, f"b3sum-official-{OFFICIAL}")
    if not os.access(official, os.X_OK):
        run("cargo", "install", "--quiet", "--locked", "b3sum", "--version", OFFICIAL, "--root", os.path.join(dir, f"official-{OFFICIAL}"),
            env={"CARGO_TARGET_DIR": os.path.join(target, "official")})
        shutil.copy(os.path.join(dir, f"official-{OFFICIAL}", "bin", "b3sum"), official)
    cargo_home = os.environ.get("CARGO_HOME", os.path.expanduser("~/.cargo"))
    official_src = sorted(glob.glob(os.path.join(cargo_home, "registry", "src", "*", f"b3sum-{OFFICIAL}")))[0]
    contenders = relink("official", os.path.join(official_src, "Cargo.toml"), official, **layout)
    # Official's on one thread too, as BLAKE3-Servil's is, so the two compare on one
    # thread as well as on every core (official's maintainer asked; Zooko,
    # October 25).
    contenders += [c.replace("official=", "official-1-thread=", 1) + " --num-threads 1" for c in list(contenders)]
    for rev in revs:
        commit = out("git", "-C", fork, "rev-parse", "--short=7", f"{rev}^{{commit}}")
        worktree = os.path.join(target, f"worktree-{commit}")
        built = os.path.join(dir, f"b3sum-{commit}")
        # The worktree is wanted below too (relink builds from it), so it is made
        # whether or not an earlier run left the build in DIR.
        if not os.path.isdir(worktree):
            run("git", "-C", fork, "worktree", "add", "--detach", "--quiet", worktree, commit)
        if not os.access(built, os.X_OK):
            run("cargo", "build", "--quiet", "--release", "--manifest-path", os.path.join(worktree, "b3sum", "Cargo.toml"),
                "--target", host, env={"RUSTFLAGS": static, "CARGO_TARGET_DIR": os.path.join(target, "build")})
            shutil.copy(os.path.join(target, "build", host, "release", "b3sum"), built)
        one = len(revs) == 1
        builds = relink("b3sum" if one else f"b3sum-{commit}", os.path.join(worktree, "b3sum", "Cargo.toml"), built, static, **layout)
        contenders += servil_forms(builds, one, outboard)
    # The system's own sum programs, as a user has them (whichever implementation
    # the platform ships: GNU or uutils coreutils, BSD).
    contenders += [f"{n}={shutil.which(n)}" for n in ("sha256sum", "sha1sum", "md5sum") if shutil.which(n)]
    print("\n".join(contenders))


if __name__ == "__main__":
    main()
