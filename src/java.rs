use zed_extension_api::{process::Command as ProcessCommand, Result};

use crate::constants::LANGUAGE_SERVER_NAME;

pub struct JavaRuntime {
    path: String,
    version: Option<u32>,
}

impl JavaRuntime {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            version: None,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// Major version of the installed runtime, determined by running
    /// `java -XshowSettings:properties -version` and cached for this instance.
    pub fn version(&mut self) -> Result<u32> {
        if let Some(version) = self.version {
            return Ok(version);
        }

        let output = ProcessCommand::new(self.path.as_str())
            .args(["-XshowSettings:properties", "-version"])
            .output()
            .map_err(|error| format!("failed to run `{} -version`: {}", self.path, error))?;

        let probe = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );

        let version = crate::version::parse_output(&probe).ok_or_else(|| {
            format!(
                "failed to determine the installed Java version from `java` at {}",
                self.path
            )
        })?;

        self.version = Some(version);
        Ok(version)
    }

    /// Message shown when no `java` executable can be found on the `PATH`.
    pub fn missing_message() -> String {
        format!(
            "A Java runtime is required to run the official {} (a Java application), but `java` \
             was not found on your PATH. Install Java 21 or newer (e.g. Temurin OpenJDK: \
             https://adoptium.net) and try again.",
            LANGUAGE_SERVER_NAME
        )
    }
}
