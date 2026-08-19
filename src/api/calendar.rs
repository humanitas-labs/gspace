use serde::Deserialize;
use serde::Serialize;

use crate::error::AppResult;

use super::http::JsonClient;
use super::models::EventView;

const CALENDAR_API_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";
const CALENDAR_AUTH_HINT: &str = "run `gmail auth login` (profiles authorized before calendar \
                                  support need a one-time re-login to grant the calendar scope)";

/// A new event to be inserted, as assembled by the `gcal add` command.
///
/// `start`/`end` are RFC 3339 timestamps with offset. When `meet` is set the
/// insert request asks Calendar to mint a Google Meet conference, using
/// `request_id` as the idempotency key for conference creation.
#[derive(Debug, Clone)]
pub struct EventDraft {
    pub summary: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: String,
    pub end: String,
    pub attendees: Vec<String>,
    pub meet: bool,
    pub request_id: String,
}

/// Field-level changes for `events.patch`, as assembled by the `gcal edit`
/// command. `None` fields are omitted from the request body and left untouched
/// on the event; `start`/`end` are RFC 3339 timestamps with offset.
#[derive(Debug, Clone, Default)]
pub struct EventPatch {
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub attendees: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct CalendarClient {
    inner: JsonClient,
}

impl CalendarClient {
    /// Construct a client targeting the public Calendar v3 API base URL.
    pub fn new() -> Self {
        Self {
            inner: JsonClient::new(CALENDAR_API_BASE_URL, "calendar", CALENDAR_AUTH_HINT),
        }
    }

    /// Insert an event, sending invite emails to all attendees. Returns the
    /// created event including its Meet link when one was requested.
    pub async fn insert_event(
        &self,
        calendar_id: &str,
        draft: &EventDraft,
        access_token: &str,
    ) -> AppResult<EventView> {
        let endpoint = events_endpoint(calendar_id);
        let query = vec![
            ("conferenceDataVersion".to_string(), "1".to_string()),
            ("sendUpdates".to_string(), "all".to_string()),
        ];
        let body = CalendarEventRequest::from_draft(draft);
        let resource: CalendarEventResource = self
            .inner
            .post_json(&endpoint, access_token, &body, Some(&query))
            .await?;
        Ok(resource.into_view())
    }

    /// List events in `[time_min, time_max)` ordered by start time, expanding
    /// recurring events into their individual instances.
    pub async fn list_events(
        &self,
        calendar_id: &str,
        time_min: &str,
        time_max: &str,
        limit: u32,
        access_token: &str,
    ) -> AppResult<Vec<EventView>> {
        let endpoint = events_endpoint(calendar_id);
        let query = vec![
            ("timeMin".to_string(), time_min.to_string()),
            ("timeMax".to_string(), time_max.to_string()),
            ("singleEvents".to_string(), "true".to_string()),
            ("orderBy".to_string(), "startTime".to_string()),
            ("maxResults".to_string(), limit.to_string()),
        ];
        let resource: CalendarEventListResource = self
            .inner
            .get_json(&endpoint, access_token, Some(&query))
            .await?;

        Ok(resource
            .items
            .unwrap_or_default()
            .into_iter()
            .filter(|item| item.status.as_deref() != Some("cancelled"))
            .map(CalendarEventResource::into_view)
            .collect())
    }

    /// Fetch a single event by id.
    pub async fn get_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        access_token: &str,
    ) -> AppResult<EventView> {
        let endpoint = format!("{}/{event_id}", events_endpoint(calendar_id));
        let resource: CalendarEventResource =
            self.inner.get_json(&endpoint, access_token, None).await?;
        Ok(resource.into_view())
    }

    /// Patch only the fields set on `patch`, leaving everything else (including
    /// the event id and any Meet conference) intact. Attendees receive a single
    /// "updated event" email.
    pub async fn patch_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        patch: &EventPatch,
        access_token: &str,
    ) -> AppResult<EventView> {
        let endpoint = format!("{}/{event_id}", events_endpoint(calendar_id));
        let query = vec![("sendUpdates".to_string(), "all".to_string())];
        let body = CalendarEventPatchRequest::from_patch(patch);
        let resource: CalendarEventResource = self
            .inner
            .patch_json(&endpoint, access_token, &body, Some(&query))
            .await?;
        Ok(resource.into_view())
    }

    /// Delete an event, sending cancellation emails to all attendees.
    pub async fn delete_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        access_token: &str,
    ) -> AppResult<()> {
        let endpoint = format!("{}/{event_id}", events_endpoint(calendar_id));
        let query = vec![("sendUpdates".to_string(), "all".to_string())];
        self.inner.delete(&endpoint, access_token, Some(&query)).await
    }
}

impl Default for CalendarClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Endpoint path for a calendar's events collection.
fn events_endpoint(calendar_id: &str) -> String {
    format!("/calendars/{calendar_id}/events")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventRequest {
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    start: CalendarEventTimeRequest,
    end: CalendarEventTimeRequest,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attendees: Vec<CalendarAttendeeRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conference_data: Option<CalendarConferenceRequest>,
}

impl CalendarEventRequest {
    /// Build the insert body from a draft, attaching a Meet conference request
    /// only when the draft asked for one.
    fn from_draft(draft: &EventDraft) -> Self {
        let conference_data = draft.meet.then(|| CalendarConferenceRequest {
            create_request: CalendarConferenceCreateRequest {
                request_id: draft.request_id.clone(),
                conference_solution_key: CalendarConferenceSolutionKey {
                    kind: "hangoutsMeet".to_string(),
                },
            },
        });

        Self {
            summary: draft.summary.clone(),
            location: draft.location.clone(),
            description: draft.description.clone(),
            start: CalendarEventTimeRequest {
                date_time: draft.start.clone(),
            },
            end: CalendarEventTimeRequest {
                date_time: draft.end.clone(),
            },
            attendees: draft
                .attendees
                .iter()
                .map(|email| CalendarAttendeeRequest {
                    email: email.clone(),
                })
                .collect(),
            conference_data,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<CalendarEventTimeRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<CalendarEventTimeRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attendees: Option<Vec<CalendarAttendeeRequest>>,
}

impl CalendarEventPatchRequest {
    /// Build the patch body, mapping only the fields set on the patch.
    fn from_patch(patch: &EventPatch) -> Self {
        Self {
            summary: patch.summary.clone(),
            location: patch.location.clone(),
            description: patch.description.clone(),
            start: patch.start.clone().map(|date_time| CalendarEventTimeRequest { date_time }),
            end: patch.end.clone().map(|date_time| CalendarEventTimeRequest { date_time }),
            attendees: patch.attendees.as_ref().map(|emails| {
                emails
                    .iter()
                    .map(|email| CalendarAttendeeRequest {
                        email: email.clone(),
                    })
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventTimeRequest {
    date_time: String,
}

#[derive(Debug, Serialize)]
struct CalendarAttendeeRequest {
    email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarConferenceRequest {
    create_request: CalendarConferenceCreateRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarConferenceCreateRequest {
    request_id: String,
    conference_solution_key: CalendarConferenceSolutionKey,
}

#[derive(Debug, Serialize)]
struct CalendarConferenceSolutionKey {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventListResource {
    items: Option<Vec<CalendarEventResource>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventResource {
    id: String,
    summary: Option<String>,
    status: Option<String>,
    location: Option<String>,
    start: Option<CalendarEventTimeResource>,
    end: Option<CalendarEventTimeResource>,
    attendees: Option<Vec<CalendarAttendeeResource>>,
    hangout_link: Option<String>,
    html_link: Option<String>,
}

impl CalendarEventResource {
    /// Flatten the raw resource into an `EventView`, taking whichever of
    /// `dateTime` (timed) or `date` (all-day) each boundary carries.
    fn into_view(self) -> EventView {
        EventView {
            id: self.id,
            summary: self.summary,
            status: self.status,
            location: self.location,
            start: self.start.and_then(CalendarEventTimeResource::into_string),
            end: self.end.and_then(CalendarEventTimeResource::into_string),
            attendees: self
                .attendees
                .unwrap_or_default()
                .into_iter()
                .map(|attendee| attendee.email)
                .collect(),
            meet_link: self.hangout_link,
            html_link: self.html_link,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventTimeResource {
    date_time: Option<String>,
    date: Option<String>,
}

impl CalendarEventTimeResource {
    /// The timestamp for a timed event, or the bare date for an all-day event.
    fn into_string(self) -> Option<String> {
        self.date_time.or(self.date)
    }
}

#[derive(Debug, Deserialize)]
struct CalendarAttendeeResource {
    email: String,
}
