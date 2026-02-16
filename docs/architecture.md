# Architecture

## Goal

Provide a composable Rust CLI named `gmail` with stable command UX first, then layer OAuth and Gmail API implementations behind clear module boundaries.

## Runtime flow

1. `src/main.rs` parses CLI args and calls `gmail::run`.
2. `src/app.rs` builds `AppContext` from profile/output flags and dispatches to a command handler.
3. `src/commands/*` validates args and orchestrates auth/token/API calls.
4. `src/output/*` renders results as text or JSON.

## Module responsibilities

- `config`
  - Resolves profile name.
  - Computes config/data paths.
  - Loads profile settings.
- `auth`
  - Owns token schema and token persistence interfaces.
  - Exposes `AuthService` (`login`, `refresh`, `status`, `logout`) as auth entrypoint.
  - Implements browser OAuth code flow with PKCE and local callback capture.
- `api`
  - Owns API-facing model types and endpoint helpers.
  - Exposes `GmailClient` methods for `list`, `get`, `send`, and `label` operations.
- `commands`
  - Maps command args to service calls.
  - Keeps business rules local to command behavior.
  - Prompts for missing OAuth profile settings during `auth login`.
- `mail`
  - Handles MIME construction and encoding concerns.
- `output`
  - Encapsulates formatting strategy for text vs JSON output.

## State and storage

- Profile settings path: `<config_dir>/gmail/profiles/<profile>.json`
- Token path: `<data_dir>/gmail/tokens/<profile>.json`
- `AppContext` carries resolved profile, settings, token store, and API client.

## OAuth details

- Grant type: authorization code with PKCE (`S256`).
- Auth endpoint: `https://accounts.google.com/o/oauth2/v2/auth`
- Token endpoint: `https://oauth2.googleapis.com/token`
- Revoke endpoint: `https://oauth2.googleapis.com/revoke`
- Userinfo endpoint: `https://openidconnect.googleapis.com/v1/userinfo`
- Scopes: `gmail.modify`, `gmail.send`, `openid`, `email`
- Redirect URI: profile setting `redirect_uri`, default `http://127.0.0.1:8787/callback`
- Token refresh: `AppContext::access_token` auto-refreshes expired access tokens when refresh token exists.

## Error model

`AppError` is a single typed enum for config, auth, validation, I/O, HTTP, JSON, URL parsing, and `not implemented` surfaces.

## Planned implementation phases

1. **Scaffold (current)**
   - Command tree and module layout compiled.
   - Core dependencies and dispatch in place.
2. **Auth**
   - OAuth login flow and refresh implemented.
   - Optionally move token storage to keychain/keyring.
3. **Gmail APIs**
   - `list`, `get`, `send`, and label operations implemented.
   - reply-from-file flow implemented with thread headers.
   - attachment sending implemented via multipart MIME.
   - Add request/response mapping tests.
4. **Hardening**
   - Retry/backoff for transient failures.
   - Better user-facing diagnostics and pagination/search additions.
