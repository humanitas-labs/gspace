use crate::api::client::GmailClient;
use crate::auth::token_store::TokenStore;
use crate::auth::{AuthService, FileTokenStore};
use crate::config::{self, AppPaths, Settings};
use crate::error::{AppError, AppResult};
use crate::output::Output;

#[derive(Debug)]
pub struct AppContext {
    pub profile: String,
    pub verbose: u8,
    pub paths: AppPaths,
    pub settings: Settings,
    pub token_store: FileTokenStore,
    pub gmail_client: GmailClient,
    pub output: Output,
}

impl AppContext {
    pub fn bootstrap(profile: String, json: bool, verbose: u8) -> AppResult<Self> {
        let profile = config::resolve_profile(&profile);
        let paths = AppPaths::discover()?;
        let settings = config::load_settings(&paths, &profile)?;
        let token_store = FileTokenStore::new(paths.clone());
        let gmail_client = GmailClient::new();
        let output = Output::new(json);

        Ok(Self {
            profile,
            verbose,
            paths,
            settings,
            token_store,
            gmail_client,
            output,
        })
    }

    pub async fn access_token(&self) -> AppResult<String> {
        let token = self.token_store.load(&self.profile)?.ok_or_else(|| {
            AppError::InvalidInput("not logged in. run `gmail auth login`".to_string())
        })?;

        if token.is_expired(std::time::SystemTime::now()) {
            let refreshed =
                AuthService::refresh(&self.profile, &self.settings, &self.token_store).await?;
            return Ok(refreshed.access_token);
        }

        Ok(token.access_token)
    }
}
