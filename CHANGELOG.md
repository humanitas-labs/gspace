# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.

## [0.5.0] - 2026-07-02

### Added

- `gmail send --from <address>` — send from a Gmail send-as alias. The address
  is validated against the account's aliases (`users/me/settings/sendAs`):
  unknown or unverified aliases fail with an error instead of Gmail silently
  rewriting the From header to the primary address. The alias's display name is
  used when set, falling back to `sender_name` / the token's name.
- `send_from` profile setting — default From address applied when `--from` is
  absent. Plain sends (no flag, no setting) skip the alias lookup entirely, so
  existing usage gains no extra API call.
- `gmail aliases ls` (alias: `list`) — list send-as aliases with display name,
  primary/default flags, and verification status.

## [0.4.0] - 2026-06-30

### Changed

- `gmail get <id>` now lists a message's attachments (filename, MIME type, size)
  with a download hint, so attachment presence is visible without a separate
  `attachments ls` call. `MessageView` gains an `attachments` array, surfaced in
  JSON output too.

## [0.3.0] - 2026-06-30

### Changed

- `gmail get <id>` now fetches `format=full` and prints the decoded message body
  (preferring `text/plain`, falling back to tag-stripped `text/html`), instead of
  only headers and a snippet. JSON output gains a `body` field. Falls back to the
  snippet when no decodable body part is present.

## [0.2.0] - 2026-06-30

### Added

- `gmail attachments ls <id>` — list a message's attachments (filename, MIME
  type, size) without downloading them.
- `gmail attachments get <id> [--out <dir>] [--index <n> | --name <file>]` —
  download attachments to disk. Defaults to all attachments into the current
  directory; `--index` (1-based) or `--name` narrow to a single file. Attachment
  filenames are sanitized to their basename so a crafted name cannot write
  outside `--out`.
- API layer: `GmailClient::list_attachments` (fetches `format=full` and walks the
  MIME part tree for parts with a downloadable `attachmentId`) and
  `GmailClient::get_attachment` (fetches `messages.attachments.get` and decodes
  the base64url payload, tolerating padded and unpadded input).

### Changed

- Added doc comments across the source tree.
