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

/// Write app config as pretty JSON to `path`.
pub fn save(path: PathBuf, config: &AppConfig) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = serde_json::to_string_pretty(config)?;
    fs::write(&path, payload)?;
    Ok(())
}
