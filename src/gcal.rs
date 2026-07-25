use clap::{ArgAction, Args, Parser, Subcommand};

use crate::commands::calendar;
use crate::context::AppContext;
use crate::error::AppResult;

#[derive(Debug, Parser)]
#[command(name = "gcal", version, about = "Google Calendar command line interface")]
pub struct GcalCli {
    #[arg(
        long,
        global = true,
        help = "Profile name to use (overrides GMAIL_PROFILE and the configured default)"
    )]
    pub profile: Option<String>,
    #[arg(long, global = true, help = "Emit JSON output")]
    pub json: bool,
    #[arg(short = 'v', long, global = true, action = ArgAction::Count, help = "Verbose logging")]
    pub verbose: u8,
    #[command(subcommand)]
    pub command: GcalCommand,
}

#[derive(Debug, Subcommand)]
pub enum GcalCommand {
    /// Create an event, optionally with a Google Meet link
    Add(GcalAddArgs),
    /// List upcoming events
    List(GcalListArgs),
    /// Delete an event, sending cancellations to attendees
    Rm(GcalRmArgs),
}

#[derive(Debug, Args)]
pub struct GcalAddArgs {
    #[arg(long, help = "Event title")]
    pub title: String,
    #[arg(
        long,
        help = "Start time (RFC 3339, `YYYY-MM-DD HH:MM`, `today HH:MM`, `tomorrow HH:MM`)"
    )]
    pub start: String,
    #[arg(long, conflicts_with = "duration", help = "End time (same forms as --start)")]
    pub end: Option<String>,
    #[arg(long, default_value_t = 30, help = "Duration in minutes when --end is not given")]
    pub duration: u32,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Attendee emails, comma-separated; each receives a Google invite"
    )]
    pub attendees: Vec<String>,
    #[arg(long, help = "Attach a Google Meet conference to the event")]
    pub meet: bool,
    #[arg(long, help = "Event location")]
    pub location: Option<String>,
    #[arg(long, help = "Event description")]
    pub notes: Option<String>,
    #[arg(long, default_value = "primary", help = "Calendar id to create the event on")]
    pub calendar: String,
}

#[derive(Debug, Args)]
pub struct GcalListArgs {
    #[arg(long, conflicts_with_all = ["tomorrow", "week", "from", "to"], help = "Only today's events")]
    pub today: bool,
    #[arg(long, conflicts_with_all = ["week", "from", "to"], help = "Only tomorrow's events")]
    pub tomorrow: bool,
    #[arg(long, conflicts_with_all = ["from", "to"], help = "The next 7 days (the default)")]
    pub week: bool,
    #[arg(long, requires = "to", help = "Window start (datetime or `YYYY-MM-DD`)")]
    pub from: Option<String>,
    #[arg(long, requires = "from", help = "Window end (datetime or `YYYY-MM-DD`)")]
    pub to: Option<String>,
    #[arg(long, default_value_t = 20, help = "Maximum events to return")]
    pub limit: u32,
    #[arg(long, default_value = "primary", help = "Calendar id to list")]
    pub calendar: String,
}

#[derive(Debug, Args)]
pub struct GcalRmArgs {
    #[arg(help = "Event id (from `gcal add` or `gcal list`)")]
    pub event_id: String,
    #[arg(long, default_value = "primary", help = "Calendar id the event lives on")]
    pub calendar: String,
}

/// Bootstrap the shared app context and dispatch the parsed gcal command.
pub async fn run(cli: GcalCli) -> AppResult<()> {
    let GcalCli {
        profile,
        json,
        verbose,
        command,
    } = cli;

    let ctx = AppContext::bootstrap(profile, json, verbose)?;

    match command {
        GcalCommand::Add(args) => calendar::add::run(&ctx, args).await,
        GcalCommand::List(args) => calendar::list::run(&ctx, args).await,
        GcalCommand::Rm(args) => calendar::rm::run(&ctx, args).await,
    }
}
