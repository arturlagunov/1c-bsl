use crate::version::{parse_output, parse_version};

#[test]
fn parses_versions() {
    assert_eq!(parse_version("21"), Some(21));
    assert_eq!(parse_version("21.0.2"), Some(21));
    assert_eq!(parse_version("17.0.1"), Some(17));
    assert_eq!(parse_version("1.8"), Some(8));
    assert_eq!(parse_version("25"), Some(25));
    assert_eq!(parse_version("unknown"), None);
}

#[test]
fn parses_real_output() {
    let output = std::process::Command::new("java")
        .args(["-XshowSettings:properties", "-version"])
        .output();
    let Ok(output) = output else {
        return;
    };
    let probe = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let version =
        parse_output(&probe).unwrap_or_else(|| panic!("no parseable version in:\n{}", probe));
    assert!(version >= 17, "installed Java ({}) must be at least 17", version);
}