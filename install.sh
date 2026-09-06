#!/usr/bin/env bash
set -euo pipefail

# BSL Language Server Extension for Zed - Install Script
# Installs the extension into Zed, downloads the language server jar and a
# matching Temurin JDK, then wires everything into Zed's settings.json.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZED_EXT_DIR="$HOME/.local/share/zed/extensions/installed/1c-bsl"
ZED_SETTINGS="$HOME/.config/zed/settings.json"
BSL_JAR_URL="https://github.com/1c-syntax/bsl-language-server/releases/latest/download/bsl-language-server.jar"
BSL_JAR_DIR="$HOME/.local/lib/bsl-language-server"
BSL_JAR="$BSL_JAR_DIR/bsl-language-server.jar"
TEMURIN_ROOT="$HOME/.local/lib/temurin"

detect_platform() {
    case "$(uname -s)" in
        Linux) OS="linux" ;;
        Darwin) OS="mac" ;;
        *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64 | amd64) ARCH="x64" ;;
        aarch64 | arm64) ARCH="aarch64" ;;
        *) echo "Unsupported arch: $(uname -m)" >&2; exit 1 ;;
    esac
}

version_python() {
    python3 - "$BSL_JAR" <<'PY'
import sys, zipfile

jar = sys.argv[1]


def class_version(z, path):
    try:
        data = z.read(path)
    except (KeyError, zipfile.BadZipFile):
        return None
    if len(data) < 8 or data[:4] != bytes([0xCA, 0xFE, 0xBA, 0xBE]):
        return None
    return (data[6] << 8) | data[7]


def manifest_version(z):
    try:
        manifest = z.read('META-INF/MANIFEST.MF').decode('utf-8', 'replace')
    except KeyError:
        return None
    for line in manifest.splitlines():
        if not line.startswith('Build-Jdk-Spec:'):
            continue
        try:
            version = int(line.split(':', 1)[1].strip())
        except ValueError:
            return None
        return version if version >= 17 else None
    return None


z = zipfile.ZipFile(jar)
path = 'BOOT-INF/classes/com/github/_1c_syntax/bsl/languageserver/MainApplication.class'
major = class_version(z, path)
if major is not None and major >= 49:
    print(major - 44)
else:
    print(manifest_version(z) or 21)
PY
}

version_manifest() {
    local version
    version=$(unzip -p "$BSL_JAR" META-INF/MANIFEST.MF 2>/dev/null |
        sed -n 's/^Build-Jdk-Spec:[[:space:]]*//p' | head -1)
    if [ -n "$version" ] && [ "$version" -lt 17 ] 2>/dev/null; then
        version=""
    fi
    echo "${version:-21}"
}

required_java() {
    if command -v python3 &>/dev/null; then
        version_python
        return
    fi
    if command -v unzip &>/dev/null; then
        version_manifest
        return
    fi
    echo "ERROR: cannot detect the required Java version (need python3 or unzip)." >&2
    return 1
}

install_jdk() {
    local major="$1"
    local jdk="$TEMURIN_ROOT/jdk-$major"
    local java_bin="$jdk/bin/java"
    JAVA_BIN="$java_bin"

    if [ -x "$java_bin" ] &&
        [ "$("$java_bin" -version 2>&1 | head -1 | grep -oE '[0-9]+' | head -1)" = "$major" ]; then
        echo "Temurin JDK $major already installed at $jdk"
        return
    fi

    echo "Downloading Temurin JDK $major..."
    local url="https://api.adoptium.net/v3/binary/latest/$major/ga/$OS/$ARCH/jdk/hotspot/normal/eclipse"
    local archive
    archive="$(mktemp)"
    if [ ! -f "$archive" ]; then
        archive="${TMPDIR:-/tmp}/temurin-$major.tar.gz"
    fi
    curl -fSL "$url" -o "$archive"
    rm -rf "$jdk"
    mkdir -p "$jdk"
    tar -xzf "$archive" -C "$jdk" --strip-components=1
    rm -f "$archive"
    echo "Temurin JDK $major installed at $jdk"
}

merge_python() {
    python3 - "$1" "$2" "$3" <<'PY'
import json, sys

path, java_path, jar_path = sys.argv[1], sys.argv[2], sys.argv[3]
try:
    with open(path, 'r', encoding='utf-8') as f:
        data = json.load(f)
except (OSError, ValueError):
    data = {}

data.setdefault('lsp', {}).setdefault('bsl', {})['binary'] = {
    'path': java_path,
    'arguments': ['-Xmx4g', '-jar', jar_path],
}
bsl = data.setdefault('languages', {}).setdefault('BSL', {})
bsl['language_servers'] = ['bsl']
bsl['format_on_save'] = 'off'

json.dump(data, sys.stdout, indent=2, ensure_ascii=False)
print()
PY
}

merge_jq() {
    if [ -f "$1" ]; then
        jq --arg jp "$2" --arg jr "$3" \
            '.lsp.bsl.binary = {path: $jp, arguments: ["-Xmx4g", "-jar", $jr]}
             | .languages.BSL.language_servers = ["bsl"]
             | .languages.BSL.format_on_save = "off"' \
            "$1"
    else
        jq -n --arg jp "$2" --arg jr "$3" \
            '{lsp: {bsl: {binary: {path: $jp, arguments: ["-Xmx4g", "-jar", $jr]}}},
              languages: {BSL: {language_servers: ["bsl"], format_on_save: "off"}}}'
    fi
}

settings_json() {
    local settings="$1" java="$2" jar="$3"
    if command -v python3 &>/dev/null; then
        merge_python "$settings" "$java" "$jar"
    elif command -v jq &>/dev/null; then
        merge_jq "$settings" "$java" "$jar"
    fi
}

write_settings() {
    mkdir -p "$(dirname "$ZED_SETTINGS")"

    local merged
    merged="$(settings_json "$ZED_SETTINGS" "$JAVA_BIN" "$BSL_JAR")"
    if [ -n "$merged" ]; then
        printf '%s\n' "$merged" > "$ZED_SETTINGS"
        echo "Settings updated: $ZED_SETTINGS"
        return
    fi

    cat << EOF
NOTE: add the following to $ZED_SETTINGS:
  "lsp": { "bsl": { "binary": { "path": "$JAVA_BIN", "arguments": ["-Xmx4g", "-jar", "$BSL_JAR"] } } },
  "languages": { "BSL": { "language_servers": ["bsl"], "format_on_save": "off" } }
EOF
}

echo "=== BSL Language Server Extension for Zed ==="

# Build if extension.wasm missing
if [ ! -f "$SCRIPT_DIR/extension.wasm" ]; then
    echo "extension.wasm not found, building..."
    bash "$SCRIPT_DIR/build.sh"
fi

# Download BSL Language Server JAR if missing
if [ ! -f "$BSL_JAR" ]; then
    echo "Downloading BSL Language Server..."
    mkdir -p "$BSL_JAR_DIR"
    curl -fSL "$BSL_JAR_URL" -o "$BSL_JAR"
    echo "Downloaded: $BSL_JAR ($(du -h "$BSL_JAR" | cut -f1))"
fi

detect_platform
REQUIRED_JAVA="$(required_java)"
case "$REQUIRED_JAVA" in
    *[!0-9]* | "") echo "ERROR: could not detect the required Java version." >&2; exit 1 ;;
esac
echo "Jar requires Java: $REQUIRED_JAVA"

install_jdk "$REQUIRED_JAVA"
write_settings

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

echo ""
echo "Done! Restart Zed to activate."