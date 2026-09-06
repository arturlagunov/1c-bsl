use std::path::PathBuf;

use crate::constants::MINIMUM_REQUIRED_JAVA;
use crate::jar::BslJar;

fn local_jar() -> PathBuf {
    PathBuf::from(env!("HOME"))
        .join(".local/lib/bsl-language-server/bsl-language-server.jar")
}

#[test]
fn detects_version() {
    let jar = local_jar();
    if !jar.is_file() {
        return;
    }

    let required = BslJar::new(&jar).java_version().unwrap();
    assert_eq!(required, Some(21), "the local jar is built for Java 21");
}

#[test]
fn version_matches() {
    let jar = local_jar();
    if !jar.is_file() {
        return;
    }

    let required = BslJar::new(&jar)
        .java_version()
        .unwrap()
        .unwrap_or(MINIMUM_REQUIRED_JAVA);
    let installed = installed_version();

    assert!(
        installed >= required,
        "installed Java {} is too old for the jar (requires {}); \
         the extension would refuse to start",
        installed,
        required
    );
}

#[test]
fn jar_launches() {
    let jar = local_jar();
    if !jar.is_file() {
        return;
    }

    let output = std::process::Command::new("java")
        .arg("-jar")
        .arg(&jar)
        .arg("--version")
        .output()
        .expect("failed to run java -jar --version");

    assert!(
        output.status.success(),
        "jar did not start with the installed Java: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("version"),
        "expected version output"
    );
}

fn installed_version() -> u32 {
    let output = std::process::Command::new("java")
        .args(["-XshowSettings:properties", "-version"])
        .output()
        .expect("failed to run java");
    let probe = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    probe
        .lines()
        .find_map(|line| {
            if line.contains("java.specification.version") {
                line.split('=').nth(1).map(str::trim)
            } else {
                None
            }
        })
        .and_then(|version| version.split('.').next())
        .and_then(|major| major.parse().ok())
        .unwrap_or_else(|| panic!("no parseable java.specification.version in:\n{}", probe))
}