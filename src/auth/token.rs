use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_unix: Option<u64>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl TokenSet {
    const EXPIRY_SKEW_SECS: u64 = 30;

    pub fn is_expired(&self, now: SystemTime) -> bool {
        let Some(expires_at) = self.expires_at_unix else {
            return false;
        };

        let Ok(duration) = now.duration_since(UNIX_EPOCH) else {
            return false;
        };

        duration.as_secs().saturating_add(Self::EXPIRY_SKEW_SECS) >= expires_at
    }

    pub fn expires_in_seconds(&self, now: SystemTime) -> Option<i64> {
        let expires_at = self.expires_at_unix? as i64;
        let now_secs = now.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
        Some(expires_at - now_secs)
    }

    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token.is_some()
    }
}
