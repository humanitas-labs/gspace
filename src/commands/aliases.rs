use crate::cli::AliasesCommand;
use crate::context::AppContext;
use crate::error::AppResult;
use crate::output::OutputMode;

/// Dispatch a `gmail aliases` subcommand (ls).
pub async fn run(ctx: &AppContext, command: AliasesCommand) -> AppResult<()> {
    match command {
        AliasesCommand::Ls => ls(ctx).await,
    }
}

/// List the account's send-as aliases with their verification state.
async fn ls(ctx: &AppContext) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let aliases = ctx.gmail_client.list_send_as(&access_token).await?;

    if ctx.output.mode() == OutputMode::Text {
        if aliases.is_empty() {
            println!("0 send-as aliases");
            return Ok(());
        }

        for (index, alias) in aliases.iter().enumerate() {
            let mut flags = Vec::new();
            if alias.is_primary {
                flags.push("primary".to_string());
            }
            if alias.is_default {
                flags.push("default".to_string());
            }
            if !alias.is_sendable() {
                let status = alias.verification_status.as_deref().unwrap_or("unknown");
                flags.push(format!("unverified: {status}"));
            }

            let name = alias
                .display_name
                .as_deref()
                .map(|name| format!(" \"{name}\""))
                .unwrap_or_default();
            let flags = if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            };

            println!("{}. {}{}{}", index + 1, alias.email, name, flags);
        }

        return Ok(());
    }

    let text = format!("{} send-as aliases", aliases.len());
    ctx.output.emit(&text, &aliases)
}
