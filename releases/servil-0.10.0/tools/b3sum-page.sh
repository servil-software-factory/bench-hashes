#!/bin/sh
# Declared before running (November 18, 2026, UTC), for the 0.10.0 page (BLAKE3 #402): the b3sum suite at
# the 0.10.0 version commit named in C10 below. Three suites: the full inputs and the quick ones with 0.9.1
# as the base and 0.8.0, the 0.10.0 commit and official 1.8.7 beside it in the same rounds; then the full
# inputs with the 0.10.0 commit as the base and official beside it, for the page's table against official.
# On a machine with two memory nodes the builds and the suites run bound to node 0 (numactl), since the
# benchmark measures one memory node (Zooko, November 18). For another commit, change the C10 line alone.
set -eu
export PATH="$HOME/.cargo/bin:$PATH"
O=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "${BOUNDED_R:-}" != yes ]; then
  export BOUNDED_R=yes
  status=0
  timeout --kill-after=30s 10800s sh "$O/run.sh" || status=$?
  printf 'RELEASE_EXIT=%s UTC=%s\n' "$status" "$(date -u +%FT%TZ)"
  exit "$status"
fi
export CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0
unset CARGO_TARGET_DIR RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
B="$HOME/work/bench-release-023"
M="$HOME/work/lib-release-023"
test "$(git -C "$B" rev-parse HEAD)" = cade30fdb71682719f5fa3e586ff96146ae9fc25
C10=2ef4c4431d56ad838f5260a123a8b03d48e00877
C09_TAG='v0.9.1+b01b322ad8c411133e9abc49605e65e1a9684924'
if ! git -C "$M" cat-file -e "$C10^{commit}" 2> /dev/null || ! git -C "$M" cat-file -e "$C09_TAG^{commit}" 2> /dev/null; then
  git -C "$M" fetch --tags --quiet https://github.com/servil-software-factory/BLAKE3 '+refs/heads/*:refs/remotes/origin/*' > "$O/fetch.log" 2>&1
fi
git -C "$M" cat-file -e "$C10^{commit}"
NODE=
if [ -d /sys/devices/system/node/node1 ]; then
  command -v numactl > /dev/null
  NODE="numactl --cpunodebind=0 --membind=0"
fi
{ ls /sys/devices/system/node | tr '\n' ' '; echo; echo "prefix: $NODE"; } > "$O/machine.txt"
C08=$(git -C "$M" rev-parse 'v0.8.0+b3c5efc7c975d903762dea809066f7c0a0603861^{commit}')
C09=$(git -C "$M" rev-parse "$C09_TAG^{commit}")
printf 'bench cade30fdb716\n0.8 %s\n0.9.1 %s\n0.10.0 %s\n' "$C08" "$C09" "$C10" > "$O/versions.txt"
cd "$B"
BINS="$HOME/b3sum-contenders-evolution"
LAYOUTS=4 $NODE python3 tools/b3sum-contenders.py "$M" "$BINS" "$C09" "$C08" "$C10" > "$O/contenders-raw.txt" 2> "$O/contenders.log"
s8=$(printf %s "$C08" | cut -c1-7); s9=$(printf %s "$C09" | cut -c1-7); s10=$(printf %s "$C10" | cut -c1-7)
{ grep "^b3sum-$s9=" "$O/contenders-raw.txt" | sed "s/^b3sum-$s9=/b3sum-0.9.1=/"
  grep "^b3sum-$s8=" "$O/contenders-raw.txt" | sed "s/^b3sum-$s8=/b3sum-0.8.0=/"
  grep "^b3sum-$s10=" "$O/contenders-raw.txt" | sed "s/^b3sum-$s10=/b3sum-0.10.0=/"
  grep '^official=' "$O/contenders-raw.txt"; } > "$O/contenders.txt"
test "$(wc -l < "$O/contenders.txt")" -eq 16
{ grep '^b3sum-0.10.0=' "$O/contenders.txt"; grep '^official=' "$O/contenders.txt"; } > "$O/contenders-base-0100.txt"
test "$(wc -l < "$O/contenders-base-0100.txt")" -eq 8
for mode in full quick base-0100; do
  RUNS="$HOME/work/b3sum-0100-scratch-$mode"; rm -rf "$RUNS"; mkdir "$RUNS"; cd "$RUNS"
  flag=""; [ "$mode" = quick ] && flag="--quick"
  list="$O/contenders.txt"; [ "$mode" = base-0100 ] && list="$O/contenders-base-0100.txt"
  tr '\n' '\0' < "$list" | xargs -0 $NODE "$B/target/release/bench-hashes" b3sum --files "$HOME/b3sum-bench-files" $flag > "$O/b3sum-$mode.stdout.txt" 2> "$O/b3sum-$mode.stderr.txt"
  cp -r benchmark-results "$O/b3sum-$mode-results"; cd /; rm -rf "$RUNS"
  printf '%s done %s\n' "$mode" "$(date -u +%FT%TZ)"
done
