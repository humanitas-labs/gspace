use std::fs;

use serde_json::json;

use crate::cli::SignatureCommand;
use crate::config;
use crate::context::AppContext;
use crate::error::AppResult;

/// Dispatch a `gmail signature` subcommand (show/set/set-file/clear).
pub async fn run(ctx: &AppContext, command: SignatureCommand) -> AppResult<()> {
    match command {
        SignatureCommand::Show => show(ctx),
        SignatureCommand::Set { text } => set(ctx, text),
        SignatureCommand::SetFile { path } => set(ctx, fs::read_to_string(path)?),
        SignatureCommand::Clear => clear(ctx),
    }
}

/// Print the active profile's signature, or a note when none is set.
fn show(ctx: &AppContext) -> AppResult<()> {
    match ctx
        .settings
        .signature
        .as_deref()
        .filter(|sig| !sig.trim().is_empty())
    {
        Some(signature) => ctx
            .output
            .emit(signature, &json!({ "signature": signature })),
        None => ctx.output.emit(
            "no signature set. set one with `gmail signature set \"...\"`",
            &json!({ "signature": null }),
        ),
    }
}

/// Persist a signature to the active profile's settings file.
fn set(ctx: &AppContext, text: String) -> AppResult<()> {
    let signature = text.trim_matches(['\r', '\n']).to_string();
    let profile = ctx.profile()?;

    let mut settings = ctx.settings.clone();
    settings.signature = Some(signature.clone());
    config::save_settings(&ctx.paths, profile, &settings)?;

    ctx.output.emit(
        &format!("signature set for profile `{profile}`:\n{signature}"),
        &json!({ "profile": profile, "signature": signature }),
    )
}

/// Remove the signature from the active profile's settings file.
fn clear(ctx: &AppContext) -> AppResult<()> {
    let profile = ctx.profile()?;

    let mut settings = ctx.settings.clone();
    settings.signature = None;
    config::save_settings(&ctx.paths, profile, &settings)?;

    ctx.output.emit(
        &format!("signature cleared for profile `{profile}`"),
        &json!({ "profile": profile, "signature": null }),
    )
}
