#!/bin/bash
# A small real compiler/reference fixture for the preflight gate.
# Repositories: library (BLAKE3-Servil), tools (this source).
set -euo pipefail
R="$1"; O="$R/out"
mkdir -p "$O" "$R/probe/src"
export HOME=/root
. /root/.cargo/env
{
    echo "library $(git -C "$R/library" rev-parse HEAD)"
    echo "tools $(git -C "$R/tools" rev-parse HEAD)"
    rustc -V
} > "$O/identity.txt"
printf '[package]\nname="preflight-example"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nblake3-servil={path="%s/library"}\n' "$R" > "$R/probe/Cargo.toml"
cat > "$R/probe/src/main.rs" <<'RUST'
fn main() {
    blake3_servil::initialize();
    let mut hasher = blake3_servil::convenience::Hasher::new(blake3_servil::Mode::Plain, None);
    hasher.update(b"a");
    hasher.update(b"bc");
    assert_eq!(hasher.finalize().to_hex().as_str(),
        "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85");
    println!("abc reference passed");
}
RUST
# This fixture's deliberate compile fault demonstrates that a broken probe
# fails once in preflight and launches no target jobs.
if [ "${PREFLIGHT_PLANT:-}" = compile ]; then
    printf '\ncompile_error!("planted preflight compile error");\n' >> "$R/probe/src/main.rs"
fi
(cd "$R/probe" && CARGO_TARGET_DIR="$R/target" cargo build --release && cargo tree -p blake3-servil --depth 0) > "$O/build.log" 2>&1
"$R/target/release/preflight-example" > "$O/reference.txt" 2>&1
if [ "${PINNED_RUN_PREFLIGHT:-}" = 1 ]; then
    printf 'pinned-run preflight complete\n' > "$O/preflight.txt"
else
    printf 'fanout example complete\n' > "$O/status.txt"
fi
