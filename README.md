# gmail

Rust scaffold for a Gmail CLI with this command shape:

- `gmail auth login`
- `gmail auth status`
- `gmail auth logout`
- `gmail list [--inbox] [--limit <n>] [--q <query>]`
- `gmail send ...`
- `gmail get <id>`
- `gmail label ...`

OAuth login is wired with browser auth code flow + PKCE and local callback capture.
`gmail list`, `gmail get`, `gmail send`, and `gmail label` are wired to the real Gmail API.

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
  "redirect_uri": "http://127.0.0.1:8787/callback"
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
cargo run -- send --to dev@example.com --subject "hello" --body "hi"
cargo run -- send --to dev@example.com --subject "with attachment" --body "see attached" --attach ./file.pdf
cargo run -- send --reply <message-id> --draft-file ./reply.txt --to dev@example.com
cargo run -- label ls
```

## Next implementation steps

1. Add integration tests with mocked Gmail responses.
2. Harden reply recipient inference and `References` handling.
3. Add attachment filename/content-type override flags.
