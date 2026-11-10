#!/usr/bin/env bash
# Compiles crates/crusty-wasm and generates its JS bindings into src/core/pkg.
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version 0.2.114   (must match Cargo.lock)
set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"

cargo build -p crusty-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web \
  --out-dir web/src/core/pkg \
  target/wasm32-unknown-unknown/release/crusty_wasm.wasm
