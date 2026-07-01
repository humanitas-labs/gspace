use std::fs;
use std::path::Path;

use crate::api::models::{AttachmentMeta, SavedAttachment};
use crate::cli::{AttachmentsCommand, AttachmentsGetArgs, AttachmentsLsArgs};
use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::output::OutputMode;

/// Dispatch a `gmail attachments` subcommand to its handler.
pub async fn run(ctx: &AppContext, command: AttachmentsCommand) -> AppResult<()> {
    match command {
        AttachmentsCommand::Ls(args) => ls(ctx, args).await,
        AttachmentsCommand::Get(args) => get(ctx, args).await,
    }
}

/// List the downloadable attachments on a message without fetching their bytes.
async fn ls(ctx: &AppContext, args: AttachmentsLsArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let list = ctx
        .gmail_client
        .list_attachments(&args.id, &access_token)
        .await?;

    if ctx.output.mode() == OutputMode::Text {
        if list.attachments.is_empty() {
            println!("no attachments on message {}", list.message_id);
            return Ok(());
        }

        for (index, attachment) in list.attachments.iter().enumerate() {
            println!("{}. {}", index + 1, describe(attachment));
        }

        return Ok(());
    }

    let text = format!("{} attachments", list.attachments.len());
    ctx.output.emit(&text, &list)
}

/// Download attachments to `--out`, optionally narrowed by `--index` or `--name`.
async fn get(ctx: &AppContext, args: AttachmentsGetArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let list = ctx
        .gmail_client
        .list_attachments(&args.id, &access_token)
        .await?;

    let selected = select(&list.attachments, args.index, args.name.as_deref())?;

    fs::create_dir_all(&args.out)?;

    let mut saved = Vec::new();
    for attachment in selected {
        let bytes = ctx
            .gmail_client
            .get_attachment(&list.message_id, &attachment.attachment_id, &access_token)
            .await?;

        let file_name = safe_file_name(&attachment.filename)?;
        let path = args.out.join(file_name);
        fs::write(&path, &bytes)?;

        saved.push(SavedAttachment {
            filename: attachment.filename.clone(),
            path: path.display().to_string(),
            bytes: bytes.len() as u64,
        });
    }

    if ctx.output.mode() == OutputMode::Text {
        for item in &saved {
            println!(
                "saved {} ({} bytes) -> {}",
                item.filename, item.bytes, item.path
            );
        }
        return Ok(());
    }

    let text = format!("{} attachments saved", saved.len());
    ctx.output.emit(&text, &saved)
}

/// Pick which attachments to download: a single 1-based `index`, all filename
/// matches for `name`, or every attachment when neither filter is supplied.
fn select<'a>(
    attachments: &'a [AttachmentMeta],
    index: Option<usize>,
    name: Option<&str>,
) -> AppResult<Vec<&'a AttachmentMeta>> {
    if attachments.is_empty() {
        return Err(AppError::InvalidInput(
            "message has no attachments to download".to_string(),
        ));
    }

    if let Some(index) = index {
        if index == 0 || index > attachments.len() {
            return Err(AppError::InvalidInput(format!(
                "index {index} out of range; message has {} attachment(s)",
                attachments.len()
            )));
        }
        return Ok(vec![&attachments[index - 1]]);
    }

    if let Some(name) = name {
        let matched: Vec<&AttachmentMeta> = attachments
            .iter()
            .filter(|attachment| attachment.filename.eq_ignore_ascii_case(name))
            .collect();
        if matched.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "no attachment named `{name}` on this message"
            )));
        }
        return Ok(matched);
    }

    Ok(attachments.iter().collect())
}

/// Render a single attachment as a one-line summary for text output.
fn describe(attachment: &AttachmentMeta) -> String {
    match attachment.size {
        Some(size) => format!(
            "{} | {} | {} bytes",
            attachment.filename, attachment.mime_type, size
        ),
        None => format!("{} | {}", attachment.filename, attachment.mime_type),
    }
}

/// Strip any directory components so a crafted `filename` can't write outside `--out`.
fn safe_file_name(filename: &str) -> AppResult<String> {
    Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::InvalidInput(format!("attachment has an unusable filename: `{filename}`"))
        })
}
