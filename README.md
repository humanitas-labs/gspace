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

OAuth login is wired with browser auth code flow + PKCE and local callback capture.
`gmail list`, `gmail get`, `gmail send`, and `gmail label` are wired to the real Gmail API.
`gmail send` treats body input as Markdown and sends rendered `text/html` by default.
`gmail send` also sets `From` with a display name when available (`sender_name` profile setting or Google profile name captured at login).

## Current command tree

```text
gmail
  auth
    login
    status
    logout
  list [--inbox] [--limit <n>] [--q <query>]
  send [--reply <id>] [--attach <path> ...]
       [--to ...] [--subject ...]
       (--body ... | --body-file ... | --draft-file ... | --stdin)
  get <id>
  label
    ls
    add <id> <label...>
    rm <id> <label...>
  attachments
    ls <id>
    get <id> [--out <dir>] [--index <n> | --name <file>]
```

See `docs/architecture.md` for data flow and implementation phases.

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
  "sender_name": "Jane Doe"
}
```

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
cargo run -- label ls
```

## Next implementation steps

1. Add integration tests with mocked Gmail responses.
2. Harden reply recipient inference and `References` handling.
3. Add attachment filename/content-type override flags.
