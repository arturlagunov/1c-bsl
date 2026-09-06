use std::path::PathBuf;

use crate::constants::{MAIN_CLASS_PATH, MANIFEST_PATH, MINIMUM_REQUIRED_JAVA};
use crate::zip::ZipArchive;

const CLASS_FILE_MAGIC: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];

pub struct BslJar {
    path: PathBuf,
}

impl BslJar {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Minimum Java version required to run this jar: the major class-file
    /// version of the main class, falling back to the `Build-Jdk-Spec`
    /// manifest attribute. `None` when neither indicator is usable.
    pub fn java_version(&self) -> Result<Option<u32>, String> {
        let archive = ZipArchive::new(&self.path);

        if let Some(major) = self.class_version(&archive)? {
            if major >= 49 {
                return Ok(Some(major - 44));
            }
        }

        Ok(self
            .manifest_version(&archive)?
            .filter(|version| *version >= MINIMUM_REQUIRED_JAVA))
    }

    fn class_version(&self, archive: &ZipArchive) -> Result<Option<u32>, String> {
        let Some(entry) = archive.entry(MAIN_CLASS_PATH)? else {
            return Ok(None);
        };
        let data = archive.entry_contents(&entry)?;
        if data.len() < 8 || data[..4] != CLASS_FILE_MAGIC {
            return Ok(None);
        }
        Ok(Some(((data[6] as u32) << 8) | data[7] as u32))
    }

    fn manifest_version(&self, archive: &ZipArchive) -> Result<Option<u32>, String> {
        let Some(entry) = archive.entry(MANIFEST_PATH)? else {
            return Ok(None);
        };
        let data = archive.entry_contents(&entry)?;
        let manifest = String::from_utf8_lossy(&data);
        Ok(manifest.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if key.trim() == "Build-Jdk-Spec" {
                value.trim().parse().ok()
            } else {
                None
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}