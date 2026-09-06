pub const LANGUAGE_SERVER_ID: &str = "bsl";
pub const LANGUAGE_SERVER_NAME: &str = "bsl-language-server";
pub const BSL_REPOSITORY: &str = "1c-syntax/bsl-language-server";
pub const JAR_FILENAME: &str = "bsl-language-server.jar";
pub const JAR_PATH_ENV: &str = "BSL_LANGUAGE_SERVER";
pub const JAVA_OPTIONS_ENV: &str = "BSL_LANGUAGE_SERVER_JAVA_OPTS";
pub const MANAGED_JAR_DIRECTORY: &str = "binaries";
pub const MANIFEST_PATH: &str = "META-INF/MANIFEST.MF";
pub const MAIN_CLASS_PATH: &str =
    "BOOT-INF/classes/com/github/_1c_syntax/bsl/languageserver/MainApplication.class";
pub const MINIMUM_REQUIRED_JAVA: u32 = 17;
pub const DEFAULT_JVM_OPTIONS: &[&str] = &["-Xmx4g"];