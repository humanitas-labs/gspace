pub mod add;
pub mod list;
pub mod rm;

use chrono::{DateTime, Duration, Local, NaiveDateTime, NaiveTime, TimeZone};

use crate::error::{AppError, AppResult};

/// Parse a user-supplied datetime in local time.
///
/// Accepted forms: RFC 3339 (`2026-07-28T10:00:00-07:00`), `YYYY-MM-DD HH:MM`
/// (space or `T` separator, optional seconds), and the relative forms
/// `today HH:MM` / `tomorrow HH:MM`.
pub fn parse_datetime(input: &str) -> AppResult<DateTime<Local>> {
    let trimmed = input.trim();

    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(parsed.with_timezone(&Local));
    }

    for format in ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, format) {
            return local_from_naive(naive);
        }
    }

    if let Some((word, time)) = trimmed.split_once(' ') {
        let day_offset = match word.to_ascii_lowercase().as_str() {
            "today" => Some(0),
            "tomorrow" => Some(1),
            _ => None,
        };
        if let Some(offset) = day_offset
            && let Ok(time) = NaiveTime::parse_from_str(time.trim(), "%H:%M")
        {
            let date = Local::now().date_naive() + Duration::days(offset);
            return local_from_naive(date.and_time(time));
        }
    }

    Err(AppError::InvalidInput(format!(
        "could not parse datetime `{trimmed}`; accepted forms: RFC 3339, \
         `YYYY-MM-DD HH:MM`, `today HH:MM`, `tomorrow HH:MM`"
    )))
}

/// Like [`parse_datetime`], but also accepts a bare `YYYY-MM-DD`, interpreted
/// as local midnight. Used for list-window bounds where a date is enough.
pub fn parse_date_or_datetime(input: &str) -> AppResult<DateTime<Local>> {
    let trimmed = input.trim();
    if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return local_from_naive(date.and_time(NaiveTime::MIN));
    }
    parse_datetime(trimmed)
}

/// The `[time_min, time_max)` window and a human label for a list request.
#[derive(Debug, Clone)]
pub struct ListWindow {
    pub time_min: DateTime<Local>,
    pub time_max: DateTime<Local>,
    pub label: String,
}

/// Resolve the list-window flags into concrete bounds. Precedence: an explicit
/// `--from`/`--to` range, then `--today`/`--tomorrow`, then the default of the
/// next 7 days starting now (`--week` names the same default explicitly).
pub fn resolve_window(
    today: bool,
    tomorrow: bool,
    from: Option<&str>,
    to: Option<&str>,
) -> AppResult<ListWindow> {
    if let (Some(from), Some(to)) = (from, to) {
        let time_min = parse_date_or_datetime(from)?;
        let time_max = parse_date_or_datetime(to)?;
        if time_max <= time_min {
            return Err(AppError::InvalidInput(
                "--to must be after --from".to_string(),
            ));
        }
        return Ok(ListWindow {
            time_min,
            time_max,
            label: format!(
                "{} → {}",
                time_min.format("%Y.%m.%d %H:%M"),
                time_max.format("%Y.%m.%d %H:%M")
            ),
        });
    }

    let today_start = local_from_naive(Local::now().date_naive().and_time(NaiveTime::MIN))?;
    if today {
        return Ok(ListWindow {
            time_min: today_start,
            time_max: today_start + Duration::days(1),
            label: "today".to_string(),
        });
    }
    if tomorrow {
        return Ok(ListWindow {
            time_min: today_start + Duration::days(1),
            time_max: today_start + Duration::days(2),
            label: "tomorrow".to_string(),
        });
    }

    let now = Local::now();
    Ok(ListWindow {
        time_min: now,
        time_max: now + Duration::days(7),
        label: "next 7 days".to_string(),
    })
}

/// Format an event's start/end (RFC 3339 or bare-date strings from the API)
/// for terminal display, collapsing same-day ranges to a single date.
pub fn format_when(start: Option<&str>, end: Option<&str>) -> String {
    let start_dt = start.and_then(|s| DateTime::parse_from_rfc3339(s).ok());
    let end_dt = end.and_then(|s| DateTime::parse_from_rfc3339(s).ok());

    match (start_dt, end_dt) {
        (Some(start), Some(end)) => {
            let start = start.with_timezone(&Local);
            let end = end.with_timezone(&Local);
            if start.date_naive() == end.date_naive() {
                format!(
                    "{}–{}",
                    start.format("%a %Y.%m.%d %H:%M"),
                    end.format("%H:%M")
                )
            } else {
                format!(
                    "{} → {}",
                    start.format("%a %Y.%m.%d %H:%M"),
                    end.format("%a %Y.%m.%d %H:%M")
                )
            }
        }
        // All-day events carry bare dates; anything unparsed is shown raw.
        _ => match (start, end) {
            (Some(start), Some(end)) if start == end => format!("{start} (all day)"),
            (Some(start), Some(end)) => format!("{start} → {end}"),
            (Some(start), None) => start.to_string(),
            _ => "(no time)".to_string(),
        },
    }
}

/// Resolve a naive local datetime to an instant, taking the earlier reading in
/// a DST fold and erroring on times skipped by a DST gap.
fn local_from_naive(naive: NaiveDateTime) -> AppResult<DateTime<Local>> {
    Local
        .from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "`{naive}` does not exist in the local timezone (DST gap)"
            ))
        })
}
