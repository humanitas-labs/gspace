use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8787/callback";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl Settings {
    /// The OAuth client id, erroring if it is not configured.
    pub fn client_id(&self) -> AppResult<&str> {
        self.client_id.as_deref().ok_or_else(|| {
            AppError::Config(
                "missing oauth client_id. run `gmail auth login` to set the shared client in config.json, or add client_id to the profile json"
                    .to_string(),
            )
        })
    }

    /// The configured OAuth client secret, if any.
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// Whether this profile names its own OAuth client instead of the shared one.
    pub fn has_own_client(&self) -> bool {
        non_empty(&self.client_id).is_some()
    }

    /// These settings with any unset OAuth field filled from the shared app config.
    ///
    /// Each field falls back independently; a blank profile value counts as unset.
    pub fn merged_with(&self, shared: &AppConfig) -> Settings {
        let pick = |own: &Option<String>, fallback: &Option<String>| {
            non_empty(own).or_else(|| non_empty(fallback))
        };

        Settings {
            client_id: pick(&self.client_id, &shared.client_id),
            client_secret: pick(&self.client_secret, &shared.client_secret),
            redirect_uri: pick(&self.redirect_uri, &shared.redirect_uri),
            ..self.clone()
        }
    }

    /// The configured redirect URI, or the built-in loopback default.
    pub fn redirect_uri(&self) -> String {
        self.redirect_uri
            .clone()
            .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_string())
    }
}

/// The value if it is present and not blank.
fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

/// Load settings from `path`, returning defaults when the file is absent.
pub fn load(path: PathBuf) -> AppResult<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }

    let raw = fs::read_to_string(path)?;
    let settings = serde_json::from_str(&raw)?;
    Ok(settings)
}

/// Write settings as pretty JSON to `path`, restricting it to owner-only (0600) on unix.
pub fn save(path: PathBuf, settings: &Settings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = serde_json::to_string_pretty(settings)?;
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

    fn shared() -> AppConfig {
        AppConfig {
            client_id: Some("shared-id".to_string()),
            client_secret: Some("shared-secret".to_string()),
            redirect_uri: Some("http://127.0.0.1:9000/callback".to_string()),
            ..AppConfig::default()
        }
    }

    #[test]
    fn profile_without_client_inherits_shared() {
        let profile = Settings {
            sender_name: Some("Jane".to_string()),
            ..Settings::default()
        };
        let merged = profile.merged_with(&shared());
        assert_eq!(merged.client_id.as_deref(), Some("shared-id"));
        assert_eq!(merged.client_secret.as_deref(), Some("shared-secret"));
        assert_eq!(merged.redirect_uri(), "http://127.0.0.1:9000/callback");
        assert_eq!(merged.sender_name.as_deref(), Some("Jane"));
    }

    #[test]
    fn profile_values_override_shared() {
        let profile = Settings {
            client_id: Some("own-id".to_string()),
            client_secret: Some("own-secret".to_string()),
            ..Settings::default()
        };
        let merged = profile.merged_with(&shared());
        assert_eq!(merged.client_id.as_deref(), Some("own-id"));
        assert_eq!(merged.client_secret.as_deref(), Some("own-secret"));
        assert_eq!(merged.redirect_uri(), "http://127.0.0.1:9000/callback");
    }

    #[test]
    fn blank_profile_value_falls_back_to_shared() {
        let profile = Settings {
            client_id: Some("  ".to_string()),
            ..Settings::default()
        };
        let merged = profile.merged_with(&shared());
        assert_eq!(merged.client_id.as_deref(), Some("shared-id"));
        assert!(!profile.has_own_client());
    }

    #[test]
    fn missing_everywhere_still_errors() {
        let merged = Settings::default().merged_with(&AppConfig::default());
        assert!(merged.client_id().is_err());
        assert_eq!(merged.redirect_uri(), DEFAULT_REDIRECT_URI);
    }
}
