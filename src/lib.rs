pub mod api;
pub mod app;
pub mod auth;
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod error;
pub mod mail;
pub mod output;

use cli::Cli;
use error::AppResult;

/// Entry point that runs the parsed CLI.
pub async fn run(cli: Cli) -> AppResult<()> {
    app::run(cli).await
}
