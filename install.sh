#!/usr/bin/env bash
set -euo pipefail

# BSL Language Server Extension for Zed - Install Script
# Installs the extension into Zed, downloads the language server jar and a
# matching Temurin JDK, then wires everything into Zed's settings.json.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZED_EXT_DIR="$HOME/.local/share/zed/extensions/installed/1c-bsl"
ZED_SETTINGS="$HOME/.config/zed/settings.json"
BSL_JAR_DIR="$HOME/.local/lib/bsl-language-server"
BSL_JAR="$BSL_JAR_DIR/bsl-language-server.jar"
TEMURIN_ROOT="$HOME/.local/lib/temurin"

ensure_jq() {
    if command -v jq &>/dev/null; then
        return 0
    fi
    echo "jq not found, installing..."
    case "$OS" in
        linux)
            if command -v apt-get &>/dev/null; then
                sudo apt-get install -y jq
            elif command -v dnf &>/dev/null; then
                sudo dnf install -y jq
            elif command -v pacman &>/dev/null; then
                sudo pacman -S --noconfirm jq
            elif command -v apk &>/dev/null; then
                sudo apk add jq
            else
                echo "ERROR: no known package manager, install jq manually." >&2
                return 1
            fi ;;
        mac)
            if command -v brew &>/dev/null; then
                brew install jq
            elif command -v port &>/dev/null; then
                sudo port install jq
            else
                echo "ERROR: no known package manager, install jq manually." >&2
                return 1
            fi ;;
    esac
    command -v jq &>/dev/null
}

bsl_jar_url() {
    local api="https://api.github.com/repos/1c-syntax/bsl-language-server/releases/latest"
    local json tag asset
    if json="$(curl -fsSL "$api")"; then
        tag="$(printf '%s' "$json" | jq -r '.tag_name')"
        asset="$(printf '%s' "$json" | jq -r '(.assets // [])[] | select(.name | endswith("-exec.jar")) | .name' 2>/dev/null || true)"
        if [ -n "$tag" ] && [ -n "$asset" ]; then
            echo "https://github.com/1c-syntax/bsl-language-server/releases/download/$tag/$asset"
            return
        fi
    fi
    echo "https://github.com/1c-syntax/bsl-language-server/releases/latest/download/bsl-language-server.jar"
}

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

jar_major() {
    local path="$1"
    local hdr bytes hi lo
    hdr="$(unzip -p "$BSL_JAR" "$path" 2>/dev/null | head -c 4 | od -An -tx1 | tr -d ' \n')"
    [ "$hdr" = "cafebabe" ] || return 1
    bytes="$(unzip -p "$BSL_JAR" "$path" 2>/dev/null | head -c 8 | tail -c 2 | od -An -tu1)"
    read -r hi lo <<< "$bytes"
    [ -n "$lo" ] || return 1
    echo $((hi * 256 + lo))
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
    if ! command -v unzip &>/dev/null; then
        echo "ERROR: cannot detect the required Java version: \`unzip\` is required." >&2
        return 1
    fi
    local major
    major="$(jar_major 'BOOT-INF/classes/com/github/_1c_syntax/bsl/languageserver/MainApplication.class')" || major=""
    if [ -n "$major" ] && [ "$major" -ge 49 ] 2>/dev/null; then
        echo $((major - 44))
        return
    fi
    version_manifest
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

strip_comments() {
    sed '/^[[:space:]]*\/\//d' "$1"
}

merge_settings() {
    if [ -f "$1" ]; then
        strip_comments "$1" | jq --arg jp "$2" --arg jr "$3" \
            '.lsp.bsl.binary = {path: $jp, arguments: ["-Xmx4g", "-jar", $jr]}
             | .languages.BSL.language_servers = ["bsl"]
             | .languages.BSL.format_on_save = "off"'
    else
        jq -n --arg jp "$2" --arg jr "$3" \
            '{lsp: {bsl: {binary: {path: $jp, arguments: ["-Xmx4g", "-jar", $jr]}}},
              languages: {BSL: {language_servers: ["bsl"], format_on_save: "off"}}}'
    fi
}

write_settings() {
    mkdir -p "$(dirname "$ZED_SETTINGS")"

    local merged
    merged="$(merge_settings "$ZED_SETTINGS" "$JAVA_BIN" "$BSL_JAR")"
    if [ -z "$merged" ]; then
        echo "ERROR: failed to generate $ZED_SETTINGS." >&2
        return 1
    fi
    printf '%s\n' "$merged" > "$ZED_SETTINGS"
    echo "Settings updated: $ZED_SETTINGS"
}

echo "=== BSL Language Server Extension for Zed ==="

detect_platform
ensure_jq || exit 1

# Build if extension.wasm missing
if [ ! -f "$SCRIPT_DIR/extension.wasm" ]; then
    echo "extension.wasm not found, building..."
    bash "$SCRIPT_DIR/build.sh"
fi

# Download BSL Language Server JAR if missing
if [ ! -f "$BSL_JAR" ]; then
    echo "Downloading BSL Language Server..."
    mkdir -p "$BSL_JAR_DIR"
    curl -fSL "$(bsl_jar_url)" -o "$BSL_JAR"
    echo "Downloaded: $BSL_JAR ($(du -h "$BSL_JAR" | cut -f1))"
fi

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