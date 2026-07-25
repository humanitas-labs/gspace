use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::gcal::GcalListArgs;
use crate::output::OutputMode;

use super::{format_when, resolve_window};

/// List events in the requested window and print each with its links.
pub async fn run(ctx: &AppContext, args: GcalListArgs) -> AppResult<()> {
    if args.limit == 0 {
        return Err(AppError::InvalidInput(
            "--limit must be greater than 0".to_string(),
        ));
    }

    let window = resolve_window(
        args.today,
        args.tomorrow,
        args.from.as_deref(),
        args.to.as_deref(),
    )?;

    let access_token = ctx.access_token().await?;
    let events = ctx
        .calendar_client
        .list_events(
            &args.calendar,
            &window.time_min.to_rfc3339(),
            &window.time_max.to_rfc3339(),
            args.limit,
            &access_token,
        )
        .await?;

    if ctx.output.mode() == OutputMode::Text {
        if events.is_empty() {
            println!("0 events ({})", window.label);
            return Ok(());
        }

        println!("{} events ({})", events.len(), window.label);
        for event in &events {
            let summary = event.summary.as_deref().unwrap_or("(no title)");
            println!();
            println!(
                "{}  {}",
                format_when(event.start.as_deref(), event.end.as_deref()),
                summary
            );
            println!("  id: {}", event.id);
            if !event.attendees.is_empty() {
                println!("  attendees: {}", event.attendees.join(", "));
            }
            if let Some(meet) = &event.meet_link {
                println!("  meet: {meet}");
            }
        }
        return Ok(());
    }

    let text = format!("{} events", events.len());
    ctx.output.emit(&text, &events)
}
