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
