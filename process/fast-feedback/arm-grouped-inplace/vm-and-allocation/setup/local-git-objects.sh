#!/bin/bash
set -euo pipefail
export CARGO_TARGET_DIR=/home/b3-builds/grouped-noalloc
export TMPDIR=/home/b3-builds/tmp-arm-last CC=clang-19 CARGO_BUILD_JOBS=6 CARGO_NET_OFFLINE=true
# Both exact commits already exist locally; use their Git objects, not the
# working trees (the benchmark has original uncommitted Cargo edits).
export GIT_CONFIG_COUNT=2
export GIT_CONFIG_KEY_0=url.file:///workspace/blake3-servil/.git.insteadOf
export GIT_CONFIG_VALUE_0=https://github.com/servil-software-factory/BLAKE3
export GIT_CONFIG_KEY_1=url.file:///workspace/bench-hashes/.git.insteadOf
export GIT_CONFIG_VALUE_1=https://github.com/servil-software-factory/bench-hashes
cd /workspace/blake3-servil/tmp/runs/grouped-typed-vm-local/ab
python3 /workspace/blake3-servil/tmp/wt-safety-costs/tools/studies/mac_cycles.py next ab
printf 'GROUPED-TYPED-VM-AB-PASS\n'
