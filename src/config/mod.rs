pub mod paths;
pub mod profile;
pub mod settings;

pub use paths::AppPaths;
pub use profile::resolve_profile;
pub use settings::Settings;

use crate::error::AppResult;

/// Load a profile's settings from its settings file.
pub fn load_settings(paths: &AppPaths, profile: &str) -> AppResult<Settings> {
    settings::load(paths.settings_file(profile))
}

/// Persist a profile's settings to its settings file.
pub fn save_settings(paths: &AppPaths, profile: &str, settings: &Settings) -> AppResult<()> {
    settings::save(paths.settings_file(profile), settings)
}
