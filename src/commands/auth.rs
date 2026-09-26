use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use crate::auth::AuthService;
use crate::cli::AuthCommand;
use crate::config::{self, Settings};
use crate::context::AppContext;
use crate::error::{AppError, AppResult};

/// Dispatch a `gmail auth` subcommand (login/status/logout) and emit its result.
pub async fn run(ctx: &AppContext, command: AuthCommand) -> AppResult<()> {
    match command {
        AuthCommand::Login => {
            let profile = ctx.profile()?;
            let settings = ensure_login_settings(ctx)?;
            let result = match AuthService::login(profile, &settings, &ctx.token_store).await {
                Ok(result) => result,
                Err(AppError::Auth(message)) if missing_client_secret_error(&message) => {
                    let settings = prompt_for_missing_client_secret(ctx, &settings, &message)?;
                    AuthService::login(profile, &settings, &ctx.token_store).await?
                }
                Err(err) => return Err(err),
            };
            ensure_profile_file(ctx, profile)?;

            let text = if let Some(email) = result.email.as_ref() {
                format!("{}: logged in as {}", result.profile, email)
            } else {
                format!("{}: {}", result.profile, result.note)
            };
            ctx.output.emit(&text, &result)
        }
        AuthCommand::Status => {
            let status = AuthService::status(ctx.profile()?, &ctx.token_store).await?;
            let text = if status.logged_in {
                let refresh_hint = status
                    .has_refresh_token
                    .map(|has| {
                        if has {
                            " (refresh available)"
                        } else {
                            " (no refresh token)"
                        }
                    })
                    .unwrap_or_default();
                format!(
                    "{}: logged in{}{}",
                    status.profile,
                    status
                        .email
                        .as_ref()
                        .map(|email| format!(" as {email}"))
                        .unwrap_or_default(),
                    refresh_hint,
                )
            } else {
                format!("{}: logged out", status.profile)
            };

            ctx.output.emit(&text, &status)
        }
        AuthCommand::Logout => {
            let status = AuthService::logout(ctx.profile()?, &ctx.token_store).await?;
            let text = format!("{}: logged out", status.profile);
            ctx.output.emit(&text, &status)
        }
    }
}

/// Ensure client_id/client_secret are set, prompting interactively and saving them when missing.
fn ensure_login_settings(ctx: &AppContext) -> AppResult<Settings> {
    let profile = ctx.profile()?;
    let settings = &ctx.settings;
    let missing_client_id = settings
        .client_id
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty);
    let missing_client_secret = settings
        .client_secret
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty);

    if !missing_client_id && !missing_client_secret {
        return Ok(settings.clone());
    }

    let target = OAuthTarget::resolve(ctx, profile)?;
    if !io::stdin().is_terminal() {
        let missing = format_missing_fields(missing_client_id, missing_client_secret);
        return Err(AppError::Config(format!(
            "missing oauth {missing} in {}. run `gmail auth login` in an interactive terminal to be prompted, or add the values manually",
            target.describe(),
        )));
    }

    println!("OAuth client config is missing for profile `{profile}`.");
    println!("Settings will be saved to {}.", target.describe());

    let client_id = missing_client_id
        .then(|| prompt_required("OAuth client_id: "))
        .transpose()?;
    let client_secret = missing_client_secret
        .then(|| prompt_required("OAuth client_secret: "))
        .transpose()?;

    let default_redirect = settings.redirect_uri();
    let redirect_uri = prompt_optional(&format!("OAuth redirect_uri [{default_redirect}]: "))?;
    let redirect_uri = if redirect_uri.is_empty() {
        default_redirect
    } else {
        redirect_uri
    };

    let settings = target.save(ctx, profile, client_id, client_secret, Some(redirect_uri))?;
    println!("Saved OAuth client settings to {}.", target.describe());

    Ok(settings)
}

/// Where prompted OAuth client values are persisted.
///
/// A profile that names its own `client_id` keeps its OAuth values in its own
/// settings file; every other profile shares the client in `config.json`.
struct OAuthTarget {
    own_client: bool,
    path: PathBuf,
}

impl OAuthTarget {
    /// Pick the target from the raw (unmerged) profile settings.
    fn resolve(ctx: &AppContext, profile: &str) -> AppResult<Self> {
        let own_client = config::load_settings(&ctx.paths, profile)?.has_own_client();
        let path = if own_client {
            ctx.paths.settings_file(profile)
        } else {
            ctx.paths.config_file()
        };
        Ok(Self { own_client, path })
    }

    /// Human-readable location, noting when the values are shared.
    fn describe(&self) -> String {
        if self.own_client {
            self.path.display().to_string()
        } else {
            format!("{} (shared by all profiles)", self.path.display())
        }
    }

    /// Write the given fields to the target and return the re-merged effective settings.
    fn save(
        &self,
        ctx: &AppContext,
        profile: &str,
        client_id: Option<String>,
        client_secret: Option<String>,
        redirect_uri: Option<String>,
    ) -> AppResult<Settings> {
        let mut profile_settings = config::load_settings(&ctx.paths, profile)?;
        let mut app_config = config::load_app_config(ctx.paths.config_file())?;

        if self.own_client {
            profile_settings.client_id = client_id.or(profile_settings.client_id);
            profile_settings.client_secret = client_secret.or(profile_settings.client_secret);
            profile_settings.redirect_uri = redirect_uri.or(profile_settings.redirect_uri);
            config::save_settings(&ctx.paths, profile, &profile_settings)?;
        } else {
            app_config.client_id = client_id.or(app_config.client_id);
            app_config.client_secret = client_secret.or(app_config.client_secret);
            app_config.redirect_uri = redirect_uri.or(app_config.redirect_uri);
            config::save_app_config(ctx.paths.config_file(), &app_config)?;
        }

        Ok(profile_settings.merged_with(&app_config))
    }
}

/// Create an empty settings file for `profile` if none exists, so a profile that
/// relies entirely on the shared client still shows up in `gmail profile list`.
fn ensure_profile_file(ctx: &AppContext, profile: &str) -> AppResult<()> {
    if ctx.paths.settings_file(profile).exists() {
        return Ok(());
    }
    config::save_settings(&ctx.paths, profile, &Settings::default())
}

/// Build a human-readable description of which OAuth fields are missing.
fn format_missing_fields(missing_client_id: bool, missing_client_secret: bool) -> String {
    match (missing_client_id, missing_client_secret) {
        (true, true) => "client_id and client_secret".to_string(),
        (true, false) => "client_id".to_string(),
        (false, true) => "client_secret".to_string(),
        (false, false) => "configuration".to_string(),
    }
}

/// Prompt repeatedly until the user enters a non-empty value.
fn prompt_required(prompt: &str) -> AppResult<String> {
    loop {
        let value = prompt_line(prompt)?;
        if !value.is_empty() {
            return Ok(value);
        }
        eprintln!("value is required");
    }
}

/// Prompt for a value, allowing an empty response.
fn prompt_optional(prompt: &str) -> AppResult<String> {
    prompt_line(prompt)
}

/// Write a prompt to stdout and read a single trimmed line from stdin.
fn prompt_line(prompt: &str) -> AppResult<String> {
    let mut stdout = io::stdout();
    write!(stdout, "{prompt}")?;
    stdout.flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

/// Whether an auth error message indicates a missing client secret.
fn missing_client_secret_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("client_secret is missing") || lower.contains("client secret is missing")
}

/// Interactively prompt for and persist a client secret after login failed for lack of one.
fn prompt_for_missing_client_secret(
    ctx: &AppContext,
    settings: &Settings,
    original_error: &str,
) -> AppResult<Settings> {
    if settings
        .client_secret
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return Err(AppError::Auth(original_error.to_string()));
    }

    let profile = ctx.profile()?;
    let target = OAuthTarget::resolve(ctx, profile)?;
    if !io::stdin().is_terminal() {
        return Err(AppError::Auth(format!(
            "{original_error}. add client_secret to {}",
            target.describe()
        )));
    }

    println!("Google requires a client_secret for this OAuth client.");
    let client_secret = prompt_required("OAuth client_secret: ")?;

    let updated = target.save(ctx, profile, None, Some(client_secret), None)?;
    println!("Updated OAuth client settings at {}.", target.describe());

    Ok(updated)
}
