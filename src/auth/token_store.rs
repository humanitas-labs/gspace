use std::fs;

use crate::config::AppPaths;
use crate::error::AppResult;

use super::TokenSet;

pub trait TokenStore {
    fn load(&self, profile: &str) -> AppResult<Option<TokenSet>>;
    fn save(&self, profile: &str, token: &TokenSet) -> AppResult<()>;
    fn clear(&self, profile: &str) -> AppResult<()>;
}

#[derive(Debug, Clone)]
pub struct FileTokenStore {
    paths: AppPaths,
}

impl FileTokenStore {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }
}

impl TokenStore for FileTokenStore {
    fn load(&self, profile: &str) -> AppResult<Option<TokenSet>> {
        let path = self.paths.token_file(profile);
        if !path.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(path)?;
        let token = serde_json::from_str(&raw)?;
        Ok(Some(token))
    }

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

    fn clear(&self, profile: &str) -> AppResult<()> {
        let path = self.paths.token_file(profile);
        if path.exists() {
            fs::remove_file(path)?;
        }

        Ok(())
    }
}
