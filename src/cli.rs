use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "gmail", version, about = "Gmail command line interface")]
pub struct Cli {
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
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Auth(AuthArgs),
    Profile(ProfileArgs),
    Signature(SignatureArgs),
    List(ListArgs),
    Send(SendArgs),
    Get(GetArgs),
    Label(LabelArgs),
    Attachments(AttachmentsArgs),
    Aliases(AliasesArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, default_value_t = 10, help = "Maximum messages to return")]
    pub limit: u32,
    #[arg(long, help = "Restrict to inbox messages")]
    pub inbox: bool,
    #[arg(long, help = "Gmail search query")]
    pub q: Option<String>,
}

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Login,
    Status,
    Logout,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    /// List profiles and show which is the default
    List,
    /// Set the default profile used when none is passed
    Use {
        /// Name of an existing profile
        name: String,
    },
    /// Show the profile that resolves for this invocation
    Show,
}

#[derive(Debug, Args)]
pub struct SignatureArgs {
    #[command(subcommand)]
    pub command: SignatureCommand,
}

#[derive(Debug, Subcommand)]
pub enum SignatureCommand {
    /// Show the active profile's signature
    Show,
    /// Set the active profile's signature (use literal newlines for multiple lines)
    Set {
        /// Signature text (markdown); multi-line supported
        text: String,
    },
    /// Set the signature from a file
    SetFile {
        /// Path to a file containing the signature
        path: PathBuf,
    },
    /// Remove the active profile's signature
    Clear,
}

#[derive(Debug, Args)]
pub struct SendArgs {
    #[arg(long, value_delimiter = ',', num_args = 1.., help = "Recipient addresses")]
    pub to: Vec<String>,
    #[arg(long, value_delimiter = ',', num_args = 1.., help = "CC addresses")]
    pub cc: Vec<String>,
    #[arg(long, value_delimiter = ',', num_args = 1.., help = "BCC addresses")]
    pub bcc: Vec<String>,
    #[arg(long, visible_alias = "subj", help = "Email subject")]
    pub subject: Option<String>,
    #[arg(long, help = "Inline body text")]
    pub body: Option<String>,
    #[arg(long, help = "Read body from file")]
    pub body_file: Option<PathBuf>,
    #[arg(long, help = "Read draft body from file")]
    pub draft_file: Option<PathBuf>,
    #[arg(long, help = "Read body from stdin")]
    pub stdin: bool,
    #[arg(long, help = "Reply to an existing message id")]
    pub reply: Option<String>,
    #[arg(long, action = ArgAction::Append, help = "Attach file (repeatable)")]
    pub attach: Vec<PathBuf>,
    #[arg(
        long,
        help = "Send from this address (must be a verified send-as alias; see `gmail aliases ls`)"
    )]
    pub from: Option<String>,
    #[arg(
        long,
        conflicts_with = "no_signature",
        help = "Override the profile signature for this send (markdown)"
    )]
    pub signature: Option<String>,
    #[arg(long, help = "Do not append the profile signature to this send")]
    pub no_signature: bool,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    #[arg(help = "Gmail message id")]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct AttachmentsArgs {
    #[command(subcommand)]
    pub command: AttachmentsCommand,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentsCommand {
    #[command(visible_alias = "list")]
    Ls(AttachmentsLsArgs),
    Get(AttachmentsGetArgs),
}

#[derive(Debug, Args)]
pub struct AttachmentsLsArgs {
    #[arg(help = "Gmail message id")]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct AttachmentsGetArgs {
    #[arg(help = "Gmail message id")]
    pub id: String,
    #[arg(
        long,
        default_value = ".",
        help = "Directory to write attachments into (created if missing)"
    )]
    pub out: PathBuf,
    #[arg(
        long,
        conflicts_with = "name",
        help = "Only download the attachment at this 1-based index"
    )]
    pub index: Option<usize>,
    #[arg(long, help = "Only download attachments matching this filename")]
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct AliasesArgs {
    #[command(subcommand)]
    pub command: AliasesCommand,
}

#[derive(Debug, Subcommand)]
pub enum AliasesCommand {
    #[command(visible_alias = "list")]
    Ls,
}

#[derive(Debug, Args)]
pub struct LabelArgs {
    #[command(subcommand)]
    pub command: LabelCommand,
}

#[derive(Debug, Subcommand)]
pub enum LabelCommand {
    Ls,
    Add(LabelMutateArgs),
    Rm(LabelMutateArgs),
}

#[derive(Debug, Args)]
pub struct LabelMutateArgs {
    #[arg(help = "Gmail message id")]
    pub id: String,
    #[arg(required = true, num_args = 1.., help = "Labels to mutate")]
    pub labels: Vec<String>,
}
