use crate::error::{AppError, AppResult};

use super::token::TokenSet;
use super::token_store::TokenStore;

#[derive(Debug, Default)]
pub struct KeyringTokenStore;

impl TokenStore for KeyringTokenStore {
    fn load(&self, _profile: &str) -> AppResult<Option<TokenSet>> {
        Err(AppError::NotImplemented("keyring token store"))
    }

    fn save(&self, _profile: &str, _token: &TokenSet) -> AppResult<()> {
        Err(AppError::NotImplemented("keyring token store"))
    }

    fn clear(&self, _profile: &str) -> AppResult<()> {
        Err(AppError::NotImplemented("keyring token store"))
    }
}
