use std::io::{self, IsTerminal, Write};

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
    let mut settings = ctx.settings.clone();
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
        return Ok(settings);
    }

    let settings_path = ctx.paths.settings_file(profile);
    if !io::stdin().is_terminal() {
        let missing = format_missing_fields(missing_client_id, missing_client_secret);
        return Err(AppError::Config(format!(
            "missing oauth {missing} in {}. run `gmail auth login` in an interactive terminal to be prompted, or add the values manually",
            settings_path.display(),
        )));
    }

    println!("OAuth client config is missing for profile `{profile}`.");
    println!("Settings will be saved to {}.", settings_path.display());

    if missing_client_id {
        settings.client_id = Some(prompt_required("OAuth client_id: ")?);
    }

    if missing_client_secret {
        settings.client_secret = Some(prompt_required("OAuth client_secret: ")?);
    }

    let default_redirect = settings.redirect_uri();
    let redirect_uri = prompt_optional(&format!("OAuth redirect_uri [{default_redirect}]: "))?;

    settings.redirect_uri = Some(if redirect_uri.is_empty() {
        default_redirect
    } else {
        redirect_uri
    });

    config::save_settings(&ctx.paths, profile, &settings)?;
    println!("Saved profile settings to {}.", settings_path.display());

    Ok(settings)
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
    let settings_path = ctx.paths.settings_file(profile);
    if !io::stdin().is_terminal() {
        return Err(AppError::Auth(format!(
            "{original_error}. add client_secret to {}",
            settings_path.display()
        )));
    }

    println!("Google requires a client_secret for this OAuth client.");
    let client_secret = prompt_required("OAuth client_secret: ")?;

    let mut updated = settings.clone();
    updated.client_secret = Some(client_secret);
    config::save_settings(&ctx.paths, profile, &updated)?;
    println!("Updated profile settings at {}.", settings_path.display());

    Ok(updated)
}
