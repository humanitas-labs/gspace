# gmail

Rust scaffold for a Gmail CLI with this command shape:

- `gmail auth login`
- `gmail auth status`
- `gmail auth logout`
- `gmail list [--inbox] [--limit <n>] [--q <query>]`
- `gmail send ...`
- `gmail get <id>`
- `gmail label ...`
- `gmail attachments ls|get <id> ...`
- `gmail aliases ls`

OAuth login is wired with browser auth code flow + PKCE and local callback capture.
`gmail list`, `gmail get`, `gmail send`, and `gmail label` are wired to the real Gmail API.
`gmail get` prints the full decoded message body (text/plain, falling back to
stripped text/html), not just a snippet, and lists any attachments.
`gmail send` treats body input as Markdown and sends rendered `text/html` by default.
`gmail send` also sets `From` with a display name when available (`sender_name` profile setting or Google profile name captured at login).
`gmail send --from <address>` sends from a verified send-as alias (validated against
`gmail aliases ls`; unknown or unverified addresses error instead of Gmail silently
sending from the primary). The `send_from` profile setting makes an alias the default.

## Current command tree

```text
gmail [--profile <name>]   # global; overrides GMAIL_PROFILE and the configured default
  auth
    login
    status
    logout
  profile
    list                   # list profiles, marking the default
    use <name>             # set the default profile
    show                   # show the profile resolved for this invocation
  list [--inbox] [--limit <n>] [--q <query>]
  send [--reply <id>] [--attach <path> ...]
       [--to ...] [--subject ...] [--from <alias>]
       (--body ... | --body-file ... | --draft-file ... | --stdin)
  get <id>
  label
    ls
    add <id> <label...>
    rm <id> <label...>
  attachments
    ls <id>
    get <id> [--out <dir>] [--index <n> | --name <file>]
  aliases
    ls
```

See `docs/architecture.md` for data flow and implementation phases.

## Profiles

Each account is a named profile with its own settings file
(`profiles/<name>.json`) and token (`tokens/<name>.json`). Every command
resolves one profile in this order:

1. `--profile <name>` flag
2. `GMAIL_PROFILE` environment variable
3. `default_profile` in `config.json` (set via `gmail profile use <name>`)
4. the sole profile, if only one exists
5. the profile literally named `default`, if present

If several profiles exist and none of the above picks one, mailbox commands
error and ask you to set a default; `gmail profile list` / `use` still work so
you can resolve it. Switch the default with `gmail profile use <name>` — no file
juggling.

```console
$ gmail profile list
  work
* personal (default)
$ gmail --profile work list        # one-off override
$ GMAIL_PROFILE=work gmail list     # session override
```

## OAuth setup

1. Create a Google Cloud OAuth client (Desktop app recommended).
2. Enable Gmail API in your project.
3. Add a profile file (default profile on macOS):

```text
~/Library/Application Support/gmail/profiles/default.json
```

4. Put your client config in that file:

```json
{
  "client_id": "YOUR_CLIENT_ID",
  "client_secret": "YOUR_CLIENT_SECRET",
  "redirect_uri": "http://127.0.0.1:8787/callback",
  "sender_name": "Jane Doe",
  "send_from": "you@yourdomain.com"
}
```

`send_from` is optional: when set, sends default to that send-as alias
(overridable per send with `--from`); when absent, sends come from the
logged-in account's primary address.

If either `client_id` or `client_secret` is missing, `gmail auth login` prompts for both and writes the profile file for you.
If Google still rejects login with `client_secret is missing`, `gmail auth login` prompts for `client_secret`, saves it, and retries.

## Login flow

```bash
cargo run -- auth login
```

What happens:

- Prompts for OAuth client config when `client_id` or `client_secret` is missing.
- Opens browser to Google OAuth consent page.
- Starts local callback listener on your configured `redirect_uri`.
- Exchanges auth code for access/refresh tokens.
- Stores tokens under the profile token directory.

Then verify:

```bash
cargo run -- auth status
```

Logout and revoke:

```bash
cargo run -- auth logout
```

## Local usage

```bash
cargo check
cargo run -- auth status
cargo run -- list --inbox --limit 3
cargo run -- get <message-id>
cargo run -- send --to dev@example.com --subject "hello" --body "**hi** from _markdown_"
cargo run -- send --to dev@example.com --subject "with attachment" --body "see attached" --attach ./file.pdf
cargo run -- send --reply <message-id> --draft-file ./reply.txt --to dev@example.com
cargo run -- send --to dev@example.com --subject "hello" --body "hi" --from you@yourdomain.com
cargo run -- label ls
cargo run -- aliases ls
```

## Next implementation steps

1. Add integration tests with mocked Gmail responses.
2. Harden reply recipient inference and `References` handling.
3. Add attachment filename/content-type override flags.
