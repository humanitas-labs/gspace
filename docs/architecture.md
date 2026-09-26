# Architecture

## Goal

One crate (`gspace`) hosting Google Workspace CLIs behind clear module boundaries. Shared plumbing — OAuth, profiles, token storage, HTTP transport, output — lives in the library; each product surface ships as its own binary (`gmail`, `gcal`). Adding a surface means a new `src/bin/` entry, a service client, and a command module; auth and profiles come for free.

## Runtime flow

1. `src/main.rs` (the `gmail` binary) parses CLI args and calls `gspace::run`; `src/bin/gcal.rs` parses its own `GcalCli` and calls `gspace::gcal::run`.
2. `src/app.rs` / `src/gcal.rs` build the shared `AppContext` from profile/output flags and dispatch to a command handler.
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
  - `http::JsonClient` is the shared bearer-auth JSON transport (base URL + service label + auth recovery hint); service clients wrap it.
  - Exposes `GmailClient` methods for `list`, `get`, `send`, and `label` operations.
  - Exposes `CalendarClient` methods for event insert (with Meet conference requests), get, patch (field-level `events.patch` updates), list, and delete.
- `commands`
  - Maps command args to service calls.
  - Keeps business rules local to command behavior.
  - Prompts for missing OAuth profile settings during `auth login`.
  - `commands/calendar` owns gcal handlers plus local datetime parsing (`YYYY-MM-DD HH:MM`, `today HH:MM`, RFC 3339).
- `mail`
  - Handles MIME construction and encoding concerns.
- `output`
  - Encapsulates formatting strategy for text vs JSON output.

## State and storage

- Shared app config path: `<config_dir>/gmail/config.json` (default profile and the shared OAuth client)
- Profile settings path: `<config_dir>/gmail/profiles/<profile>.json` (identity fields, optional OAuth overrides)
- Token path: `<data_dir>/gmail/tokens/<profile>.json`
- `AppContext` carries resolved profile, settings, token store, and API client. `AppContext::settings` is the effective view (profile merged over the shared client) and is never written back; commands that persist settings load and save the raw profile file.

## OAuth details

- Grant type: authorization code with PKCE (`S256`).
- Auth endpoint: `https://accounts.google.com/o/oauth2/v2/auth`
- Token endpoint: `https://oauth2.googleapis.com/token`
- Revoke endpoint: `https://oauth2.googleapis.com/revoke`
- Userinfo endpoint: `https://openidconnect.googleapis.com/v1/userinfo`
- Scopes: `gmail.modify`, `gmail.send`, `calendar.events`, `openid`, `email`, `profile` (tokens issued before `calendar.events` was added need a one-time re-login)
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
