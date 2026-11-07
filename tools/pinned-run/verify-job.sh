#!/bin/bash
set -euo pipefail
R="$1"
mkdir -p "$R/out"
{
    echo "tools $(git -C "$R/tools" rev-parse HEAD)"
    python3 --version
    uname -m
} > "$R/out/identity.txt"
timeout -k 5s 90s python3 "$R/tools/tools/pinned-run/check.py" --output "$R/out" > "$R/out/check.log" 2>&1
printf 'pinned-run validation complete\n' > "$R/out/status.txt"
