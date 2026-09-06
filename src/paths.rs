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

    pub(crate) fn candidates(&self) -> Vec<PathBuf> {
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