use clap::Parser;
use gspace::commands::calendar::{format_when, parse_date_or_datetime, parse_datetime, resolve_window};
use gspace::gcal::{GcalCli, GcalCommand};

#[test]
fn parses_add_with_attendees_and_meet() {
    let cli = GcalCli::try_parse_from([
        "gcal",
        "add",
        "--title",
        "weekly sync",
        "--start",
        "2026-07-28 10:00",
        "--attendees",
        "alex@example.com,sam@example.com",
        "--meet",
    ])
    .expect("args should parse");

    match cli.command {
        GcalCommand::Add(args) => {
            assert_eq!(args.title, "weekly sync");
            assert_eq!(args.duration, 30);
            assert_eq!(args.attendees, vec!["alex@example.com", "sam@example.com"]);
            assert!(args.meet);
            assert_eq!(args.calendar, "primary");
        }
        other => panic!("expected add, got {other:?}"),
    }
}

#[test]
fn add_rejects_end_with_duration() {
    let result = GcalCli::try_parse_from([
        "gcal",
        "add",
        "--title",
        "x",
        "--start",
        "2026-07-28 10:00",
        "--end",
        "2026-07-28 11:00",
        "--duration",
        "45",
    ]);

    assert!(result.is_err(), "--end and --duration must conflict");
}

#[test]
fn parses_edit_with_all_field_flags() {
    let cli = GcalCli::try_parse_from([
        "gcal",
        "edit",
        "evt-1",
        "--title",
        "renamed sync",
        "--start",
        "tomorrow 11:00",
        "--duration",
        "45",
        "--attendees",
        "alex@example.com,sam@example.com",
        "--location",
        "Meet",
        "--notes",
        "moved per dan",
    ])
    .expect("args should parse");

    match cli.command {
        GcalCommand::Edit(args) => {
            assert_eq!(args.event_id, "evt-1");
            assert_eq!(args.title.as_deref(), Some("renamed sync"));
            assert_eq!(args.start.as_deref(), Some("tomorrow 11:00"));
            assert_eq!(args.duration, Some(45));
            assert_eq!(
                args.attendees,
                Some(vec!["alex@example.com".to_string(), "sam@example.com".to_string()])
            );
            assert_eq!(args.location.as_deref(), Some("Meet"));
            assert_eq!(args.notes.as_deref(), Some("moved per dan"));
            assert_eq!(args.calendar, "primary");
        }
        other => panic!("expected edit, got {other:?}"),
    }
}

#[test]
fn edit_rejects_end_with_duration() {
    let result = GcalCli::try_parse_from([
        "gcal",
        "edit",
        "evt-1",
        "--end",
        "2026-07-28 11:00",
        "--duration",
        "45",
    ]);

    assert!(result.is_err(), "--end and --duration must conflict");
}

#[test]
fn edit_requires_event_id() {
    assert!(GcalCli::try_parse_from(["gcal", "edit", "--title", "x"]).is_err());
}

#[test]
fn list_window_flags_conflict() {
    assert!(GcalCli::try_parse_from(["gcal", "list", "--today", "--week"]).is_err());
    assert!(GcalCli::try_parse_from(["gcal", "list", "--from", "2026-07-28"]).is_err());
    assert!(GcalCli::try_parse_from(["gcal", "list", "--today"]).is_ok());
    assert!(
        GcalCli::try_parse_from(["gcal", "list", "--from", "2026-07-28", "--to", "2026-07-30"])
            .is_ok()
    );
}

#[test]
fn parses_naive_datetime_forms() {
    let space = parse_datetime("2026-07-28 10:00").expect("space form should parse");
    let tee = parse_datetime("2026-07-28T10:00").expect("T form should parse");
    assert_eq!(space, tee);
    assert_eq!(space.format("%Y-%m-%d %H:%M").to_string(), "2026-07-28 10:00");
}

#[test]
fn parses_rfc3339_datetime() {
    use chrono::{TimeZone, Utc};

    let parsed = parse_datetime("2026-07-28T10:00:00-07:00").expect("rfc3339 should parse");
    let expected = Utc
        .with_ymd_and_hms(2026, 7, 28, 17, 0, 0)
        .single()
        .expect("expected instant is unambiguous");
    assert_eq!(parsed.with_timezone(&Utc), expected);
}

#[test]
fn parses_relative_forms() {
    assert!(parse_datetime("today 10:00").is_ok());
    assert!(parse_datetime("tomorrow 09:30").is_ok());
    let today = parse_datetime("today 10:00").expect("today should parse");
    let tomorrow = parse_datetime("tomorrow 10:00").expect("tomorrow should parse");
    assert_eq!((tomorrow - today).num_hours(), 24);
}

#[test]
fn rejects_garbage_datetime() {
    assert!(parse_datetime("next thursday-ish").is_err());
    assert!(parse_datetime("").is_err());
    assert!(parse_datetime("2026-07-28").is_err(), "add requires a time of day");
}

#[test]
fn date_only_allowed_for_list_bounds() {
    let midnight = parse_date_or_datetime("2026-07-28").expect("bare date should parse");
    assert_eq!(midnight.format("%H:%M").to_string(), "00:00");
}

#[test]
fn window_rejects_inverted_range() {
    let result = resolve_window(false, false, Some("2026-07-30"), Some("2026-07-28"));
    assert!(result.is_err(), "--to before --from must error");
}

#[test]
fn formats_same_day_range_compactly() {
    // Rendering converts to the machine's local timezone, which can move the
    // range across a date boundary — assert on the shape, not clock values.
    let when = format_when(
        Some("2026-07-28T10:00:00-07:00"),
        Some("2026-07-28T10:30:00-07:00"),
    );
    assert!(when.contains('–') || when.contains('→'), "got: {when}");
}

#[test]
fn formats_all_day_event() {
    assert_eq!(format_when(Some("2026-08-01"), Some("2026-08-01")), "2026-08-01 (all day)");
    assert_eq!(format_when(None, None), "(no time)");
}
