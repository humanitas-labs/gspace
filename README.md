# gspace

Google Workspace CLI toolbox: one crate, one OAuth client, one profile system, shipped as two binaries.

- **`gmail`**: list, read, send, labels, attachments, aliases
- **`gcal`**: create events (with Google Meet), edit, list, delete

```bash
cargo install --path .
```

## 1. Setup

1. Create a Google Cloud OAuth client (Desktop app) and enable the Gmail and Calendar APIs in its project.
2. Run `gmail --profile <name> auth login`. The first login prompts for `client_id`, `client_secret`, and `redirect_uri` and saves them to the shared `config.json`; later profiles reuse them.
3. Approve access in the browser. The token lands in `tokens/<name>.json` and the profile in `profiles/<name>.json`.

To add another account, run step 2 with a new profile name. Check a login with `gmail auth status`; sign out and revoke with `gmail auth logout`.

Config lives in `~/Library/Application Support/gmail/` on macOS:

```jsonc
// config.json (shared, 0600)
{
  "default_profile": "work",
  "client_id": "YOUR_CLIENT_ID",
  "client_secret": "YOUR_CLIENT_SECRET",
  "redirect_uri": "http://127.0.0.1:8787/callback"
}

// profiles/work.json (per-account identity)
{
  "sender_name": "Your Name",
  "send_from": "you@yourdomain.com",   // optional default send-as alias
  "signature": "..."                   // optional, see section 3
}
```

A profile may set `client_id`, `client_secret`, or `redirect_uri` to override the shared client, e.g. for a Workspace org that only allows its own internal client. Fields fall back independently, so override `client_id` and `client_secret` together.

## 2. Profiles

`gmail` and `gcal` share profiles and tokens. Each command resolves one profile in this order:

1. `--profile <name>`
2. `GMAIL_PROFILE`
3. `default_profile` in `config.json` (set with `gmail profile use <name>`)
4. the only profile, if there is one
5. a profile named `default`

If several profiles exist and none is selected, mailbox commands error until you set a default.

## 3. gmail

```text
gmail [--profile <name>] [--json]
  auth        login | status | logout
  profile     list | use <name> | show
  signature   show | set <text> | set-file <path> | clear
  list        [--inbox] [--limit <n>] [--q <query>]
  get <id>
  send        [--to ...] [--cc ...] [--bcc ...] [--subject ...] [--from <alias>] [--reply <id>]
              [--attach <path> ...] [--signature <text> | --no-signature]
              (--body ... | --body-file ... | --draft-file ... | --stdin)
  label       ls | add <id> <label...> | rm <id> <label...>
  attachments ls <id> | get <id> [--out <dir>] [--index <n> | --name <file>]
  aliases     ls
```

- `get` prints the full decoded body (text/plain, else stripped HTML) and lists attachments.
- `send` renders the body as Markdown to HTML and sets `From` with `sender_name` (or the Google profile name).
- `send --from` must be a verified send-as alias from `aliases ls`; anything else errors rather than silently sending from the primary. `send_from` makes an alias the default.
- `send --reply <id>` keeps the thread and reply headers.
- The profile's `signature` (Markdown, line breaks preserved) is appended below the body; `--signature` overrides it for one send and `--no-signature` suppresses it.

## 4. gcal

```text
gcal [--profile <name>] [--json]
  add   --title <t> --start <datetime> [--end <datetime> | --duration <mins, default 30>]
        [--attendees a@x.com,b@y.com] [--meet] [--location <text>] [--notes <text>] [--calendar <id>]
  edit  <event-id> [--title <t>] [--start <datetime>] [--end <datetime> | --duration <mins>]
        [--attendees ...] [--location <text>] [--notes <text>] [--calendar <id>]
  list  [--today | --tomorrow | --week | --from <d> --to <d>] [--limit <n>] [--calendar <id>]
  rm    <event-id> [--calendar <id>]
```

- Datetimes accept RFC 3339, `YYYY-MM-DD HH:MM`, and `today HH:MM` / `tomorrow HH:MM`, in local time. Events go on the primary calendar unless `--calendar` is set.
- `--meet` attaches a Google Meet link. Attendees get Google invites on `add`, update emails on `edit`, and cancellations on `rm`.
- `edit` patches in place, so the event id and Meet link survive. `--start` alone shifts the event and keeps its duration; `--attendees` replaces the list. Recurring events are not supported.
- Profiles authorized before calendar support need a one-time `gmail auth login` to grant the `calendar.events` scope.

```console
$ gcal add --title "Weekly sync" --start "tomorrow 10:00" --attendees alex@example.com --meet
event created: Weekly sync
  id: k2j4...
  when: Sun 2026.07.26 10:00–10:30
  meet: https://meet.google.com/abc-defg-hij
```

See [docs/architecture.md](docs/architecture.md) for module layout and data flow.
