use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8787/callback";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub sender_name: Option<String>,
}

impl Settings {
    pub fn client_id(&self) -> AppResult<&str> {
        self.client_id.as_deref().ok_or_else(|| {
            AppError::Config(
                "missing oauth client_id in profile settings. add it to your profile json"
                    .to_string(),
            )
        })
    }

    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    pub fn redirect_uri(&self) -> String {
        self.redirect_uri
            .clone()
            .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_string())
    }
}

pub fn load(path: PathBuf) -> AppResult<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }

    let raw = fs::read_to_string(path)?;
    let settings = serde_json::from_str(&raw)?;
    Ok(settings)
}

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
