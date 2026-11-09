#!/bin/bash
set -euo pipefail
export TMPDIR=/home/b3-builds/tmp-arm-last CC=clang-19 CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true
for side in old new; do
 export CARGO_TARGET_DIR=/home/b3-builds/grouped-noalloc/finish-state-$side
 cd /workspace/blake3-servil/tmp/wt-finish-state-$side
 cargo nextest run --release --lib -E 'test(finish_shape_state_probe)' --success-output immediate > /workspace/blake3-servil/tmp/night/nov9-overnight/finish-state-$side.log 2>&1
 grep "STATE_SOURCE=$side" /workspace/blake3-servil/tmp/night/nov9-overnight/finish-state-$side.log
 done
