use chrono::Duration;

use crate::api::calendar::EventDraft;
use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::gcal::GcalAddArgs;
use crate::output::OutputMode;

use super::{format_when, parse_datetime};

/// Create an event from the args and print its links.
pub async fn run(ctx: &AppContext, args: GcalAddArgs) -> AppResult<()> {
    let summary = args.title.trim();
    if summary.is_empty() {
        return Err(AppError::InvalidInput("--title must not be empty".to_string()));
    }

    let start = parse_datetime(&args.start)?;
    let end = match &args.end {
        Some(end) => parse_datetime(end)?,
        None => start + Duration::minutes(i64::from(args.duration)),
    };
    if end <= start {
        return Err(AppError::InvalidInput(
            "event end must be after its start".to_string(),
        ));
    }

    let attendees = normalize_attendees(&args.attendees)?;
    let draft = EventDraft {
        summary: summary.to_string(),
        location: args.location.clone(),
        description: args.notes.clone(),
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        attendees,
        meet: args.meet,
        request_id: format!("{:032x}", rand::random::<u128>()),
    };

    let access_token = ctx.access_token().await?;
    let event = ctx
        .calendar_client
        .insert_event(&args.calendar, &draft, &access_token)
        .await?;

    if ctx.output.mode() == OutputMode::Text {
        println!("event created: {summary}");
        println!("  id: {}", event.id);
        println!(
            "  when: {}",
            format_when(event.start.as_deref(), event.end.as_deref())
        );
        if !event.attendees.is_empty() {
            println!("  attendees: {}", event.attendees.join(", "));
        }
        if let Some(meet) = &event.meet_link {
            println!("  meet: {meet}");
        }
        if let Some(link) = &event.html_link {
            println!("  event: {link}");
        }
        return Ok(());
    }

    ctx.output.emit("event created", &event)
}

/// Trim attendee entries, drop empties, and reject anything that isn't
/// email-shaped so a typo fails here instead of becoming a silent no-op invite.
pub(crate) fn normalize_attendees(raw: &[String]) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    for entry in raw {
        let email = entry.trim();
        if email.is_empty() {
            continue;
        }
        if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
            return Err(AppError::InvalidInput(format!(
                "`{email}` does not look like an email address"
            )));
        }
        if !out.iter().any(|existing: &String| existing == email) {
            out.push(email.to_string());
        }
    }
    Ok(out)
}
