---
title: "gcal — Google Calendar / Meet scheduling from the CLI"
date: 2026-07-25
status: done
affects: "new `gcal` binary; OAuth scopes; API client layer"
---

## Context

Scheduling a Google Meet from the terminal is not possible today. `ical` (EventKit) can create local calendar events but cannot mint Meet links — those only come from the Google Calendar API (`events.insert` with `conferenceData.createRequest` and `conferenceDataVersion=1`).

This crate already owns the hard part: OAuth2 device flow, per-profile token store (keyring-backed), profile resolution (`--profile` > `GMAIL_PROFILE` > configured default), and a JSON/text output layer. Total ~4,100 lines, cleanly layered (`auth/`, `config/`, `api/`, `commands/`, `output/`). The plan reuses all of it.

Shape decision: a **second binary, `gcal`, in this same crate** — not a `gmail calendar` subcommand. `gmail calendar add` reads wrong, and calendar has its own verb set. Both binaries share the lib (auth, config, profiles, error, output), so `gcal --profile work` and `gmail --profile work` hit the same token. Cargo builds `src/main.rs` (gmail) and `src/bin/gcal.rs` automatically.

Scope for v1: `add` (with `--meet`), `list`, `rm`. No `edit`, no free/busy, no secondary-calendar management — `--calendar <id>` passes through where trivial, defaulting to `primary`.

## Changes

1. **`src/auth/oauth.rs` — add the calendar scope.**
   Append `https://www.googleapis.com/auth/calendar.events` to `OAUTH_SCOPES` (line 25). Existing tokens lack the scope, so calendar API calls with an old token will 403. Decision: detect the 403 `insufficientPermissions` in the calendar client and return a directed error — "token predates calendar scope; run `gmail auth login` for this profile" — rather than tracking granted scopes locally. One-time re-login per profile is acceptable; scope bookkeeping is not worth the code.

2. **`src/api/calendar.rs` + model additions — Calendar v3 client.**
   New module beside `messages.rs`/`labels.rs`, base URL `https://www.googleapis.com/calendar/v3`. Three operations:
   - `insert_event` — POST `/calendars/{id}/events?conferenceDataVersion=1&sendUpdates=all`. When `--meet` is set, body includes `conferenceData.createRequest` with a fresh UUID `requestId` and `conferenceSolutionKey.type = "hangoutsMeet"`. `sendUpdates=all` is mandatory — without it attendees silently get no invite email.
   - `list_events` — GET `/calendars/{id}/events` with `timeMin`/`timeMax`/`singleEvents=true&orderBy=startTime`.
   - `delete_event` — DELETE `/calendars/{id}/events/{eventId}?sendUpdates=all`.
   Response projection into an `EventView` (id, summary, start/end, attendees, `hangoutLink`, `htmlLink`) mirroring the existing `MessageView` pattern.

3. **`src/bin/gcal.rs` + `src/commands/calendar/` — the CLI.**
   Own clap parser (name `gcal`, global `--profile`/`--json`/`-v` matching `gmail`), thin main that builds the same `Context` and dispatches:

   ```text
   gcal add --title <t> --start <datetime>
            (--end <datetime> | --duration <mins, default 30>)
            [--attendees a@x.com,b@y.com] [--meet] [--location] [--notes]
            [--calendar <id>]                      # default: primary
   gcal list [--today|--tomorrow|--week | --from <d> --to <d>] [--limit N]
   gcal rm <event-id>
   ```

   Datetime parsing via chrono: ISO 8601, `YYYY-MM-DD HH:MM`, and `today HH:MM` / `tomorrow HH:MM` (matching `ical`'s accepted forms), interpreted in local time and sent as RFC 3339 with offset. `add` prints the event's `htmlLink` and, with `--meet`, the `hangoutLink` on success.

4. **Tests — `tests/api_calendar.rs`, `tests/cli_gcal.rs`.**
   Follow the existing test layout: serialize/deserialize the insert body (conferenceData present only with `--meet`; `requestId` non-empty), event-list projection, datetime parsing table including the relative forms, and clap parse tests for the new binary.

5. **Docs — `README.md`, `docs/architecture.md`, workspace `~/.claude/CLAUDE.md` tools section.**
   Document the second binary, the re-login requirement after upgrade, and the reinstall command (`cargo install --path ~/Documents/repos/gmail-cli` now installs both binaries).

## Files touched

```
┌────────────────────────────────┬────────────────────────────────────────────┐
│              File              │                   Action                   │
├────────────────────────────────┼────────────────────────────────────────────┤
│ Cargo.toml                     │ Edit (uuid dep for requestId, if absent)   │
│ src/auth/oauth.rs              │ Edit (append calendar.events scope)        │
│ src/api/calendar.rs            │ Create (Calendar v3 client + EventView)    │
│ src/api/mod.rs                 │ Edit (register module)                     │
│ src/bin/gcal.rs                │ Create (clap parser + dispatch)            │
│ src/commands/calendar/add.rs   │ Create                                     │
│ src/commands/calendar/list.rs  │ Create                                     │
│ src/commands/calendar/rm.rs    │ Create                                     │
│ src/commands/calendar/mod.rs   │ Create (+ datetime parsing helpers)        │
│ src/commands/mod.rs            │ Edit (register module)                     │
│ tests/api_calendar.rs          │ Create                                     │
│ tests/cli_gcal.rs              │ Create                                     │
│ README.md, docs/architecture.md│ Edit (document gcal, re-login note)        │
└────────────────────────────────┴────────────────────────────────────────────┘
```

## Verification

1. `cargo test` — new suites plus no regressions in the gmail suites.
2. `cargo install --path .` — confirm both `gmail` and `gcal` land in `~/.cargo/bin`.
3. Re-auth one profile: `gmail --profile personal auth login`; `gmail auth status` still healthy; `gmail send` still works (scope superset, no regression).
4. Live round-trip on the personal profile:
   - `gcal add --title "gcal smoke test" --start "tomorrow 10:00" --duration 15 --attendees jane@example.com --meet` — verify the printed `hangoutLink`, the invite email arriving at the attendee address, and the event visible in Google Calendar UI.
   - `gcal list --tomorrow` shows it; `gcal rm <id>` removes it and sends the cancellation.
5. Stale-token path: run `gcal add` on the not-yet-re-authed profile — confirm the directed "re-run gmail auth login" error, not a raw 403.
