use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{AppError, AppResult};

use super::labels;
use super::messages;
use super::models::{
    AttachmentList, AttachmentMeta, LabelMutationResult, LabelView, MessageView, SendAsView,
    SendResult,
};
use super::send_as;

const GMAIL_API_BASE_URL: &str = "https://gmail.googleapis.com";

#[derive(Debug, Clone)]
pub struct GmailClient {
    http: Client,
    base_url: String,
}

impl GmailClient {
    /// Construct a client targeting the public Gmail API base URL.
    pub fn new() -> Self {
        Self {
            http: Client::new(),
            base_url: GMAIL_API_BASE_URL.to_string(),
        }
    }

    /// Fetch a single message with `format=metadata` and project it into a `MessageView`.
    pub async fn get_msg(&self, id: &str, access_token: &str) -> AppResult<MessageView> {
        let endpoint = messages::message_endpoint(id);
        let query = messages::get_query();
        let resource: GmailMessageResource =
            self.get_json(&endpoint, access_token, Some(&query)).await?;
        Ok(resource.into_view())
    }

    /// Fetch a single message with `format=full`, projecting it into a
    /// `MessageView` that includes the decoded text body.
    pub async fn get_msg_full(&self, id: &str, access_token: &str) -> AppResult<MessageView> {
        let endpoint = messages::message_endpoint(id);
        let query = messages::full_query();
        let resource: GmailMessageResource =
            self.get_json(&endpoint, access_token, Some(&query)).await?;
        Ok(resource.into_view())
    }

    /// Fetch a message with `format=full` and walk its MIME tree, returning
    /// metadata for every part that carries a downloadable `attachmentId`.
    pub async fn list_attachments(
        &self,
        id: &str,
        access_token: &str,
    ) -> AppResult<AttachmentList> {
        let endpoint = messages::message_endpoint(id);
        let query = messages::full_query();
        let resource: GmailMessageResource =
            self.get_json(&endpoint, access_token, Some(&query)).await?;

        let mut attachments = Vec::new();
        if let Some(payload) = &resource.payload {
            collect_attachments(payload, &mut attachments);
        }

        Ok(AttachmentList {
            message_id: resource.id,
            attachments,
        })
    }

    /// Download a single attachment's bytes via `messages.attachments.get`,
    /// decoding the base64url payload the Gmail API returns.
    pub async fn get_attachment(
        &self,
        message_id: &str,
        attachment_id: &str,
        access_token: &str,
    ) -> AppResult<Vec<u8>> {
        let endpoint = messages::attachment_endpoint(message_id, attachment_id);
        let resource: GmailAttachmentResource =
            self.get_json(&endpoint, access_token, None).await?;

        let data = resource.data.ok_or_else(|| {
            AppError::Api("gmail attachment response contained no data".to_string())
        })?;

        decode_base64url(&data)
    }

    /// List messages matching `query` (up to `limit`), fetching each one's metadata.
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

    /// Submit a base64url-encoded raw RFC 822 message, optionally into an existing thread.
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

    /// Fetch the account's send-as aliases, primary first then alphabetical by email.
    pub async fn list_send_as(&self, access_token: &str) -> AppResult<Vec<SendAsView>> {
        let endpoint = send_as::list_send_as_endpoint();
        let response: GmailSendAsListResponse = self.get_json(endpoint, access_token, None).await?;
        let mut aliases = response
            .send_as
            .unwrap_or_default()
            .into_iter()
            .map(GmailSendAsResource::into_view)
            .collect::<Vec<_>>();
        aliases.sort_by(|a, b| b.is_primary.cmp(&a.is_primary).then(a.email.cmp(&b.email)));
        Ok(aliases)
    }

    /// Fetch all labels on the account, sorted alphabetically by name.
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

    /// Add the given labels to a message.
    pub async fn add_labels(
        &self,
        id: &str,
        labels: &[String],
        access_token: &str,
    ) -> AppResult<LabelMutationResult> {
        self.modify_labels(id, labels, &[], access_token).await
    }

    /// Remove the given labels from a message.
    pub async fn rm_labels(
        &self,
        id: &str,
        labels: &[String],
        access_token: &str,
    ) -> AppResult<LabelMutationResult> {
        self.modify_labels(id, &[], labels, access_token).await
    }

    /// Resolve label names/ids, then issue a single `messages.modify` adding and removing them.
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

    /// Map requested label names or ids to canonical label ids, erroring on any unknown label.
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

    /// Issue a bearer-authenticated GET with optional query params and deserialize the JSON body.
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

    /// Issue a bearer-authenticated POST with a JSON body and deserialize the JSON response.
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

    /// Join an endpoint path onto the client's base URL.
    fn endpoint_url(&self, endpoint: &str) -> AppResult<Url> {
        let mut url = Url::parse(&self.base_url)?;
        url.set_path(endpoint.trim_start_matches('/'));
        Ok(url)
    }

    /// Deserialize a successful response, or convert an error status + body into an `AppError`.
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
    /// Flatten the raw resource into a `MessageView`, extracting common headers
    /// and (when the payload carries part data, i.e. `format=full`) the body text.
    fn into_view(self) -> MessageView {
        let GmailMessageResource {
            id,
            thread_id,
            snippet,
            payload,
        } = self;

        let headers = payload
            .as_ref()
            .and_then(|payload| payload.headers.as_deref())
            .unwrap_or_default();
        let body = payload.as_ref().and_then(extract_body);
        let mut attachments = Vec::new();
        if let Some(payload) = payload.as_ref() {
            collect_attachments(payload, &mut attachments);
        }

        MessageView {
            id,
            thread_id,
            snippet,
            subject: header_value(headers, "Subject"),
            from: header_value(headers, "From"),
            reply_to: header_value(headers, "Reply-To"),
            date: header_value(headers, "Date"),
            message_id: header_value(headers, "Message-ID"),
            in_reply_to: header_value(headers, "In-Reply-To"),
            references: header_value(headers, "References"),
            body,
            attachments,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GmailMessagePayload {
    headers: Option<Vec<GmailMessageHeader>>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    filename: Option<String>,
    body: Option<GmailPartBody>,
    parts: Option<Vec<GmailMessagePayload>>,
}

#[derive(Debug, Deserialize)]
struct GmailPartBody {
    #[serde(rename = "attachmentId")]
    attachment_id: Option<String>,
    size: Option<u64>,
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GmailAttachmentResource {
    data: Option<String>,
}

/// Extract a human-readable body from a MIME part tree, preferring `text/plain`
/// and falling back to a tag-stripped `text/html` part.
fn extract_body(payload: &GmailMessagePayload) -> Option<String> {
    part_text(payload, "text/plain")
        .or_else(|| part_text(payload, "text/html").map(|html| strip_html(&html)))
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// Depth-first search for the first part whose MIME type matches `want_mime`,
/// returning its inline base64url `data` decoded to a UTF-8 string.
fn part_text(part: &GmailMessagePayload, want_mime: &str) -> Option<String> {
    if part.mime_type.as_deref() == Some(want_mime)
        && let Some(data) = part.body.as_ref().and_then(|body| body.data.as_ref())
        && let Ok(bytes) = decode_base64url(data)
    {
        return Some(String::from_utf8_lossy(&bytes).into_owned());
    }

    if let Some(parts) = &part.parts {
        for nested in parts {
            if let Some(found) = part_text(nested, want_mime) {
                return Some(found);
            }
        }
    }

    None
}

/// Crudely reduce an HTML fragment to plain text: drop tags, decode entities,
/// and collapse trailing whitespace. Good enough for reading an email in a terminal.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    let decoded = html_escape::decode_html_entities(&out);
    decoded
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Recursively descend a MIME part tree, pushing metadata for each part that
/// has both an `attachmentId` and a non-empty filename (skipping inline bodies).
fn collect_attachments(part: &GmailMessagePayload, out: &mut Vec<AttachmentMeta>) {
    if let Some(body) = &part.body
        && let Some(attachment_id) = &body.attachment_id
    {
        let filename = part.filename.clone().unwrap_or_default();
        if !filename.is_empty() {
            out.push(AttachmentMeta {
                attachment_id: attachment_id.clone(),
                filename,
                mime_type: part
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                size: body.size,
            });
        }
    }

    if let Some(parts) = &part.parts {
        for nested in parts {
            collect_attachments(nested, out);
        }
    }
}

/// Decode a base64url string, tolerating both padded and unpadded input.
fn decode_base64url(data: &str) -> AppResult<Vec<u8>> {
    let trimmed = data.trim_end_matches('=');
    URL_SAFE_NO_PAD
        .decode(trimmed)
        .map_err(|err| AppError::Api(format!("failed to decode attachment data: {err}")))
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
struct GmailSendAsListResponse {
    #[serde(rename = "sendAs")]
    send_as: Option<Vec<GmailSendAsResource>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GmailSendAsResource {
    send_as_email: String,
    display_name: Option<String>,
    #[serde(default)]
    is_primary: bool,
    #[serde(default)]
    is_default: bool,
    verification_status: Option<String>,
}

impl GmailSendAsResource {
    /// Project the raw resource into a `SendAsView`, dropping empty display names.
    fn into_view(self) -> SendAsView {
        SendAsView {
            email: self.send_as_email,
            display_name: self
                .display_name
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty()),
            is_primary: self.is_primary,
            is_default: self.is_default,
            verification_status: self.verification_status,
        }
    }
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

/// Find a header by case-insensitive name, returning its trimmed value if non-empty.
fn header_value(headers: &[GmailMessageHeader], target: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(target))
        .map(|header| header.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Map an HTTP error status and body into an `AppError`, routing 401/403 to an auth error.
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

/// Parse Gmail's JSON error envelope into a compact `message, status, code, reason` string.
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
