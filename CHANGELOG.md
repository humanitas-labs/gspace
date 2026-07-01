# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions follow semver.

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
