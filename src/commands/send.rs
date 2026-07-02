use std::fs;
use std::io::{self, Read};

use crate::api::models::{Attachment, SendAsView, SendRequest};
use crate::auth::TokenSet;
use crate::auth::token_store::TokenStore;
use crate::cli::SendArgs;
use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::mail::mime;

/// Build a send request from the args, encode it as a raw message, and submit it.
pub async fn run(ctx: &AppContext, args: SendArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let request = build_send_request(ctx, &access_token, args).await?;
    let raw = mime::build_raw_message(&request);
    let result = ctx
        .gmail_client
        .send(&raw, request.thread_id.as_deref(), &access_token)
        .await?;

    let text = format!("sent message {}", result.id);
    ctx.output.emit(&text, &result)
}

/// Assemble a `SendRequest` from args, rendering the markdown body and reading attachments;
/// delegates to the reply path when `--reply` is set.
async fn build_send_request(
    ctx: &AppContext,
    access_token: &str,
    args: SendArgs,
) -> AppResult<SendRequest> {
    let body_markdown = read_body(&args)?;
    let body = mime::markdown_to_html(&body_markdown);
    let attachments = read_attachments(&args.attach)?;
    let from_override = args.from.clone().or_else(|| ctx.settings.send_from.clone());
    let from = resolve_from_header(ctx, access_token, from_override.as_deref()).await?;

    if let Some(reply_id) = args.reply.clone() {
        return build_reply_request(ctx, access_token, args, body, attachments, from, &reply_id)
            .await;
    }

    if args.to.is_empty() {
        return Err(AppError::InvalidInput(
            "--to is required unless --reply is used".to_string(),
        ));
    }

    let subject = args.subject.ok_or_else(|| {
        AppError::InvalidInput("--subject is required unless --reply is used".to_string())
    })?;

    Ok(SendRequest {
        from,
        to: args.to,
        cc: args.cc,
        bcc: args.bcc,
        subject,
        body,
        in_reply_to: None,
        references: None,
        thread_id: None,
        attachments,
    })
}

/// Build a reply by fetching the parent message and deriving recipient, subject, threading headers.
#[allow(clippy::too_many_arguments)]
async fn build_reply_request(
    ctx: &AppContext,
    access_token: &str,
    args: SendArgs,
    body: String,
    attachments: Vec<Attachment>,
    from: Option<String>,
    reply_id: &str,
) -> AppResult<SendRequest> {
    let parent = ctx.gmail_client.get_msg(reply_id, access_token).await?;
    let mut to = args.to;
    if to.is_empty() {
        let fallback = parent.reply_to.clone().or_else(|| parent.from.clone());
        let Some(recipient) = fallback else {
            return Err(AppError::InvalidInput(
                "unable to infer reply recipient; pass --to explicitly".to_string(),
            ));
        };
        to.push(recipient);
    }

    let subject = match args.subject {
        Some(subject) => ensure_reply_subject(subject),
        None => {
            let base = parent.subject.unwrap_or_else(|| "(no subject)".to_string());
            ensure_reply_subject(base)
        }
    };

    let in_reply_to = parent.message_id;
    let references = merge_references(parent.references, in_reply_to.clone());

    Ok(SendRequest {
        from,
        to,
        cc: args.cc,
        bcc: args.bcc,
        subject,
        body,
        in_reply_to,
        references,
        thread_id: parent.thread_id,
        attachments,
    })
}

/// Resolve the `From` header. An explicit alias (from `--from` or the
/// `send_from` setting) is validated against the account's send-as aliases so
/// typos and unverified addresses fail loudly instead of Gmail silently
/// rewriting them to the primary address. Without an override, the header is
/// derived from the stored token's email as before (no extra API call).
async fn resolve_from_header(
    ctx: &AppContext,
    access_token: &str,
    from_override: Option<&str>,
) -> AppResult<Option<String>> {
    let token = ctx.token_store.load(&ctx.profile)?;

    if let Some(requested) = from_override {
        let alias = resolve_send_as_alias(ctx, access_token, requested).await?;
        let name = alias
            .display_name
            .as_deref()
            .map(sanitize_header_value)
            .filter(|value| !value.is_empty())
            .or_else(|| configured_or_token_name(ctx, token.as_ref()));
        return Ok(Some(format_from_header(name, alias.email)));
    }

    let Some(token) = token else {
        return Ok(None);
    };

    let email = token
        .email
        .as_deref()
        .map(sanitize_header_value)
        .filter(|value| !value.is_empty());
    let name = configured_or_token_name(ctx, Some(&token));

    match (name, email) {
        (name, Some(email)) => Ok(Some(format_from_header(name, email))),
        _ => Ok(None),
    }
}

/// Look up `requested` among the account's send-as aliases, erroring when it
/// is unknown or not yet verified.
async fn resolve_send_as_alias(
    ctx: &AppContext,
    access_token: &str,
    requested: &str,
) -> AppResult<SendAsView> {
    let requested = sanitize_header_value(requested);
    if requested.is_empty() {
        return Err(AppError::InvalidInput(
            "--from address must not be empty".to_string(),
        ));
    }

    let aliases = ctx.gmail_client.list_send_as(access_token).await?;
    let Some(alias) = aliases
        .into_iter()
        .find(|alias| alias.email.eq_ignore_ascii_case(&requested))
    else {
        return Err(AppError::InvalidInput(format!(
            "`{requested}` is not a send-as alias on this account; run `gmail aliases ls` to inspect aliases"
        )));
    };

    if !alias.is_sendable() {
        return Err(AppError::InvalidInput(format!(
            "send-as alias `{}` is not verified (status: {}); gmail would silently send from your primary address instead",
            alias.email,
            alias.verification_status.as_deref().unwrap_or("unknown")
        )));
    }

    Ok(alias)
}

/// The configured sender_name, falling back to the token's display name.
fn configured_or_token_name(ctx: &AppContext, token: Option<&TokenSet>) -> Option<String> {
    ctx.settings
        .sender_name
        .as_deref()
        .map(sanitize_header_value)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            token
                .and_then(|token| token.name.as_deref())
                .map(sanitize_header_value)
                .filter(|value| !value.is_empty())
        })
}

/// Format a From header value as `Name <email>` or a bare address.
fn format_from_header(name: Option<String>, email: String) -> String {
    match name {
        Some(name) => format!("{name} <{email}>"),
        None => email,
    }
}

/// Strip CR, LF, and quote characters to prevent header injection.
fn sanitize_header_value(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|value| *value != '\r' && *value != '\n' && *value != '"')
        .collect()
}

/// Read the message body from exactly one of --body, --body-file, --draft-file, or --stdin.
fn read_body(args: &SendArgs) -> AppResult<String> {
    let mut selected = 0;

    if args.body.is_some() {
        selected += 1;
    }
    if args.body_file.is_some() {
        selected += 1;
    }
    if args.draft_file.is_some() {
        selected += 1;
    }
    if args.stdin {
        selected += 1;
    }

    if selected == 0 {
        return Err(AppError::InvalidInput(
            "missing body source; pass one of --body, --body-file, --draft-file, or --stdin"
                .to_string(),
        ));
    }

    if selected > 1 {
        return Err(AppError::InvalidInput(
            "pass only one body source: --body, --body-file, --draft-file, or --stdin".to_string(),
        ));
    }

    if let Some(body) = &args.body {
        return Ok(body.clone());
    }

    if let Some(path) = &args.body_file {
        return Ok(fs::read_to_string(path)?);
    }

    if let Some(path) = &args.draft_file {
        return Ok(fs::read_to_string(path)?);
    }

    let mut body = String::new();
    io::stdin().read_to_string(&mut body)?;
    Ok(body)
}

/// Read each attachment path into bytes, inferring filename and MIME type.
fn read_attachments(paths: &[std::path::PathBuf]) -> AppResult<Vec<Attachment>> {
    let mut attachments = Vec::new();

    for path in paths {
        let data = fs::read(path)?;
        let filename = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .ok_or_else(|| {
                AppError::InvalidInput(format!("invalid attachment path: {}", path.display()))
            })?;
        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .essence_str()
            .to_string();

        attachments.push(Attachment {
            filename,
            mime_type,
            data,
        });
    }

    Ok(attachments)
}

/// Prefix a subject with `Re:` unless it already starts with one.
fn ensure_reply_subject(subject: String) -> String {
    let trimmed = subject.trim();
    if trimmed.to_ascii_lowercase().starts_with("re:") {
        trimmed.to_string()
    } else {
        format!("Re: {trimmed}")
    }
}

/// Append the parent's Message-ID to the existing References chain, avoiding duplicates.
fn merge_references(existing: Option<String>, message_id: Option<String>) -> Option<String> {
    let message_id = message_id?.trim().to_string();
    if message_id.is_empty() {
        return None;
    }

    let mut refs = existing
        .unwrap_or_default()
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if !refs.iter().any(|value| value == &message_id) {
        refs.push(message_id);
    }

    if refs.is_empty() {
        None
    } else {
        Some(refs.join(" "))
    }
}
