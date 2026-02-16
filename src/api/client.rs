use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{AppError, AppResult};

use super::labels;
use super::messages;
use super::models::{LabelMutationResult, LabelView, MessageView, SendResult};

const GMAIL_API_BASE_URL: &str = "https://gmail.googleapis.com";

#[derive(Debug, Clone)]
pub struct GmailClient {
    http: Client,
    base_url: String,
}

impl GmailClient {
    pub fn new() -> Self {
        Self {
            http: Client::new(),
            base_url: GMAIL_API_BASE_URL.to_string(),
        }
    }

    pub async fn get_msg(&self, id: &str, access_token: &str) -> AppResult<MessageView> {
        let endpoint = messages::message_endpoint(id);
        let query = messages::get_query();
        let resource: GmailMessageResource =
            self.get_json(&endpoint, access_token, Some(&query)).await?;
        Ok(resource.into_view())
    }

    pub async fn list(
        &self,
        access_token: &str,
        limit: u32,
        query: Option<&str>,
    ) -> AppResult<Vec<MessageView>> {
        let endpoint = messages::list_endpoint();
        let query_params = messages::list_query(limit, query);
        let list_resource: GmailMessageListResource = self
            .get_json(endpoint, access_token, Some(&query_params))
            .await?;

        let mut results = Vec::new();
        for entry in list_resource.messages.unwrap_or_default() {
            let message = self.get_msg(&entry.id, access_token).await?;
            results.push(message);
        }

        Ok(results)
    }

    pub async fn send(
        &self,
        raw_message: &str,
        thread_id: Option<&str>,
        access_token: &str,
    ) -> AppResult<SendResult> {
        let endpoint = messages::send_endpoint();
        let request = GmailSendRequest {
            raw: raw_message.to_string(),
            thread_id: thread_id.map(ToOwned::to_owned),
        };
        let response: GmailSendResponse = self.post_json(endpoint, access_token, &request).await?;

        Ok(SendResult {
            id: response.id,
            thread_id: response.thread_id,
            note: "message accepted by gmail api".to_string(),
        })
    }

    pub async fn list_labels(&self, _access_token: &str) -> AppResult<Vec<LabelView>> {
        let endpoint = labels::list_labels_endpoint();
        let response: GmailLabelListResponse = self.get_json(endpoint, _access_token, None).await?;
        let mut labels_out = response
            .labels
            .unwrap_or_default()
            .into_iter()
            .map(|label| LabelView {
                id: label.id,
                name: label.name,
                kind: label.kind,
            })
            .collect::<Vec<_>>();
        labels_out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(labels_out)
    }

    pub async fn add_labels(
        &self,
        id: &str,
        labels: &[String],
        access_token: &str,
    ) -> AppResult<LabelMutationResult> {
        self.modify_labels(id, labels, &[], access_token).await
    }

    pub async fn rm_labels(
        &self,
        id: &str,
        labels: &[String],
        access_token: &str,
    ) -> AppResult<LabelMutationResult> {
        self.modify_labels(id, &[], labels, access_token).await
    }

    async fn modify_labels(
        &self,
        id: &str,
        add: &[String],
        rm: &[String],
        access_token: &str,
    ) -> AppResult<LabelMutationResult> {
        let resolved_add = self.resolve_label_ids(add, access_token).await?;
        let resolved_rm = self.resolve_label_ids(rm, access_token).await?;

        let endpoint = labels::modify_labels_endpoint(id);
        let body = GmailModifyLabelsRequest {
            add_label_ids: resolved_add.clone(),
            remove_label_ids: resolved_rm.clone(),
        };

        let _: GmailModifyLabelsResponse = self.post_json(&endpoint, access_token, &body).await?;
        Ok(LabelMutationResult {
            id: id.to_string(),
            added: resolved_add,
            removed: resolved_rm,
            note: "message labels updated".to_string(),
        })
    }

    async fn resolve_label_ids(
        &self,
        requested: &[String],
        access_token: &str,
    ) -> AppResult<Vec<String>> {
        if requested.is_empty() {
            return Ok(Vec::new());
        }

        let known = self.list_labels(access_token).await?;
        let mut out = Vec::new();

        for raw in requested {
            let needle = raw.trim();
            if needle.is_empty() {
                continue;
            }

            let mut matched = None;
            for label in &known {
                if label.id == needle || label.name.eq_ignore_ascii_case(needle) {
                    matched = Some(label.id.clone());
                    break;
                }
            }

            let Some(label_id) = matched else {
                return Err(AppError::InvalidInput(format!(
                    "unknown label `{needle}`; run `gmail label ls` to inspect labels"
                )));
            };

            if !out.contains(&label_id) {
                out.push(label_id);
            }
        }

        Ok(out)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        access_token: &str,
        query: Option<&[(String, String)]>,
    ) -> AppResult<T> {
        let url = self.endpoint_url(endpoint)?;
        let mut request = self.http.get(url).bearer_auth(access_token);
        if let Some(query) = query {
            request = request.query(query);
        }

        let response = request.send().await?;
        self.parse_json_response(response).await
    }

    async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        endpoint: &str,
        access_token: &str,
        body: &B,
    ) -> AppResult<T> {
        let url = self.endpoint_url(endpoint)?;
        let response = self
            .http
            .post(url)
            .bearer_auth(access_token)
            .json(body)
            .send()
            .await?;

        self.parse_json_response(response).await
    }

    fn endpoint_url(&self, endpoint: &str) -> AppResult<Url> {
        let mut url = Url::parse(&self.base_url)?;
        url.set_path(endpoint.trim_start_matches('/'));
        Ok(url)
    }

    async fn parse_json_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> AppResult<T> {
        let status = response.status();
        if status.is_success() {
            return Ok(response.json().await?);
        }

        let body = response.text().await.unwrap_or_default();
        Err(map_api_error(status, &body))
    }
}

impl Default for GmailClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct GmailMessageResource {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
    snippet: Option<String>,
    payload: Option<GmailMessagePayload>,
}

impl GmailMessageResource {
    fn into_view(self) -> MessageView {
        let headers = self
            .payload
            .and_then(|payload| payload.headers)
            .unwrap_or_default();

        MessageView {
            id: self.id,
            thread_id: self.thread_id,
            snippet: self.snippet,
            subject: header_value(&headers, "Subject"),
            from: header_value(&headers, "From"),
            reply_to: header_value(&headers, "Reply-To"),
            date: header_value(&headers, "Date"),
            message_id: header_value(&headers, "Message-ID"),
            in_reply_to: header_value(&headers, "In-Reply-To"),
            references: header_value(&headers, "References"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GmailMessagePayload {
    headers: Option<Vec<GmailMessageHeader>>,
}

#[derive(Debug, Deserialize)]
struct GmailMessageListResource {
    messages: Option<Vec<GmailMessageListEntry>>,
}

#[derive(Debug, Deserialize)]
struct GmailMessageListEntry {
    id: String,
}

#[derive(Debug, Serialize)]
struct GmailSendRequest {
    raw: String,
    #[serde(rename = "threadId", skip_serializing_if = "Option::is_none")]
    thread_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GmailSendResponse {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GmailLabelListResponse {
    labels: Option<Vec<GmailLabelResource>>,
}

#[derive(Debug, Deserialize)]
struct GmailLabelResource {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Serialize)]
struct GmailModifyLabelsRequest {
    #[serde(rename = "addLabelIds")]
    add_label_ids: Vec<String>,
    #[serde(rename = "removeLabelIds")]
    remove_label_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GmailModifyLabelsResponse {}

#[derive(Debug, Deserialize)]
struct GmailMessageHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct GmailApiErrorEnvelope {
    error: GmailApiError,
}

#[derive(Debug, Deserialize)]
struct GmailApiError {
    code: Option<u16>,
    status: Option<String>,
    message: Option<String>,
    errors: Option<Vec<GmailApiErrorDetail>>,
}

#[derive(Debug, Deserialize)]
struct GmailApiErrorDetail {
    reason: Option<String>,
}

fn header_value(headers: &[GmailMessageHeader], target: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(target))
        .map(|header| header.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn map_api_error(status: StatusCode, body: &str) -> AppError {
    let message = parse_api_error_message(body).unwrap_or_else(|| {
        let body = body.trim();
        if body.is_empty() {
            "no error details in response body".to_string()
        } else {
            body.to_string()
        }
    });

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return AppError::Auth(format!(
            "gmail api authorization failed ({status}): {message}. run `gmail auth login`"
        ));
    }

    AppError::Api(format!("gmail api request failed ({status}): {message}"))
}

fn parse_api_error_message(body: &str) -> Option<String> {
    let envelope = serde_json::from_str::<GmailApiErrorEnvelope>(body).ok()?;
    let mut parts = Vec::new();

    if let Some(message) = envelope.error.message {
        parts.push(message);
    }

    if let Some(status) = envelope.error.status {
        parts.push(format!("status={status}"));
    }

    if let Some(code) = envelope.error.code {
        parts.push(format!("code={code}"));
    }

    if let Some(reason) = envelope
        .error
        .errors
        .and_then(|errors| errors.into_iter().find_map(|detail| detail.reason))
    {
        parts.push(format!("reason={reason}"));
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_message_resource_to_view() {
        let resource = GmailMessageResource {
            id: "msg-123".to_string(),
            thread_id: Some("thread-456".to_string()),
            snippet: Some("hello world".to_string()),
            payload: Some(GmailMessagePayload {
                headers: Some(vec![
                    GmailMessageHeader {
                        name: "Subject".to_string(),
                        value: "hello".to_string(),
                    },
                    GmailMessageHeader {
                        name: "From".to_string(),
                        value: "dev@example.com".to_string(),
                    },
                    GmailMessageHeader {
                        name: "Date".to_string(),
                        value: "Mon, 16 Feb 2026 10:00:00 +0000".to_string(),
                    },
                    GmailMessageHeader {
                        name: "Message-ID".to_string(),
                        value: "<abc@example.com>".to_string(),
                    },
                ]),
            }),
        };

        let view = resource.into_view();
        assert_eq!(view.id, "msg-123");
        assert_eq!(view.thread_id.as_deref(), Some("thread-456"));
        assert_eq!(view.subject.as_deref(), Some("hello"));
        assert_eq!(view.from.as_deref(), Some("dev@example.com"));
        assert_eq!(view.message_id.as_deref(), Some("<abc@example.com>"));
    }

    #[test]
    fn header_lookup_is_case_insensitive() {
        let headers = vec![GmailMessageHeader {
            name: "sUbJeCt".to_string(),
            value: "case test".to_string(),
        }];

        assert_eq!(
            header_value(&headers, "Subject").as_deref(),
            Some("case test")
        );
    }

    #[test]
    fn maps_unauthorized_as_auth_error() {
        let error = map_api_error(
            StatusCode::UNAUTHORIZED,
            r#"{"error":{"code":401,"message":"Request had invalid authentication credentials.","status":"UNAUTHENTICATED"}}"#,
        );

        match error {
            AppError::Auth(message) => {
                assert!(message.contains("invalid authentication credentials"));
            }
            other => panic!("expected auth error, got {other:?}"),
        }
    }

    #[test]
    fn maps_not_found_as_api_error() {
        let error = map_api_error(
            StatusCode::NOT_FOUND,
            r#"{"error":{"code":404,"message":"Requested entity was not found.","status":"NOT_FOUND"}}"#,
        );

        match error {
            AppError::Api(message) => {
                assert!(message.contains("Requested entity was not found"));
            }
            other => panic!("expected api error, got {other:?}"),
        }
    }
}
