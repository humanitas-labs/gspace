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

## 2026-06-30

### Stack

- [x] Add `gmail attachments ls|get <id>` to download message attachments.
      Motivation: CLI could see messages via `get`/`list` but had no way to pull
      attachments (hit while trying to fetch emailed PDFs). Adds a
      `full`-format fetch + MIME-tree walk (`list_attachments`) and
      `messages.attachments.get` byte download (`get_attachment`) in the API
      layer, plus an `attachments` subcommand mirroring the `label` shape.
      `get` supports `--out <dir>`, `--index <n>`, `--name <file>`; filenames are
      sanitized to `file_name()` to prevent path traversal.

### Testing Checklist

- [x] `cargo fmt && cargo check && cargo test` (added attachment-collection +
      base64url decode unit tests in `tests/api_client.rs`)
- [x] `cargo run -- attachments ls <id>` (live: messages with PDF attachments)
- [x] `cargo run -- attachments get <id> --out ~/Downloads/gmail-attachments`
      (live: 4 resumes downloaded, verified as valid PDFs, byte-exact sizes)

- [x] Make `gmail get <id>` show the full message body, not just the snippet.
      Motivation: `get` fetched `format=metadata` (headers + snippet only), so the
      actual email text was never exposed — same gap as attachments. Adds
      `get_msg_full` (fetches `format=full`) + a MIME-tree body extractor
      (`extract_body`) that prefers `text/plain` and falls back to tag-stripped
      `text/html`. `MessageView` gains a `body` field (surfaced in JSON too); text
      output prints headers then body, falling back to the snippet when no body
      part decodes.

- [x] `cargo fmt && cargo check && cargo test` (added text/plain-preference and
      html-fallback body extraction unit tests)
- [x] `cargo run -- get <id>` (live: read a long job-description email body in full)

- [x] Make `gmail get <id>` list attachments so their presence is visible.
      Motivation: `get` now fetches `format=full` but only showed body — a reader
      (incl. an agent) had no signal that a message carried attachments unless the
      body happened to mention them. `MessageView` gains an `attachments` array
      (populated via the existing `collect_attachments` walk, surfaced in JSON);
      text output prints a numbered attachment list with a `gmail attachments get`
      download hint.
- [x] `cargo fmt && cargo check && cargo test` (asserted metadata-format view has
      no attachments)
- [x] `cargo run -- get <id>` (live: 3 PDF attachments listed in text + JSON)

## 2026-09-26

- [x] Share one OAuth client across profiles via `config.json`, with per-field profile overrides; login saves prompted client values to the shared config and creates missing profile files; signature writes touch only the raw profile file.
- [x] `cargo fmt && cargo clippy --all-targets && cargo test` (76 passed; merge-fallback and legacy-config unit tests)
- [x] Live: migrated local config, token refresh for work + personal via shared client, `signature set` on a scratch profile wrote no OAuth fields

## Heap

- [ ] Add integration tests with mocked Gmail responses.
- [ ] Harden reply recipient inference and `References` handling.
- [ ] Add attachment filename/content-type override flags.
