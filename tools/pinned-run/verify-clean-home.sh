#!/bin/bash
# Reviewer reproduction: standard system PATH, empty HOME, no Sam-specific tools.
set -euo pipefail
R="$1"
mkdir -p "$R/out" "$R/reviewer-home"
{
    echo "tools $(git -C "$R/tools" rev-parse HEAD)"
    python3 --version
    uname -m
} > "$R/out/identity.txt"
if PATH=/usr/bin:/bin command -v replace; then
    echo 'clean-PATH fixture unexpectedly has replace' >&2
    exit 1
fi
HOME="$R/reviewer-home" PATH=/usr/bin:/bin timeout -k 5s 90s \
    python3 "$R/tools/tools/pinned-run/check.py" --output "$R/out" > "$R/out/check.log" 2>&1
[ ! -e "$R/reviewer-home/work" ]
printf 'self-contained runner validation complete\n' > "$R/out/status.txt"
