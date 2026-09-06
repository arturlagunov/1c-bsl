use crate::constants::{DEFAULT_JVM_OPTIONS, JAVA_OPTIONS_ENV};
use crate::shell_env;

/// JVM flags applied when launching the language server, overridable through
/// the `BSL_LANGUAGE_SERVER_JAVA_OPTS` environment variable.
pub struct JvmOptions {
    args: Vec<String>,
}

impl JvmOptions {
    pub fn from_env(env_vars: &[(String, String)]) -> Self {
        let args = shell_env::value(env_vars, JAVA_OPTIONS_ENV)
            .map(|options| options.split_whitespace().map(str::to_string).collect())
            .unwrap_or_else(|| {
                DEFAULT_JVM_OPTIONS
                    .iter()
                    .map(|option| option.to_string())
                    .collect()
            });
        Self { args }
    }

    pub fn to_args(&self) -> Vec<String> {
        self.args.clone()
    }
}