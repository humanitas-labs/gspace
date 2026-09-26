use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// Top-level (profile-independent) app configuration, stored at `config.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Name of the profile to use when none is given via flag or environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    /// OAuth client id shared by every profile that does not set its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// OAuth client secret shared by every profile that does not set its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// OAuth redirect URI shared by every profile that does not set its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

/// Load app config from `path`, returning defaults when the file is absent.
pub fn load(path: PathBuf) -> AppResult<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let raw = fs::read_to_string(path)?;
    let config = serde_json::from_str(&raw)?;
    Ok(config)
}

/// Write app config as pretty JSON to `path`, restricting it to owner-only (0600) on unix.
pub fn save(path: PathBuf, config: &AppConfig) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = serde_json::to_string_pretty(config)?;
    fs::write(&path, payload)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_without_oauth_fields_loads() {
        let config: AppConfig = serde_json::from_str(r#"{"default_profile":"work"}"#).unwrap();
        assert_eq!(config.default_profile.as_deref(), Some("work"));
        assert!(config.client_id.is_none());
    }

    #[test]
    fn unset_fields_are_not_serialized() {
        let config = AppConfig {
            default_profile: Some("work".to_string()),
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, r#"{"default_profile":"work"}"#);
    }
}
