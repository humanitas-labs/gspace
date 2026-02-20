# TODO

## 2026-02-16

- [x] Agree CLI shape: `gmail auth login|status|logout`, `gmail send ...`, `gmail get <id>`, `gmail label ...`.
- [x] Init Rust crate and scaffold module architecture for Gmail CLI.
- [x] Add architecture docs and run baseline checks.
- [x] Rename module namespace from `gmail_api` to `api`.
- [x] Wire real OAuth login flow (browser auth code + PKCE) and token refresh.
- [x] Prompt for missing OAuth client settings during `auth login` and persist them.
- [x] Retry `auth login` with prompted `client_secret` when Google requires it.
- [x] Prompt for both `client_id` and `client_secret` up front when either is missing.
- [x] Implement API core + real `gmail get <id>` command path.
- [x] Rename command from `show` to `get`.
- [x] Add real `gmail list` command with inbox/query/limit support.
- [x] Add Gmail-style snippet preview to `gmail list` text output.
- [x] Decode common HTML entities in list preview snippets.
- [x] Replace custom HTML decoding with `html-escape` crate in preview formatting.
- [x] Implement real `gmail send` via Gmail API `messages.send`.
- [x] Send live test message to `jane@example.com`.
- [x] Implement `gmail label ls|add|rm` against Gmail API labels and message modify endpoints.
- [x] Add `gmail send --reply <id> --draft-file <path>` with thread-aware reply headers.
- [x] Add repeatable `--attach <path>` support for sending attachments.
- [x] Improve `gmail label ls` text output to print formatted labels without `--json`.

### Testing Checklist

- [x] `cargo fmt`
- [x] `cargo check`
- [x] `cargo test`
- [x] `cargo run -- auth login` (verified missing config error path)
- [x] `cargo run -- auth status`
- [x] `cargo run -- auth login` (verified non-interactive prompt fallback message)
- [x] `cargo fmt && cargo check && cargo test` after client_secret retry prompt
- [x] `cargo fmt && cargo check && cargo test` after up-front client_secret prompt update
- [x] `cargo run -- auth login` (verified successful login path)
- [x] `cargo run -- auth status` (verified logged-in state)
- [x] `cargo run -- get foo` (verified real Gmail API error mapping)
- [x] `cargo run -- list --inbox --limit 3` (verified inbox listing path)
- [x] `cargo run -- list --inbox --limit 3` (verified preview line output)
- [x] `cargo run -- list --inbox --limit 3` (verified HTML entity decoding)
- [x] `cargo fmt && cargo check && cargo test` after html-escape integration
- [x] `cargo run -- send --to jane@example.com --subject "gmail-cli send test" --body "..."`
- [x] `cargo run -- --json label ls`
- [x] `cargo run -- label ls` (verified formatted text output)
- [x] `cargo run -- label add 19c6880b2c6d1ea7 Invoices && cargo run -- label rm 19c6880b2c6d1ea7 Invoices`
- [x] `cargo run -- send --to jane@example.com --subject "gmail-cli attachment test" --body "..." --attach /tmp/gmail-cli-attach-test.txt`
- [x] `cargo run -- send --reply 19c6880b2c6d1ea7 --draft-file /tmp/gmail-cli-reply-draft.txt --to jane@example.com`

### Status

- Core Gmail CLI scope for this phase is complete.

### Optional Follow-ups

- Package/install flow (`cargo install --path .`) and release automation.
- Richer list/get formatting options (columns, pager, compact mode).
- More advanced attachment controls (display name/content-type overrides).

## 2026-02-20

### Stack

- [x] Implement markdown-to-HTML default `gmail send` body handling.
- [x] Consolidate tests under `tests/` and remove inline `src/` test modules.
- [x] Adjust HTML template to normal Gmail-like layout (remove centered card).
- [x] Set explicit `From` header with display name/email identity.
- [x] Bump CLI patch version to `0.1.1` and reinstall globally.

### Heap

- [ ] Evaluate richer text/plain fallback strategy for HTML emails.
