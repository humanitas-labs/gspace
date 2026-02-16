use crate::cli::{LabelCommand, LabelMutateArgs};
use crate::context::AppContext;
use crate::error::AppResult;

pub async fn run(ctx: &AppContext, command: LabelCommand) -> AppResult<()> {
    match command {
        LabelCommand::Ls => {
            let access_token = ctx.access_token().await?;
            let labels = ctx.gmail_client.list_labels(&access_token).await?;

            let text = format!("{} labels", labels.len());
            ctx.output.emit(&text, &labels)
        }
        LabelCommand::Add(args) => mutate_add(ctx, args).await,
        LabelCommand::Rm(args) => mutate_rm(ctx, args).await,
    }
}

async fn mutate_add(ctx: &AppContext, args: LabelMutateArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let result = ctx
        .gmail_client
        .add_labels(&args.id, &args.labels, &access_token)
        .await?;

    let text = format!("labels added on {}", result.id);
    ctx.output.emit(&text, &result)
}

async fn mutate_rm(ctx: &AppContext, args: LabelMutateArgs) -> AppResult<()> {
    let access_token = ctx.access_token().await?;
    let result = ctx
        .gmail_client
        .rm_labels(&args.id, &args.labels, &access_token)
        .await?;

    let text = format!("labels removed on {}", result.id);
    ctx.output.emit(&text, &result)
}
