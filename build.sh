#!/usr/bin/env bash
set -euo pipefail

# BSL Language Server Extension for Zed - Build Script
# Builds WASM component from Rust source

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WASM_TARGET="wasm32-wasip2"

# Check Rust toolchain
if ! command -v rustc &>/dev/null; then
    echo "Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
    source "$HOME/.cargo/env"
fi

# Check WASM target
if ! rustup target list --installed | grep -q "$WASM_TARGET"; then
    echo "Adding $WASM_TARGET target..."
    rustup target add "$WASM_TARGET"
fi

echo "Building WASM component..."
cd "$SCRIPT_DIR"
cargo build --target "$WASM_TARGET" --release

cp "target/$WASM_TARGET/release/zed_1c_bsl.wasm" extension.wasm
echo "Built: extension.wasm ($(stat -c%s extension.wasm | numfmt --to=iec-i))"
