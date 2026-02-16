use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

const APP_DIR: &str = "gmail";

#[derive(Debug, Clone)]
pub struct AppPaths {
    config_dir: PathBuf,
    data_dir: PathBuf,
    profiles_dir: PathBuf,
    tokens_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> AppResult<Self> {
        let config_root = dirs::config_dir()
            .ok_or_else(|| AppError::Config("unable to resolve config directory".to_string()))?;
        let data_root = dirs::data_dir()
            .ok_or_else(|| AppError::Config("unable to resolve data directory".to_string()))?;

        let config_dir = config_root.join(APP_DIR);
        let data_dir = data_root.join(APP_DIR);
        let profiles_dir = config_dir.join("profiles");
        let tokens_dir = data_dir.join("tokens");

        fs::create_dir_all(&profiles_dir)?;
        fs::create_dir_all(&tokens_dir)?;

        Ok(Self {
            config_dir,
            data_dir,
            profiles_dir,
            tokens_dir,
        })
    }

    pub fn settings_file(&self, profile: &str) -> PathBuf {
        self.profiles_dir.join(format!("{profile}.json"))
    }

    pub fn token_file(&self, profile: &str) -> PathBuf {
        self.tokens_dir.join(format!("{profile}.json"))
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}
