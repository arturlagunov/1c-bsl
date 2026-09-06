use crate::config::JvmOptions;
use crate::constants::JAVA_OPTIONS_ENV;

#[test]
fn uses_default() {
    let options = JvmOptions::from_env(&[]);
    assert_eq!(options.to_args(), vec!["-Xmx4g".to_string()]);
}

#[test]
fn parses_override() {
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