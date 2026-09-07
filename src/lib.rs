mod config;
mod constants;
mod download;
mod jar;
mod paths;
mod shell_env;
mod status;
mod version;
mod java;
mod zip;

use std::env;

use zed_extension_api::{
    self as zed,
    settings::LspSettings,
    Command, LanguageServerId, Result, Worktree,
};

use config::JvmOptions;
use constants::{LANGUAGE_SERVER_ID, LANGUAGE_SERVER_NAME, MINIMUM_REQUIRED_JAVA};
use download::BslJarDownloader;
use jar::BslJar;
use java::JavaRuntime;
use paths::JarRegistry;
use status::Status;

#[cfg(test)]
mod tests;

pub struct BslExtension {
    java_runtime: Option<JavaRuntime>,
}

impl BslExtension {
    pub fn new() -> Self {
        Self { java_runtime: None }
    }

    fn command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        if let Some(binary) = self.binary_settings(language_server_id, worktree)? {
            return Ok(binary);
        }

        let java_path = worktree
            .which("java")
            .ok_or_else(|| JavaRuntime::missing_message())?;
        let runtime = self.runtime_for(&java_path);

        let extension_dir = env::current_dir()
            .map_err(|error| format!("failed to determine the extension directory: {}", error))?;
        let jar_registry = JarRegistry::new(&worktree.shell_env(), &extension_dir);

        let jar = match jar_registry.existing() {
            Some(jar) => jar,
            None => {
                let destination = jar_registry.managed_path();
                BslJarDownloader::download_to(language_server_id, &destination)?;
                destination
            }
        };

        let status = Status::new(language_server_id);
        let required_version = match BslJar::new(&jar).java_version()? {
            Some(version) => version,
            None => {
                status
                    .failed("Failed to determine the required Java version from the downloaded jar");
                MINIMUM_REQUIRED_JAVA
            }
        };

        let installed_version = runtime.version().map_err(|error| {
            status.failed(&format!(
                "Failed to determine the installed Java version from `{}`",
                java_path
            ));
            error
        })?;

        if installed_version < required_version {
            return Err(format!(
                "{} requires Java {} or newer, but the Java runtime at `{}` is version {}. \
                 Please install Java {} (e.g. Temurin OpenJDK) and make sure it is on your `PATH`.",
                LANGUAGE_SERVER_NAME,
                required_version,
                java_path,
                installed_version,
                required_version,
            ));
        }

        let mut args = JvmOptions::from_env(&worktree.shell_env()).to_args();
        args.push("-jar".to_string());
        args.push(jar.to_string_lossy().into_owned());

        Ok(Command {
            command: java_path,
            args,
            env: Vec::new(),
        })
    }

    fn runtime_for(&mut self, java_path: &str) -> &mut JavaRuntime {
        if self
            .java_runtime
            .as_ref()
            .map(|runtime| runtime.path() != java_path)
            .unwrap_or(true)
        {
            self.java_runtime = Some(JavaRuntime::new(java_path));
        }
        self.java_runtime.as_mut().unwrap()
    }

    fn binary_settings(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Command>> {
        let settings = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree).ok();
        let Some(binary) = settings.and_then(|settings| settings.binary) else {
            return Ok(None);
        };
        let Some(command) = binary.path else {
            return Ok(None);
        };
        let env = binary
            .env
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<(String, String)>>();
        Ok(Some(Command {
            command,
            args: binary.arguments.unwrap_or_default(),
            env,
        }))
    }
}

impl zed::Extension for BslExtension {
    fn new() -> Self {
        Self::new()
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        self.command(language_server_id, worktree)
    }
}

zed::register_extension!(BslExtension);