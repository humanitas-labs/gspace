mod error {
    pub use gspace::error::*;
}

mod http {
    pub use gspace::api::http::*;
}

mod models {
    pub use gspace::api::models::*;
}

mod calendar_under_test {
    #![allow(dead_code)]

    include!("../src/api/calendar.rs");

    fn draft(meet: bool) -> EventDraft {
        EventDraft {
            summary: "weekly sync".to_string(),
            location: Some("Meet".to_string()),
            description: None,
            start: "2026-07-28T10:00:00-07:00".to_string(),
            end: "2026-07-28T10:30:00-07:00".to_string(),
            attendees: vec!["alex@example.com".to_string()],
            meet,
            request_id: "req-123".to_string(),
        }
    }

    #[test]
    fn insert_body_with_meet_carries_conference_request() {
        let body = serde_json::to_value(CalendarEventRequest::from_draft(&draft(true)))
            .expect("body should serialize");

        assert_eq!(body["summary"], "weekly sync");
        assert_eq!(body["start"]["dateTime"], "2026-07-28T10:00:00-07:00");
        assert_eq!(body["end"]["dateTime"], "2026-07-28T10:30:00-07:00");
        assert_eq!(body["attendees"][0]["email"], "alex@example.com");
        assert_eq!(body["conferenceData"]["createRequest"]["requestId"], "req-123");
        assert_eq!(
            body["conferenceData"]["createRequest"]["conferenceSolutionKey"]["type"],
            "hangoutsMeet"
        );
        assert!(body.get("description").is_none());
    }

    #[test]
    fn insert_body_without_meet_omits_conference_data() {
        let body = serde_json::to_value(CalendarEventRequest::from_draft(&draft(false)))
            .expect("body should serialize");

        assert!(body.get("conferenceData").is_none());
    }

    #[test]
    fn patch_body_carries_only_set_fields() {
        let patch = EventPatch {
            summary: Some("renamed sync".to_string()),
            start: Some("2026-07-28T11:00:00-07:00".to_string()),
            end: Some("2026-07-28T11:30:00-07:00".to_string()),
            ..EventPatch::default()
        };
        let body = serde_json::to_value(CalendarEventPatchRequest::from_patch(&patch))
            .expect("body should serialize");

        assert_eq!(body["summary"], "renamed sync");
        assert_eq!(body["start"]["dateTime"], "2026-07-28T11:00:00-07:00");
        assert_eq!(body["end"]["dateTime"], "2026-07-28T11:30:00-07:00");
        assert!(body.get("location").is_none());
        assert!(body.get("description").is_none());
        assert!(body.get("attendees").is_none());
    }

    #[test]
    fn patch_body_maps_attendee_emails() {
        let patch = EventPatch {
            attendees: Some(vec!["alex@example.com".to_string(), "sam@example.com".to_string()]),
            ..EventPatch::default()
        };
        let body = serde_json::to_value(CalendarEventPatchRequest::from_patch(&patch))
            .expect("body should serialize");

        assert_eq!(body["attendees"][0]["email"], "alex@example.com");
        assert_eq!(body["attendees"][1]["email"], "sam@example.com");
        assert!(body.get("summary").is_none());
        assert!(body.get("start").is_none());
        assert!(body.get("end").is_none());
    }

    #[test]
    fn maps_timed_event_resource_to_view() {
        let resource: CalendarEventResource = serde_json::from_str(
            r#"{
                "id": "evt-1",
                "summary": "weekly sync",
                "status": "confirmed",
                "start": {"dateTime": "2026-07-28T10:00:00-07:00"},
                "end": {"dateTime": "2026-07-28T10:30:00-07:00"},
                "attendees": [{"email": "alex@example.com"}, {"email": "jane@example.net"}],
                "hangoutLink": "https://meet.google.com/abc-defg-hij",
                "htmlLink": "https://www.google.com/calendar/event?eid=xyz"
            }"#,
        )
        .expect("resource should deserialize");

        let view = resource.into_view();
        assert_eq!(view.id, "evt-1");
        assert_eq!(view.summary.as_deref(), Some("weekly sync"));
        assert_eq!(view.start.as_deref(), Some("2026-07-28T10:00:00-07:00"));
        assert_eq!(view.end.as_deref(), Some("2026-07-28T10:30:00-07:00"));
        assert_eq!(view.attendees.len(), 2);
        assert_eq!(
            view.meet_link.as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
    }

    #[test]
    fn maps_all_day_event_dates() {
        let resource: CalendarEventResource = serde_json::from_str(
            r#"{
                "id": "evt-2",
                "summary": "offsite",
                "start": {"date": "2026-08-01"},
                "end": {"date": "2026-08-02"}
            }"#,
        )
        .expect("resource should deserialize");

        let view = resource.into_view();
        assert_eq!(view.start.as_deref(), Some("2026-08-01"));
        assert_eq!(view.end.as_deref(), Some("2026-08-02"));
        assert!(view.attendees.is_empty());
        assert!(view.meet_link.is_none());
    }
}
