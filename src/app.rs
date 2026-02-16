use crate::cli::{Cli, Command};
use crate::commands;
use crate::context::AppContext;
use crate::error::AppResult;

pub async fn run(cli: Cli) -> AppResult<()> {
    let Cli {
        profile,
        json,
        verbose,
        command,
    } = cli;

    let ctx = AppContext::bootstrap(profile, json, verbose)?;

    match command {
        Command::Auth(args) => commands::auth::run(&ctx, args.command).await,
        Command::List(args) => commands::list::run(&ctx, args).await,
        Command::Send(args) => commands::send::run(&ctx, args).await,
        Command::Get(args) => commands::get::run(&ctx, args).await,
        Command::Label(args) => commands::label::run(&ctx, args.command).await,
    }
}
