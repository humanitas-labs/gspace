use std::fs;
use std::io::{self, Read};

use crate::api::models::{Attachment, SendRequest};
use crate::auth::token_store::TokenStore;
use crate::cli::SendArgs;
use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::mail::mime;

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

async fn build_send_request(
    ctx: &AppContext,
    access_token: &str,
    args: SendArgs,
) -> AppResult<SendRequest> {
    let body_markdown = read_body(&args)?;
    let body = mime::markdown_to_html(&body_markdown);
    let attachments = read_attachments(&args.attach)?;
    let from = resolve_from_header(ctx)?;

    if let Some(reply_id) = args.reply.clone() {
        return build_reply_request(ctx, access_token, args, body, attachments, &reply_id).await;
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

async fn build_reply_request(
    ctx: &AppContext,
    access_token: &str,
    args: SendArgs,
    body: String,
    attachments: Vec<Attachment>,
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
        from: resolve_from_header(ctx)?,
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

fn resolve_from_header(ctx: &AppContext) -> AppResult<Option<String>> {
    let token = ctx.token_store.load(&ctx.profile)?;
    let Some(token) = token else {
        return Ok(None);
    };

    let email = token
        .email
        .as_deref()
        .map(sanitize_header_value)
        .filter(|value| !value.is_empty());
    let name = ctx
        .settings
        .sender_name
        .as_deref()
        .map(sanitize_header_value)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            token
                .name
                .as_deref()
                .map(sanitize_header_value)
                .filter(|value| !value.is_empty())
        });

    match (name, email) {
        (Some(name), Some(email)) => Ok(Some(format!("{name} <{email}>"))),
        (None, Some(email)) => Ok(Some(email)),
        _ => Ok(None),
    }
}

fn sanitize_header_value(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|value| *value != '\r' && *value != '\n' && *value != '"')
        .collect()
}

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

fn ensure_reply_subject(subject: String) -> String {
    let trimmed = subject.trim();
    if trimmed.to_ascii_lowercase().starts_with("re:") {
        trimmed.to_string()
    } else {
        format!("Re: {trimmed}")
    }
}

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
