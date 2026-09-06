/// Parses a `java.specification.version` value such as `21`, `17.0.1` or the
/// legacy `1.8` into an integer Java version (8, 17, 21, ...).
pub fn parse_version(value: &str) -> Option<u32> {
    let value = value.trim();
    if let Some(value) = value.strip_prefix("1.") {
        return value.split('.').next().and_then(|part| part.parse().ok());
    }
    value.split('.').next().and_then(|part| part.parse().ok())
}

/// Extracts the Java specification version from the combined stderr + stdout
/// output of `java -XshowSettings:properties -version`.
pub fn parse_output(probe: &str) -> Option<u32> {
    probe.lines().find_map(|line| {
        if line.contains("java.specification.version") {
            line.split('=').nth(1).map(str::trim).and_then(parse_version)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let version = parse_output(&probe)
            .unwrap_or_else(|| panic!("no parseable version in:\n{}", probe));
        assert!(version >= 17, "installed Java ({}) must be at least 17", version);
    }
}
