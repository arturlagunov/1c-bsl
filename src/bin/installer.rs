#![allow(dead_code)]

use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use serde_json::{json, Value};
use tar::Archive as TarArchive;

#[path = "../constants.rs"]
mod constants;
#[path = "../zip/mod.rs"]
mod zip;
#[path = "../jar.rs"]
mod jar;
#[path = "../version.rs"]
mod version;

fn main() -> Result<(), String> {
    let root = env::current_dir().map_err(|e| format!("failed to determine cwd: {}", e))?;
    let home = env::var("HOME").map_err(|_| "HOME not set")?;
    let home = PathBuf::from(home);

    let ext_dir = home.join(".local/share/zed/extensions/installed/1c-bsl");
    let settings = home.join(".config/zed/settings.json");
    let jar_dir = home.join(".local/lib/bsl-language-server");
    let jar = jar_dir.join(constants::JAR_FILENAME);
    let temurin = home.join(".local/lib/temurin");

    println!("=== BSL Language Server Extension for Zed ===\n");

    ensure_wasm(&root)?;
    download_jar_if_needed(&jar_dir, &jar)?;
    let required = required_java(&jar)?;
    println!("Jar requires Java: {}", required);
    let (os, arch) = detect_platform()?;
    let java = install_jdk(required, &os, &arch, &temurin)?;
    write_settings(&settings, &java, &jar)?;
    install_extension(&root, &ext_dir)?;

    println!("\nDone! Restart Zed to activate.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Platform detection
// ---------------------------------------------------------------------------

fn detect_platform() -> Result<(&'static str, &'static str), String> {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        other => return Err(format!("unsupported OS: {other}")),
    };
    let arch = match env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => return Err(format!("unsupported architecture: {other}")),
    };
    Ok((os, arch))
}

// ---------------------------------------------------------------------------
// WASM build
// ---------------------------------------------------------------------------

fn ensure_wasm(root: &Path) -> Result<(), String> {
    if root.join("extension.wasm").exists() {
        return Ok(());
    }
    println!("extension.wasm not found, building...");
    let status = Command::new("bash")
        .arg(root.join("build.sh"))
        .current_dir(root)
        .status()
        .map_err(|e| format!("failed to run build.sh: {e}"))?;
    if !status.success() {
        return Err("build.sh failed".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// BSL language-server jar
// ---------------------------------------------------------------------------

fn download_jar_if_needed(dir: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }
    println!("Downloading BSL Language Server...");
    let url = resolve_jar_url()?;
    fs::create_dir_all(dir)
        .map_err(|e| format!("failed to create {}: {}", dir.display(), e))?;
    download_file(&url, dest, "BSL Language Server")?;
    let size = human_size(dest)?;
    println!("Downloaded: {} ({size})", dest.display());
    Ok(())
}

fn resolve_jar_url() -> Result<String, String> {
    let body = http_get_string(
        "https://api.github.com/repos/1c-syntax/bsl-language-server/releases/latest",
    )?;
    let release: Value =
        serde_json::from_str(&body).map_err(|e| format!("failed to parse release JSON: {e}"))?;

    let assets = release["assets"]
        .as_array()
        .ok_or("no assets in release")?;

    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(constants::JAR_FILENAME))
        .or_else(|| {
            assets.iter().find(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n| n.ends_with("-exec.jar"))
            })
        })
        .ok_or("no jar asset found in release")?;

    asset["browser_download_url"]
        .as_str()
        .map(String::from)
        .ok_or("missing download URL".into())
}

// ---------------------------------------------------------------------------
// Java version detection
// ---------------------------------------------------------------------------

fn required_java(jar: &Path) -> Result<u32, String> {
    match jar::BslJar::new(jar).java_version()? {
        Some(v) => Ok(v),
        None => {
            eprintln!(
                "WARNING: could not detect required Java version, assuming {}",
                constants::MINIMUM_REQUIRED_JAVA
            );
            Ok(constants::MINIMUM_REQUIRED_JAVA)
        }
    }
}

fn probe_java_version(java: &Path) -> Option<u32> {
    let output = Command::new(java)
        .args(["-XshowSettings:properties", "-version"])
        .output()
        .ok()?;
    let probe = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    version::parse_output(&probe)
}

// ---------------------------------------------------------------------------
// Temurin JDK
// ---------------------------------------------------------------------------

fn install_jdk(major: u32, os: &str, arch: &str, root: &Path) -> Result<PathBuf, String> {
    let jdk_dir = root.join(format!("jdk-{major}"));
    let java_bin = jdk_dir.join("bin/java");

    if java_bin.exists() {
        if let Some(v) = probe_java_version(&java_bin) {
            if v == major {
                println!("Temurin JDK {major} already installed at {}", jdk_dir.display());
                return Ok(java_bin);
            }
        }
    }

    println!("Downloading Temurin JDK {major}...");
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );

    let archive = env::temp_dir().join(format!("temurin-{major}.tar.gz"));
    download_file(&url, &archive, "Temurin JDK")?;

    if jdk_dir.exists() {
        fs::remove_dir_all(&jdk_dir)
            .map_err(|e| format!("failed to remove {}: {e}", jdk_dir.display()))?;
    }
    extract_jdk_tar(&archive, &jdk_dir)?;
    let _ = fs::remove_file(&archive);

    if !java_bin.exists() {
        return Err(format!(
            "JDK extraction failed: {} not found",
            java_bin.display()
        ));
    }
    fs::set_permissions(&java_bin, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("failed to chmod {}: {e}", java_bin.display()))?;

    println!("Temurin JDK {major} installed at {}", jdk_dir.display());
    Ok(java_bin)
}

fn extract_jdk_tar(archive: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest)
        .map_err(|e| format!("failed to create {}: {e}", dest.display()))?;

    // Discover top-level component
    let top = {
        let file = fs::File::open(archive)
            .map_err(|e| format!("failed to open {}: {e}", archive.display()))?;
        let mut tar = TarArchive::new(GzDecoder::new(file));
        let mut entries = tar.entries().map_err(|e| format!("failed to read tar: {e}"))?;
        let first = entries
            .next()
            .ok_or("empty JDK archive")?
            .map_err(|e| format!("tar entry: {e}"))?;
        first
            .path()
            .map_err(|e| format!("bad tar path: {e}"))?
            .components()
            .next()
            .ok_or("empty tar path")?
            .as_os_str()
            .to_owned()
    };

    // Extract with first component stripped
    let file = fs::File::open(archive)
        .map_err(|e| format!("failed to reopen {}: {e}", archive.display()))?;
    let mut tar = TarArchive::new(GzDecoder::new(file));

    for entry in tar.entries().map_err(|e| format!("failed to iterate tar: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        let path = entry.path().map_err(|e| format!("bad path: {e}"))?;

        let stripped = path.strip_prefix(&top).unwrap_or(&path);
        if stripped.as_os_str().is_empty() {
            continue;
        }

        let target = dest.join(stripped);

        if let Some(parent) = target.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
        }

        entry
            .unpack(&target)
            .map_err(|e| format!("extract {}: {e}", target.display()))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

fn write_settings(path: &Path, java: &Path, jar: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    let exists = path
        .metadata()
        .map(|m| m.len() > 0)
        .unwrap_or(false);

    if !exists {
        fs::write(path, fresh_settings(java, jar))
            .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        println!("Settings updated: {}", path.display());
        return Ok(());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let stripped = strip_comments(&content);

    match serde_json::from_str::<Value>(&stripped) {
        Ok(mut settings) => {
            settings["lsp"]["bsl"]["binary"]["path"] =
                json!(java.to_string_lossy().as_ref());
            settings["lsp"]["bsl"]["binary"]["arguments"] =
                json!(["-Xmx4g", "-jar", jar.to_string_lossy().as_ref()]);
            settings["languages"]["BSL"]["language_servers"] = json!(["bsl"]);
            settings["languages"]["BSL"]["format_on_save"] = json!("off");

            let output = serde_json::to_string_pretty(&settings)
                .map_err(|e| format!("failed to serialize settings: {e}"))?;
            fs::write(path, output)
                .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
            println!("Settings updated: {}", path.display());
        }
        Err(e) => {
            eprintln!(
                "NOTE: {} could not be parsed ({e}).\n\
                 Add this block to it manually:\n\n\
                 \"lsp\": {{ \"bsl\": {{ \"binary\": {{ \"path\": \"{}\", \
                 \"arguments\": [\"-Xmx4g\", \"-jar\", \"{}\"] }} }} }},\n\
                 \"languages\": {{ \"BSL\": {{ \"language_servers\": [\"bsl\"], \
                 \"format_on_save\": \"off\" }} }}",
                path.display(),
                java.display(),
                jar.display(),
            );
        }
    }

    Ok(())
}

fn strip_comments(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn fresh_settings(java: &Path, jar: &Path) -> String {
    format!(
        r#"{{
  "lsp": {{
    "bsl": {{
      "binary": {{
        "path": "{}",
        "arguments": ["-Xmx4g", "-jar", "{}"]
      }}
    }}
  }},
  "languages": {{
    "BSL": {{
      "language_servers": ["bsl"],
      "format_on_save": "off"
    }}
  }}
}}"#,
        java.display(),
        jar.display(),
    )
}

// ---------------------------------------------------------------------------
// Extension files
// ---------------------------------------------------------------------------

fn install_extension(root: &Path, ext_dir: &Path) -> Result<(), String> {
    println!("Installing Zed extension...");
    if ext_dir.exists() {
        fs::remove_dir_all(ext_dir)
            .map_err(|e| format!("failed to remove {}: {e}", ext_dir.display()))?;
    }
    fs::create_dir_all(ext_dir)
        .map_err(|e| format!("failed to create {}: {e}", ext_dir.display()))?;

    fs::copy(root.join("extension.toml"), ext_dir.join("extension.toml"))
        .map_err(|e| format!("copy extension.toml: {e}"))?;
    fs::copy(root.join("extension.wasm"), ext_dir.join("extension.wasm"))
        .map_err(|e| format!("copy extension.wasm: {e}"))?;

    for dir in &["grammars", "languages", "snippets"] {
        let src = root.join(dir);
        if src.exists() {
            copy_dir(&src, &ext_dir.join(dir))
                .map_err(|e| format!("copy {dir}: {e}"))?;
        }
    }

    println!("Extension installed to: {}", ext_dir.display());
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

fn http_get_string(url: &str) -> Result<String, String> {
    let resp = ureq::get(url)
        .set("User-Agent", "1c-bsl-zed-extension")
        .call()
        .map_err(|e| format!("HTTP GET {url} failed: {e}"))?;
    resp.into_string()
        .map_err(|e| format!("failed to read response from {url}: {e}"))
}

fn download_file(url: &str, dest: &Path, label: &str) -> Result<(), String> {
    let resp = ureq::get(url)
        .set("User-Agent", "1c-bsl-zed-extension")
        .call()
        .map_err(|e| format!("failed to download {label}: {e}"))?;

    let mut reader = resp.into_reader();
    let tmp = dest.with_extension("tmp");
    let mut file = fs::File::create(&tmp)
        .map_err(|e| format!("failed to create {}: {e}", tmp.display()))?;
    io::copy(&mut reader, &mut file)
        .map_err(|e| format!("failed to write {label}: {e}"))?;
    drop(file);
    fs::rename(&tmp, dest)
        .map_err(|e| format!("failed to rename {}: {e}", tmp.display()))?;
    Ok(())
}

fn human_size(path: &Path) -> Result<String, String> {
    let bytes = fs::metadata(path)
        .map_err(|e| format!("stat {}: {e}", path.display()))?
        .len();
    Ok(match bytes {
        b if b >= 1024 * 1024 => format!("{} MB", b / (1024 * 1024)),
        b if b >= 1024 => format!("{} KB", b / 1024),
        b => format!("{b} B"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_comments_removes_full_line_comments() {
        let input = "{\n  // comment\n  \"key\": 1\n}";
        assert_eq!(strip_comments(input), "{\n  \"key\": 1\n}");
    }

    #[test]
    fn strip_comments_preserves_inline_strings() {
        let input = r#"{"key": "value with // inside"}"#;
        assert_eq!(strip_comments(input), input);
    }

    #[test]
    fn fresh_settings_is_valid_json() {
        let java = PathBuf::from("/usr/bin/java");
        let jar = PathBuf::from("/tmp/test.jar");
        let settings = fresh_settings(&java, &jar);
        let parsed: Value = serde_json::from_str(&settings).unwrap();
        assert_eq!(
            parsed["lsp"]["bsl"]["binary"]["path"].as_str().unwrap(),
            "/usr/bin/java"
        );
        assert_eq!(
            parsed["languages"]["BSL"]["language_servers"][0]
                .as_str()
                .unwrap(),
            "bsl"
        );
    }
}
