use clap::Parser;
use gmail::cli::{AuthCommand, Cli, Command};

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
