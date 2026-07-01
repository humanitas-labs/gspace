use crate::cli::GetArgs;
use crate::context::AppContext;
use crate::error::AppResult;
use crate::output::OutputMode;

/// Fetch a single message by id and emit its headers plus decoded body text.
pub async fn run(ctx: &AppContext, args: GetArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let message = ctx
        .gmail_client
        .get_msg_full(&args.id, &access_token)
        .await?;

    if ctx.output.mode() == OutputMode::Text {
        let from = message.from.as_deref().unwrap_or("(unknown sender)");
        let subject = message.subject.as_deref().unwrap_or("(no subject)");
        println!("{} | {} | {}", message.id, from, subject);
        if let Some(date) = &message.date {
            println!("date: {date}");
        }

        if !message.attachments.is_empty() {
            println!("attachments ({}):", message.attachments.len());
            for (index, attachment) in message.attachments.iter().enumerate() {
                match attachment.size {
                    Some(size) => println!(
                        "  {}. {} | {} | {} bytes",
                        index + 1,
                        attachment.filename,
                        attachment.mime_type,
                        size
                    ),
                    None => println!(
                        "  {}. {} | {}",
                        index + 1,
                        attachment.filename,
                        attachment.mime_type
                    ),
                }
            }
            println!("  (download with: gmail attachments get {})", message.id);
        }
        println!();

        match message.body.as_deref() {
            Some(body) => println!("{body}"),
            // Fall back to the snippet when no decodable body part was found.
            None => println!("{}", message.snippet.as_deref().unwrap_or("(no body)")),
        }

        return Ok(());
    }

    let from = message.from.as_deref().unwrap_or("(unknown sender)");
    let subject = message.subject.as_deref().unwrap_or("(no subject)");
    let text = format!("{} | {} | {}", message.id, from, subject);
    ctx.output.emit(&text, &message)
}
