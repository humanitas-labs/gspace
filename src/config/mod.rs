pub mod app_config;
pub mod paths;
pub mod profile;
pub mod settings;

pub use app_config::AppConfig;
pub use paths::AppPaths;
pub use profile::{PROFILE_ENV, resolve_profile};
pub use settings::Settings;

use std::path::PathBuf;

use crate::error::AppResult;

/// Load the top-level app config from `path`.
pub fn load_app_config(path: PathBuf) -> AppResult<AppConfig> {
    app_config::load(path)
}

/// Persist the top-level app config to `path`.
pub fn save_app_config(path: PathBuf, config: &AppConfig) -> AppResult<()> {
    app_config::save(path, config)
}

/// Load a profile's settings from its settings file.
pub fn load_settings(paths: &AppPaths, profile: &str) -> AppResult<Settings> {
    settings::load(paths.settings_file(profile))
}

/// Persist a profile's settings to its settings file.
pub fn save_settings(paths: &AppPaths, profile: &str, settings: &Settings) -> AppResult<()> {
    settings::save(paths.settings_file(profile), settings)
}
