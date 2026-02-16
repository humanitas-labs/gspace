use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "gmail", version, about = "Gmail command line interface")]
pub struct Cli {
    #[arg(
        long,
        global = true,
        default_value = "default",
        help = "Profile name to use"
    )]
    pub profile: String,
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
    List(ListArgs),
    Send(SendArgs),
    Get(GetArgs),
    Label(LabelArgs),
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
}

#[derive(Debug, Args)]
pub struct GetArgs {
    #[arg(help = "Gmail message id")]
    pub id: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_auth_login() {
        let cli = Cli::try_parse_from(["gmail", "auth", "login"]).expect("cli parse should work");
        match cli.command {
            Command::Auth(auth) => assert!(matches!(auth.command, AuthCommand::Login)),
            _ => panic!("expected auth command"),
        }
    }

    #[test]
    fn parses_get() {
        let cli = Cli::try_parse_from(["gmail", "get", "abc123"]).expect("cli parse should work");
        match cli.command {
            Command::Get(get) => assert_eq!(get.id, "abc123"),
            _ => panic!("expected get command"),
        }
    }

    #[test]
    fn parses_send() {
        let cli = Cli::try_parse_from([
            "gmail",
            "send",
            "--to",
            "dev@example.com",
            "--subject",
            "hi",
            "--body",
            "hello",
            "--attach",
            "a.txt",
            "--attach",
            "b.txt",
        ])
        .expect("cli parse should work");
        match cli.command {
            Command::Send(send) => {
                assert_eq!(send.to, ["dev@example.com"]);
                assert_eq!(send.subject.as_deref(), Some("hi"));
                assert_eq!(send.body.as_deref(), Some("hello"));
                assert_eq!(send.attach.len(), 2);
            }
            _ => panic!("expected send command"),
        }
    }

    #[test]
    fn parses_list() {
        let cli = Cli::try_parse_from([
            "gmail", "list", "--inbox", "--limit", "3", "--q", "from:foo",
        ])
        .expect("cli parse should work");
        match cli.command {
            Command::List(list) => {
                assert!(list.inbox);
                assert_eq!(list.limit, 3);
                assert_eq!(list.q.as_deref(), Some("from:foo"));
            }
            _ => panic!("expected list command"),
        }
    }
}
