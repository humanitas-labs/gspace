use std::fs;

use crate::config::AppPaths;
use crate::error::AppResult;

use super::TokenSet;

/// Persistence backend for a profile's OAuth token set.
pub trait TokenStore {
    /// Load the stored token set for a profile, or `None` if none exists.
    fn load(&self, profile: &str) -> AppResult<Option<TokenSet>>;
    /// Persist a token set for a profile.
    fn save(&self, profile: &str, token: &TokenSet) -> AppResult<()>;
    /// Remove any stored token set for a profile.
    fn clear(&self, profile: &str) -> AppResult<()>;
}

#[derive(Debug, Clone)]
pub struct FileTokenStore {
    paths: AppPaths,
}

impl FileTokenStore {
    /// Create a store that keeps per-profile token files under the given app paths.
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }
}

impl TokenStore for FileTokenStore {
    /// Read and deserialize the profile's token file, returning `None` when absent.
    fn load(&self, profile: &str) -> AppResult<Option<TokenSet>> {
        let path = self.paths.token_file(profile);
        if !path.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(path)?;
        let token = serde_json::from_str(&raw)?;
        Ok(Some(token))
    }

    /// Write the token file as pretty JSON, restricting it to owner-only (0600) on unix.
    fn save(&self, profile: &str, token: &TokenSet) -> AppResult<()> {
        let path = self.paths.token_file(profile);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let payload = serde_json::to_string_pretty(token)?;
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

    /// Delete the profile's token file if it exists.
    fn clear(&self, profile: &str) -> AppResult<()> {
        let path = self.paths.token_file(profile);
        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }
}
