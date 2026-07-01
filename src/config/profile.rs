/// Normalize a requested profile name, falling back to `default` when blank.
pub fn resolve_profile(requested: &str) -> String {
    let trimmed = requested.trim();
    if trimmed.is_empty() {
        return "default".to_string();
    }

    trimmed.to_string()
}
