#!/bin/sh
# Install and select the pinned Rust toolchain for CI, matching BLAKE3's
# tools/b3sum-release.toolchain and dropping dtolnay/rust-toolchain.
#
#     sh tools/ci-rust.sh [COMPONENT...]
set -eu
TOOLCHAIN=1.99.1
args=""
for c in "$@"; do
    args="$args --component $c"
done
# shellcheck disable=SC2086
rustup toolchain install "$TOOLCHAIN" --profile minimal --no-self-update $args
rustup default "$TOOLCHAIN"
rustc -vV
cargo -V
