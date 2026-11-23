#!/bin/sh
# Install hash-verified cargo-nextest 0.9.152 into DEST (default:
# ${CARGO_HOME:-$HOME/.cargo}/bin), dropping taiki-e/install-action.
#
#     sh tools/ci-nextest.sh [DEST]
set -eu
VERSION=0.9.152
DEST=${1:-"${CARGO_HOME:-$HOME/.cargo}/bin"}

case "$(uname -sm)" in
  "Linux x86_64")
    target=x86_64-unknown-linux-gnu
    sha=d039b43c3e0958441b6ad0a39c8884d018b3d2e8fb9928f209074b1100b5e17a
    # BLAKE3: 31690208bdb8da793c0480fc470e82662f69c18edf845465b65099f8027a767c
    ;;
  "Linux aarch64")
    target=aarch64-unknown-linux-gnu
    sha=ca1f81a5b86533eb45ab97e0064aa88e925e53aab9a698ca6331a7cad1f85f07
    # BLAKE3: 5364c97094b42f458661d2b1e0626de7b0bb3beeb2e32ab82129d5d1c39af3ab
    ;;
  "Darwin arm64"|"Darwin x86_64")
    target=universal-apple-darwin
    sha=2fadf57df4243e691f088ee3bff4f4ab9af16da0c95a1db84b5cb1685e098a76
    # BLAKE3: 5a6fafdb3f1e4c18116a600ea264eaae7a615a0723fb9dc9756ff06ceab8cdf0
    ;;
  *)
    echo "no cargo-nextest binary named for $(uname -sm)" >&2
    exit 1
    ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
tarball="$tmp/cargo-nextest.tar.gz"
url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${VERSION}/cargo-nextest-${VERSION}-${target}.tar.gz"
curl --proto '=https' --tlsv1.2 -sSfL -o "$tarball" "$url"
printf '%s  %s\n' "$sha" "$tarball" > "$tmp/sum"
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 -c "$tmp/sum"
else
  sha256sum -c "$tmp/sum"
fi
mkdir -p "$DEST"
tar -xzf "$tarball" -C "$DEST" cargo-nextest
"$DEST/cargo-nextest" --version
