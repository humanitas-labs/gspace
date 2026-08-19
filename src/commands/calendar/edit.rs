use chrono::{DateTime, Duration, Local};

use crate::api::calendar::EventPatch;
use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::gcal::GcalEditArgs;
use crate::output::OutputMode;

use super::add::normalize_attendees;
use super::{format_when, parse_datetime};

/// Patch an existing event in place via `events.patch`, keeping its id and
/// Meet link; attendees get a single "updated event" email.
pub async fn run(ctx: &AppContext, args: GcalEditArgs) -> AppResult<()> {
    let event_id = args.event_id.trim();
    if event_id.is_empty() {
        return Err(AppError::InvalidInput("event id must not be empty".to_string()));
    }

    let summary = match &args.title {
        Some(title) => {
            let title = title.trim();
            if title.is_empty() {
                return Err(AppError::InvalidInput("--title must not be empty".to_string()));
            }
            Some(title.to_string())
        }
        None => None,
    };

    let attendees = match &args.attendees {
        Some(raw) => {
            let attendees = normalize_attendees(raw)?;
            if attendees.is_empty() {
                return Err(AppError::InvalidInput(
                    "--attendees must contain at least one email".to_string(),
                ));
            }
            Some(attendees)
        }
        None => None,
    };

    let has_time_edit = args.start.is_some() || args.end.is_some() || args.duration.is_some();
    if !has_time_edit
        && summary.is_none()
        && attendees.is_none()
        && args.location.is_none()
        && args.notes.is_none()
    {
        return Err(AppError::InvalidInput(
            "nothing to update; pass at least one field flag".to_string(),
        ));
    }

    let access_token = ctx.access_token().await?;

    // Only time edits need the current event: to keep the duration when just
    // --start moves, and to validate end > start when only one side changes.
    let (start, end) = if has_time_edit {
        let current = ctx
            .calendar_client
            .get_event(&args.calendar, event_id, &access_token)
            .await?;
        let new_start = args.start.as_deref().map(parse_datetime).transpose()?;
        let new_end = args.end.as_deref().map(parse_datetime).transpose()?;
        let (start, end) = resolve_times(
            current.start.as_deref(),
            current.end.as_deref(),
            new_start,
            new_end,
            args.duration,
        )?;
        (Some(start), Some(end))
    } else {
        (None, None)
    };

    let patch = EventPatch {
        summary,
        location: args.location.clone(),
        description: args.notes.clone(),
        start,
        end,
        attendees,
    };

    let event = ctx
        .calendar_client
        .patch_event(&args.calendar, event_id, &patch, &access_token)
        .await?;

    if ctx.output.mode() == OutputMode::Text {
        println!("event updated: {}", event.summary.as_deref().unwrap_or("(no title)"));
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

    ctx.output.emit("event updated", &event)
}

/// Resolve the event's new start/end from the current boundaries and the
/// requested changes, returning RFC 3339 strings for the patch body.
///
/// The start moves if `--start` was given; the end comes from `--end`, else
/// from `--duration` measured off the new start, else the current duration is
/// preserved (so `--start` alone shifts the whole event).
fn resolve_times(
    old_start: Option<&str>,
    old_end: Option<&str>,
    new_start: Option<DateTime<Local>>,
    new_end: Option<DateTime<Local>>,
    duration: Option<u32>,
) -> AppResult<(String, String)> {
    let old_start = parse_event_time(old_start, "start")?;
    let old_end = parse_event_time(old_end, "end")?;

    let start = new_start.unwrap_or(old_start);
    let end = match (new_end, duration) {
        (Some(end), _) => end,
        (None, Some(minutes)) => start + Duration::minutes(i64::from(minutes)),
        (None, None) => start + (old_end - old_start),
    };
    if end <= start {
        return Err(AppError::InvalidInput(
            "event end must be after its start".to_string(),
        ));
    }

    Ok((start.to_rfc3339(), end.to_rfc3339()))
}

/// Parse an event boundary from the API; bare dates (all-day events) and
/// missing boundaries are rejected since edit only handles timed events.
fn parse_event_time(value: Option<&str>, label: &str) -> AppResult<DateTime<Local>> {
    value
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Local))
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "the event's {label} has no timestamp (all-day events are not supported by edit)"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(s: &str) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Local)
    }

    const OLD_START: &str = "2026-08-20T10:00:00-07:00";
    const OLD_END: &str = "2026-08-20T10:30:00-07:00";

    #[test]
    fn start_only_shift_preserves_duration() {
        let (start, end) = resolve_times(
            Some(OLD_START),
            Some(OLD_END),
            Some(local("2026-08-20T11:00:00-07:00")),
            None,
            None,
        )
        .unwrap();
        assert_eq!(local(&start), local("2026-08-20T11:00:00-07:00"));
        assert_eq!(local(&end), local("2026-08-20T11:30:00-07:00"));
    }

    #[test]
    fn duration_only_extends_from_existing_start() {
        let (start, end) =
            resolve_times(Some(OLD_START), Some(OLD_END), None, None, Some(60)).unwrap();
        assert_eq!(local(&start), local(OLD_START));
        assert_eq!(local(&end), local("2026-08-20T11:00:00-07:00"));
    }

    #[test]
    fn end_only_keeps_existing_start() {
        let (start, end) = resolve_times(
            Some(OLD_START),
            Some(OLD_END),
            None,
            Some(local("2026-08-20T12:00:00-07:00")),
            None,
        )
        .unwrap();
        assert_eq!(local(&start), local(OLD_START));
        assert_eq!(local(&end), local("2026-08-20T12:00:00-07:00"));
    }

    #[test]
    fn start_with_duration_uses_new_start() {
        let (start, end) = resolve_times(
            Some(OLD_START),
            Some(OLD_END),
            Some(local("2026-08-20T14:00:00-07:00")),
            None,
            Some(45),
        )
        .unwrap();
        assert_eq!(local(&start), local("2026-08-20T14:00:00-07:00"));
        assert_eq!(local(&end), local("2026-08-20T14:45:00-07:00"));
    }

    #[test]
    fn end_before_start_is_rejected() {
        let err = resolve_times(
            Some(OLD_START),
            Some(OLD_END),
            None,
            Some(local("2026-08-20T09:00:00-07:00")),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("end must be after"));
    }

    #[test]
    fn all_day_event_is_rejected() {
        let err = resolve_times(
            Some("2026-08-20"),
            Some("2026-08-21"),
            Some(local(OLD_START)),
            None,
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("all-day"));
    }
}
