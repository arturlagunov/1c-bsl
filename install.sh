#!/usr/bin/env bash
set -euo pipefail

# BSL Language Server Extension for Zed - Install Script
# Installs the extension, downloads the JDK, and configures Zed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Ensure Rust toolchain
if ! command -v cargo &>/dev/null; then
    echo "Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
    source "$HOME/.cargo/env"
fi

cargo run --release --bin installer
