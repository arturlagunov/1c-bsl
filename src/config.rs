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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_default_heap_when_unset() {
        let options = JvmOptions::from_env(&[]);
        assert_eq!(options.to_args(), vec!["-Xmx4g".to_string()]);
    }

    #[test]
    fn parses_environment_override() {
        let env_vars = vec![(
            JAVA_OPTIONS_ENV.to_string(),
            "-Xmx2g -Xss4m".to_string(),
        )];
        let options = JvmOptions::from_env(&env_vars);
        assert_eq!(
            options.to_args(),
            vec!["-Xmx2g".to_string(), "-Xss4m".to_string()]
        );
    }
}