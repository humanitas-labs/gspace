use serde_json::json;

use crate::cli::ProfileCommand;
use crate::config;
use crate::context::AppContext;
use crate::error::{AppError, AppResult};

/// Dispatch a `gmail profile` subcommand (list/use/show) and emit its result.
pub async fn run(ctx: &AppContext, command: ProfileCommand) -> AppResult<()> {
    match command {
        ProfileCommand::List => list(ctx),
        ProfileCommand::Use { name } => use_profile(ctx, &name),
        ProfileCommand::Show => show(ctx),
    }
}

/// List every profile on disk, marking the configured default.
fn list(ctx: &AppContext) -> AppResult<()> {
    let profiles = ctx.paths.list_profiles()?;
    let default = config::load_app_config(ctx.paths.config_file())?.default_profile;

    if profiles.is_empty() {
        return ctx.output.emit(
            "no profiles configured. run `gmail auth login` to create one",
            &json!({ "profiles": [], "default": default }),
        );
    }

    let lines: Vec<String> = profiles
        .iter()
        .map(|name| {
            if Some(name) == default.as_ref() {
                format!("* {name} (default)")
            } else {
                format!("  {name}")
            }
        })
        .collect();

    ctx.output.emit(
        &lines.join("\n"),
        &json!({ "profiles": profiles, "default": default }),
    )
}

/// Set the default profile, verifying it exists first.
fn use_profile(ctx: &AppContext, name: &str) -> AppResult<()> {
    let profiles = ctx.paths.list_profiles()?;
    if !profiles.iter().any(|profile| profile == name) {
        let available = if profiles.is_empty() {
            "(none)".to_string()
        } else {
            profiles.join(", ")
        };
        return Err(AppError::InvalidInput(format!(
            "no profile named `{name}`. available: {available}"
        )));
    }

    let mut app_config = config::load_app_config(ctx.paths.config_file())?;
    app_config.default_profile = Some(name.to_string());
    config::save_app_config(ctx.paths.config_file(), &app_config)?;

    ctx.output.emit(
        &format!("default profile set to `{name}`"),
        &json!({ "default": name }),
    )
}

/// Show the profile resolved for this invocation, or the ambiguity to resolve.
fn show(ctx: &AppContext) -> AppResult<()> {
    match ctx.profile() {
        Ok(profile) => ctx.output.emit(
            &format!("resolved profile: {profile}"),
            &json!({ "profile": profile }),
        ),
        Err(_) => {
            let profiles = ctx.paths.list_profiles()?;
            ctx.output.emit(
                &format!(
                    "no default profile set. profiles: {}. run `gmail profile use <name>`",
                    profiles.join(", ")
                ),
                &json!({ "profile": null, "profiles": profiles }),
            )
        }
    }
}
