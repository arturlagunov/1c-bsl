#!/usr/bin/env bash
set -euo pipefail

# BSL Language Server Extension for Zed - Install Script
# Installs the extension into Zed and configures settings

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZED_EXT_DIR="$HOME/.local/share/zed/extensions/installed/1c-bsl"
ZED_SETTINGS="$HOME/.config/zed/settings.json"
BSL_JAR_URL="https://github.com/1c-syntax/bsl-language-server/releases/latest/download/bsl-language-server.jar"
BSL_JAR_DIR="$HOME/.local/lib/bsl-language-server"
BSL_JAR="$BSL_JAR_DIR/bsl-language-server.jar"

echo "=== BSL Language Server Extension for Zed ==="

# Build if extension.wasm missing
if [ ! -f "$SCRIPT_DIR/extension.wasm" ]; then
    echo "extension.wasm not found, building..."
    bash "$SCRIPT_DIR/build.sh"
fi

# Download BSL Language Server JAR (optional - the extension can download it automatically)
# Kept so an existing jar at the well-known location is reused instead of re-downloaded.
if [ ! -f "$BSL_JAR" ]; then
    echo "Downloading BSL Language Server..."
    mkdir -p "$BSL_JAR_DIR"
    curl -fSL "$BSL_JAR_URL" -o "$BSL_JAR"
    echo "Downloaded: $BSL_JAR ($(stat -c%s "$BSL_JAR" | numfmt --to=iec-i))"
fi

# Check Java
if ! command -v java &>/dev/null; then
    echo "ERROR: Java not found. Install Java 17+ (Temurin recommended)."
    echo "  curl -sL https://adoptium.net/temurin/releases/ -o /dev/null"
    exit 1
fi
JAVA_VER=$(java -version 2>&1 | head -1 | grep -oP '[\d.]+' | head -1)
echo "Java: $JAVA_VER"

# Install extension
echo "Installing Zed extension..."
rm -rf "$ZED_EXT_DIR"
mkdir -p "$ZED_EXT_DIR"
cp "$SCRIPT_DIR/extension.toml" "$ZED_EXT_DIR/"
cp "$SCRIPT_DIR/extension.wasm" "$ZED_EXT_DIR/"
cp -r "$SCRIPT_DIR/grammars" "$ZED_EXT_DIR/"
cp -r "$SCRIPT_DIR/languages" "$ZED_EXT_DIR/"
cp -r "$SCRIPT_DIR/snippets" "$ZED_EXT_DIR/"
echo "Extension installed to: $ZED_EXT_DIR"

# Update settings.json if needed
if ! grep -q '"bsl"' "$ZED_SETTINGS" 2>/dev/null; then
    echo ""
    echo "NOTE: Add this to your ~/.config/zed/settings.json:"
    echo ""
    cat << 'SETTINGS'
  "lsp": {
    "bsl": {
      "binary": {
        "path": "java",
        "arguments": ["-Xmx4g", "-jar", "~/.local/lib/bsl-language-server/bsl-language-server.jar"]
      }
    }
  },
  "languages": {
    "BSL": {
      "language_servers": ["bsl"],
      "format_on_save": "off"
    }
  },
SETTINGS
fi

echo ""
echo "Done! Restart Zed to activate."
