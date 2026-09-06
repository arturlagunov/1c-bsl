use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::{JAR_FILENAME, MANAGED_JAR_DIRECTORY};
use crate::paths::JarRegistry;

fn env(home: &str, data_home: &str) -> Vec<(String, String)> {
    vec![
        ("BSL_LANGUAGE_SERVER".to_string(), "/explicit.jar".to_string()),
        ("HOME".to_string(), home.to_string()),
        ("XDG_DATA_HOME".to_string(), data_home.to_string()),
    ]
}

#[test]
fn priority_order() {
    let registry = JarRegistry::new(&env("/home/test", "/home/test/.data"), Path::new("/ext"));

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
    let base = std::env::temp_dir().join(format!("zed-bsl-paths-test-{}", std::process::id()));
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