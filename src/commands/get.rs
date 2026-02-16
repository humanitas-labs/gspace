use crate::cli::GetArgs;
use crate::context::AppContext;
use crate::error::AppResult;

pub async fn run(ctx: &AppContext, args: GetArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let message = ctx.gmail_client.get_msg(&args.id, &access_token).await?;

    let from = message.from.as_deref().unwrap_or("(unknown sender)");
    let subject = message.subject.as_deref().unwrap_or("(no subject)");
    let text = format!("{} | {} | {}", message.id, from, subject);
    ctx.output.emit(&text, &message)
}
