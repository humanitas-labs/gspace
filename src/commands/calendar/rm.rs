use serde_json::json;

use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::gcal::GcalRmArgs;

/// Delete an event; Calendar emails cancellations to all attendees.
pub async fn run(ctx: &AppContext, args: GcalRmArgs) -> AppResult<()> {
    let event_id = args.event_id.trim();
    if event_id.is_empty() {
        return Err(AppError::InvalidInput("event id must not be empty".to_string()));
    }

    let access_token = ctx.access_token().await?;
    ctx.calendar_client
        .delete_event(&args.calendar, event_id, &access_token)
        .await?;

    let text = format!("event {event_id} deleted (cancellations sent to attendees)");
    ctx.output.emit(
        &text,
        &json!({ "id": event_id, "deleted": true, "calendar": args.calendar }),
    )
}
