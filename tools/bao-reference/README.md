# Bao outboard reference for the comparison study

A single-threaded CLI using bao-tree's public `io::sync::outboard` writer and
kept `PreOrderMemOutboard` storage. It writes the pre-order parent pairs above
16 KiB groups, without a length prefix, to `DIR/<root>.obao4`, then prints the
root and input path. The group buffer and hashing strategy are bao-tree's.

```sh
# Build from a clean, committed checkout; keep the dependency lock.
BAO_REFERENCE_SOURCE=$(git rev-parse HEAD) cargo build --release --locked --manifest-path tools/bao-reference/Cargo.toml
tools/bao-reference/target/release/bao-reference --outboard OUT INPUT...
```

The output directory must already exist, as for the compared b3sum command.
Inputs must be regular files whose contents stay unchanged while mapped. Empty
files use the empty slice. Each process keeps its output allocation across files;
input mappings end after their file. Writes use ordinary filesystem completion,
without fsync. Existing output paths are replaced. These choices are recorded
caller behavior, not a claim to the fastest possible Bao CLI.

`--version` names the adapter's full source commit and prints its exact pinned
bao-tree/BLAKE3 dependency declarations. The study also retains the lock, toolchain
and paired executable digests. The enclosing benchmark's dependencies stay intact:
this reference is a separate Cargo project.

Correctness validation compares complete process outputs with BLAKE3-Servil's
pinned outboard candidate before timing. In-memory generation and full commands
have separate scopes. Hash-only results describe added cost, not equivalent work.
