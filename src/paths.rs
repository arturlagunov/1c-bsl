use std::path::{Path, PathBuf};

use crate::constants::{JAR_FILENAME, JAR_PATH_ENV, LANGUAGE_SERVER_NAME, MANAGED_JAR_DIRECTORY};
use crate::shell_env;

/// Knows where the BSL language server jar can live and resolves the first
/// existing copy in priority order:
///   1. the `BSL_LANGUAGE_SERVER` environment variable,
///   2. `~/.local/lib/bsl-language-server/`,
///   3. `$XDG_DATA_HOME/lib/bsl-language-server/`,
///   4. a previously downloaded copy in the extension directory.
pub struct JarRegistry {
    env_vars: Vec<(String, String)>,
    extension_dir: PathBuf,
}

impl JarRegistry {
    pub fn new(env_vars: &[(String, String)], extension_dir: &Path) -> Self {
        Self {
            env_vars: env_vars.to_vec(),
            extension_dir: extension_dir.to_path_buf(),
        }
    }

    /// The first candidate that already exists on disk, if any.
    pub fn existing(&self) -> Option<PathBuf> {
        self.candidates().into_iter().find(|path| path.is_file())
    }

    /// Where fresh downloads are placed inside the extension directory.
    pub fn managed_path(&self) -> PathBuf {
        self.extension_dir
            .join(MANAGED_JAR_DIRECTORY)
            .join(JAR_FILENAME)
    }

    fn candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(path) = shell_env::value(&self.env_vars, JAR_PATH_ENV) {
            candidates.push(PathBuf::from(path));
        }
        if let Some(home) = shell_env::value(&self.env_vars, "HOME") {
            candidates.push(
                PathBuf::from(home)
                    .join(".local/lib")
                    .join(LANGUAGE_SERVER_NAME)
                    .join(JAR_FILENAME),
            );
        }
        if let Some(data_home) = shell_env::value(&self.env_vars, "XDG_DATA_HOME") {
            candidates.push(
                PathBuf::from(data_home)
                    .join("lib")
                    .join(LANGUAGE_SERVER_NAME)
                    .join(JAR_FILENAME),
            );
        }

        candidates.push(self.managed_path());
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn env(home: &str, data_home: &str) -> Vec<(String, String)> {
        vec![
            ("BSL_LANGUAGE_SERVER".to_string(), "/explicit.jar".to_string()),
            ("HOME".to_string(), home.to_string()),
            ("XDG_DATA_HOME".to_string(), data_home.to_string()),
        ]
    }

    #[test]
    fn priority_order() {
        let registry =
            JarRegistry::new(&env("/home/test", "/home/test/.data"), Path::new("/ext"));

        let candidates = registry.candidates();
        assert_eq!(candidates[0], PathBuf::from("/explicit.jar"));
        assert_eq!(
            candidates[1],
            PathBuf::from("/home/test/.local/lib/bsl-language-server/bsl-language-server.jar")
        );
        assert_eq!(
            candidates[2],
            PathBuf::from("/home/test/.data/lib/bsl-language-server/bsl-language-server.jar")
        );
        assert_eq!(
            candidates[3],
            PathBuf::from("/ext/binaries/bsl-language-server.jar")
        );
        assert_eq!(
            registry.managed_path(),
            PathBuf::from("/ext/binaries/bsl-language-server.jar")
        );
    }

    #[test]
    fn skips_missing() {
        let base =
            std::env::temp_dir().join(format!("zed-bsl-paths-test-{}", std::process::id()));
        let extension_dir = base.join("ext");
        fs::create_dir_all(extension_dir.join(MANAGED_JAR_DIRECTORY)).unwrap();
        fs::write(
            extension_dir.join(MANAGED_JAR_DIRECTORY).join(JAR_FILENAME),
            b"fake jar",
        )
        .unwrap();

        let registry = JarRegistry::new(
            &env(
                &base.join("missing-home").to_string_lossy(),
                &base.join("missing-data").to_string_lossy(),
            ),
            &extension_dir,
        );

        assert_eq!(
            registry.existing(),
            Some(extension_dir.join(MANAGED_JAR_DIRECTORY).join(JAR_FILENAME))
        );

        fs::remove_dir_all(&base).ok();
    }
}