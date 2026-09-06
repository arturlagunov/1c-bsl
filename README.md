# 1c-bsl — Zed extension using the official BSL Language Server

A Zed extension that connects the **official [bsl-language-server](https://1c-syntax.github.io/bsl-language-server/)**
(Java, by 1c-syntax) to `.bsl` files in Zed. It runs through a full **WASM component** (not a Rust port of
bsl-analyzer) built for the WebAssembly Component Model.

## What you get

- Syntax highlighting for BSL / OneScript (`.bsl`, `.osl`) and SDBL (tree-sitter)
- LSP features from the official **BSL Language Server**: go to definition, hover, autocomplete, synonyms,
  diagnostics, code actions, rename, document symbols, semantic tokens and more
- Automatic download of the latest `bsl-language-server.jar` on first use — no manual setup required
- Dynamic detection of the required Java version: the extension inspects the **downloaded** jar to determine
  the Java version it was built for and verifies your installed Java runtime before launching

## Requirements

- **Zed** (Linux/macOS)
- **Java 17+** (Java 21+ recommended; the exact requirement is detected dynamically from the downloaded jar).
  Java must be on your `PATH`.

## Installation

The extension is intended to be published to the Zed extension registry. Until then you can install it
manually:

```bash
bash install.sh
```

For development you can also use Zed's `Install Dev Extension` on this directory.

### How it works

On the first `.bsl` file open:

1. The extension looks for an existing `bsl-language-server.jar` in the following places:
   - the `BSL_LANGUAGE_SERVER` environment variable (path to a jar)
   - `~/.local/lib/bsl-language-server/bsl-language-server.jar` (also `$XDG_DATA_HOME/lib/...`)
   - a previously downloaded jar in the extension directory
2. If none exists, it downloads the latest release of
   [`1c-syntax/bsl-language-server`](https://github.com/1c-syntax/bsl-language-server) into the extension
   directory (`binaries/bsl-language-server.jar`).
3. It dynamically determines the Java version required by the downloaded jar (from its bytecode and manifest)
   and the version of the `java` runtime found on your `PATH` (`BSL_LANGUAGE_SERVER_JAVA_OPTS` overrides the
   default `-Xmx4g` heap size). If your Java is too old, you get a clear message instead of a dead server.
4. The language server is started with `java -Xmx4g -jar <jar>`.

### Configuration

You can fully override the launch command in `~/.config/zed/settings.json`:

```jsonc
{
  "lsp": {
    "bsl": {
      "binary": {
        "path": "java",
        "arguments": ["-Xmx4g", "-jar", "/path/to/bsl-language-server.jar"]
      }
    }
  }
}
```

If a `binary` is configured it is used as-is; otherwise the automatic behavior above applies.

Set `BSL_LANGUAGE_SERVER_JAVA_OPTS` (e.g. `-Xmx2g -Xss4m`) to customize JVM flags when not using the
`binary` override. The extension also checks the `BSL_LANGUAGE_SERVER` environment variable for an explicit
path to an existing jar.

## Building the WASM component from source

```bash
bash build.sh   # requires rustup + the wasm32-wasip2 target
```

Output: `extension.wasm` (a WebAssembly Component; Zed rejects plain modules with
`failed to compile wasm component`).

After any change to `src/`, rebuild with `bash build.sh` and commit the new `extension.wasm`.

## Troubleshooting

- Logs: `~/.local/share/zed/logs/Zed.log` — look for `starting language server process. binary path: "java"`
- Extension load errors: `Failed to load extension: 1c-bsl`
- Memory: `-Xmx` defaults to 4 GB (enough for large projects like BSP/ERP). Override via
  `BSL_LANGUAGE_SERVER_JAVA_OPTS`. If the JVM runs out of heap the connection silently drops
  (`server shut down`).
- Java version mismatch: the extension reports the required Java version detected from the downloaded jar.

## License

MIT. This extension is a derivative work of [Nesterland/zed-1c-bsl](https://github.com/Nesterland/zed-1c-bsl).
The core language server is [1c-syntax/bsl-language-server](https://github.com/1c-syntax/bsl-language-server).