pub fn value(env_vars: &[(String, String)], key: &str) -> Option<String> {
    env_vars
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}