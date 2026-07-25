use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{AppError, AppResult};

/// Bearer-authenticated JSON transport shared by the Google API clients.
///
/// Each service client (Gmail, Calendar) wraps one of these with its own base
/// URL, a `service` label used in error messages, and an `auth_hint` appended
/// to 401/403 failures telling the user how to recover.
#[derive(Debug, Clone)]
pub struct JsonClient {
    http: Client,
    base_url: String,
    service: &'static str,
    auth_hint: &'static str,
}

impl JsonClient {
    /// Construct a transport for one Google API service.
    pub fn new(base_url: &str, service: &'static str, auth_hint: &'static str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.to_string(),
            service,
            auth_hint,
        }
    }

    /// Issue a GET with optional query params and deserialize the JSON body.
    pub async fn get_json<T: DeserializeOwned>(
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

    /// Issue a POST with a JSON body and optional query params, deserializing the response.
    pub async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        endpoint: &str,
        access_token: &str,
        body: &B,
        query: Option<&[(String, String)]>,
    ) -> AppResult<T> {
        let url = self.endpoint_url(endpoint)?;
        let mut request = self.http.post(url).bearer_auth(access_token).json(body);
        if let Some(query) = query {
            request = request.query(query);
        }

        let response = request.send().await?;
        self.parse_json_response(response).await
    }

    /// Issue a DELETE with optional query params, expecting an empty success body.
    pub async fn delete(
        &self,
        endpoint: &str,
        access_token: &str,
        query: Option<&[(String, String)]>,
    ) -> AppResult<()> {
        let url = self.endpoint_url(endpoint)?;
        let mut request = self.http.delete(url).bearer_auth(access_token);
        if let Some(query) = query {
            request = request.query(query);
        }

        let response = request.send().await?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        let body = response.text().await.unwrap_or_default();
        Err(self.map_api_error(status, &body))
    }

    /// Join an endpoint path onto the client's base URL, preserving any base path.
    fn endpoint_url(&self, endpoint: &str) -> AppResult<Url> {
        let base = Url::parse(&self.base_url)?;
        let joined = format!(
            "{}/{}",
            base.path().trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        );
        let mut url = base;
        url.set_path(&joined);
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
        Err(self.map_api_error(status, &body))
    }

    /// Map an HTTP error status and body into an `AppError`, routing 401/403 to
    /// an auth error carrying this service's recovery hint.
    fn map_api_error(&self, status: StatusCode, body: &str) -> AppError {
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
                "{} api authorization failed ({status}): {message}. {}",
                self.service, self.auth_hint
            ));
        }

        AppError::Api(format!(
            "{} api request failed ({status}): {message}",
            self.service
        ))
    }
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorEnvelope {
    error: GoogleApiError,
}

#[derive(Debug, Deserialize)]
struct GoogleApiError {
    code: Option<u16>,
    status: Option<String>,
    message: Option<String>,
    errors: Option<Vec<GoogleApiErrorDetail>>,
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorDetail {
    reason: Option<String>,
}

/// Parse Google's JSON error envelope into a compact `message, status, code, reason` string.
fn parse_api_error_message(body: &str) -> Option<String> {
    let envelope = serde_json::from_str::<GoogleApiErrorEnvelope>(body).ok()?;
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
