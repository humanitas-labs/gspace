use crate::api::client::GmailClient;
use crate::auth::token_store::TokenStore;
use crate::auth::{AuthService, FileTokenStore};
use crate::config::{self, AppPaths, Settings};
use crate::error::{AppError, AppResult};
use crate::output::Output;

#[derive(Debug)]
pub struct AppContext {
    profile: String,
    profile_error: Option<String>,
    pub verbose: u8,
    pub paths: AppPaths,
    pub settings: Settings,
    pub token_store: FileTokenStore,
    pub gmail_client: GmailClient,
    pub output: Output,
}

impl AppContext {
    /// Resolve paths, settings, token store, client, and output mode into an app context.
    ///
    /// Profile resolution is deferred: an ambiguous result is captured rather
    /// than raised, so profile-management commands still run. Commands that act
    /// on a mailbox reach for [`AppContext::profile`], which surfaces the error.
    pub fn bootstrap(profile: Option<String>, json: bool, verbose: u8) -> AppResult<Self> {
        let paths = AppPaths::discover()?;
        let app_config = config::load_app_config(paths.config_file())?;
        let available = paths.list_profiles()?;
        let env_profile = std::env::var(config::PROFILE_ENV).ok();
        let (profile, profile_error) = match config::resolve_profile(
            profile.as_deref(),
            env_profile.as_deref(),
            app_config.default_profile.as_deref(),
            &available,
        ) {
            Ok(name) => (name, None),
            Err(AppError::Config(message)) => {
                (config::profile::FALLBACK_PROFILE.to_string(), Some(message))
            }
            Err(err) => return Err(err),
        };
        let settings = config::load_settings(&paths, &profile)?;
        let token_store = FileTokenStore::new(paths.clone());
        let gmail_client = GmailClient::new();
        let output = Output::new(json);

        Ok(Self {
            profile,
            profile_error,
            verbose,
            paths,
            settings,
            token_store,
            gmail_client,
            output,
        })
    }

    /// The resolved profile name, or the deferred ambiguity error when several
    /// profiles exist and none was selected.
    pub fn profile(&self) -> AppResult<&str> {
        match &self.profile_error {
            Some(message) => Err(AppError::Config(message.clone())),
            None => Ok(&self.profile),
        }
    }

    /// Return a valid access token, refreshing it if the stored one has expired.
    pub async fn access_token(&self) -> AppResult<String> {
        let profile = self.profile()?;
        let token = self.token_store.load(profile)?.ok_or_else(|| {
            AppError::InvalidInput("not logged in. run `gmail auth login`".to_string())
        })?;

        if token.is_expired(std::time::SystemTime::now()) {
            let refreshed =
                AuthService::refresh(profile, &self.settings, &self.token_store).await?;
            return Ok(refreshed.access_token);
        }

        Ok(token.access_token)
    }
}
