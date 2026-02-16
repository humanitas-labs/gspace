use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time;
use url::Url;

use crate::config::Settings;
use crate::error::{AppError, AppResult};

use super::token::TokenSet;
use super::token_store::TokenStore;

const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
const GOOGLE_USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const OAUTH_CALLBACK_TIMEOUT_SECS: u64 = 180;
const OAUTH_SCOPES: &str = "https://www.googleapis.com/auth/gmail.modify https://www.googleapis.com/auth/gmail.send openid email";

#[derive(Debug, Serialize)]
pub struct AuthLoginResult {
    pub profile: String,
    pub started: bool,
    pub opened_browser: bool,
    pub authorization_url: String,
    pub email: Option<String>,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    pub profile: String,
    pub logged_in: bool,
    pub email: Option<String>,
    pub expired: Option<bool>,
    pub expires_in_seconds: Option<i64>,
    pub has_refresh_token: Option<bool>,
    pub note: Option<String>,
}

#[derive(Debug, Default)]
pub struct AuthService;

impl AuthService {
    pub async fn login<S: TokenStore>(
        profile: &str,
        settings: &Settings,
        store: &S,
    ) -> AppResult<AuthLoginResult> {
        let oauth = OAuthConfig::from_settings(settings)?;
        let flow = LoginFlow::new(&oauth)?;
        let opened_browser = open_browser(&flow.authorization_url);

        if !opened_browser {
            eprintln!(
                "open this URL in your browser to continue login:\n{}",
                flow.authorization_url
            );
        }

        let code = wait_for_auth_callback(
            &oauth.redirect_uri,
            &flow.state,
            Duration::from_secs(OAUTH_CALLBACK_TIMEOUT_SECS),
        )
        .await?;

        let mut token = exchange_auth_code(&oauth, &code, &flow.code_verifier).await?;
        if let Ok(email) = fetch_email(&token.access_token).await {
            token.email = email;
        }
        store.save(profile, &token)?;

        Ok(AuthLoginResult {
            profile: profile.to_string(),
            started: true,
            opened_browser,
            authorization_url: flow.authorization_url,
            email: token.email,
            note: "oauth login completed and token stored".to_string(),
        })
    }

    pub async fn refresh<S: TokenStore>(
        profile: &str,
        settings: &Settings,
        store: &S,
    ) -> AppResult<TokenSet> {
        let oauth = OAuthConfig::from_settings(settings)?;

        let current = store.load(profile)?.ok_or_else(|| {
            AppError::InvalidInput("not logged in. run `gmail auth login`".to_string())
        })?;

        if !current.is_expired(SystemTime::now()) {
            return Ok(current);
        }

        let refresh_token = current.refresh_token.clone().ok_or_else(|| {
            AppError::Auth("access token expired and no refresh token is stored".to_string())
        })?;

        let mut refreshed = exchange_refresh_token(&oauth, &refresh_token).await?;
        if refreshed.refresh_token.is_none() {
            refreshed.refresh_token = Some(refresh_token);
        }

        if refreshed.email.is_none() {
            refreshed.email = current.email;
        }

        store.save(profile, &refreshed)?;
        Ok(refreshed)
    }

    pub async fn status<S: TokenStore>(profile: &str, store: &S) -> AppResult<AuthStatus> {
        let Some(token) = store.load(profile)? else {
            return Ok(AuthStatus {
                profile: profile.to_string(),
                logged_in: false,
                email: None,
                expired: None,
                expires_in_seconds: None,
                has_refresh_token: None,
                note: Some("no token found".to_string()),
            });
        };

        let now = SystemTime::now();
        let expired = token.is_expired(now);
        let expires_in_seconds = token.expires_in_seconds(now);

        Ok(AuthStatus {
            profile: profile.to_string(),
            logged_in: true,
            email: token.email.clone(),
            expired: Some(expired),
            expires_in_seconds,
            has_refresh_token: Some(token.has_refresh_token()),
            note: Some("token loaded from local store".to_string()),
        })
    }

    pub async fn logout<S: TokenStore>(profile: &str, store: &S) -> AppResult<AuthStatus> {
        let token = store.load(profile)?;
        let note = if let Some(token) = token {
            let token_to_revoke = token
                .refresh_token
                .as_deref()
                .unwrap_or(token.access_token.as_str());

            match revoke_token(token_to_revoke).await {
                Ok(()) => "remote token revoked and local credentials removed".to_string(),
                Err(err) => format!("local credentials removed (revoke failed: {err})"),
            }
        } else {
            "local credentials removed".to_string()
        };

        store.clear(profile)?;

        Ok(AuthStatus {
            profile: profile.to_string(),
            logged_in: false,
            email: None,
            expired: None,
            expires_in_seconds: None,
            has_refresh_token: None,
            note: Some(note),
        })
    }
}

#[derive(Debug)]
struct OAuthConfig {
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
}

impl OAuthConfig {
    fn from_settings(settings: &Settings) -> AppResult<Self> {
        Ok(Self {
            client_id: settings.client_id()?.to_string(),
            client_secret: settings.client_secret().map(ToOwned::to_owned),
            redirect_uri: settings.redirect_uri(),
        })
    }
}

#[derive(Debug)]
struct LoginFlow {
    authorization_url: String,
    code_verifier: String,
    state: String,
}

impl LoginFlow {
    fn new(config: &OAuthConfig) -> AppResult<Self> {
        let state = random_token(32);
        let code_verifier = random_token(96);
        let code_challenge = pkce_challenge(&code_verifier);

        let mut url = Url::parse(GOOGLE_AUTH_ENDPOINT)?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_uri)
            .append_pair("scope", OAUTH_SCOPES)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("state", &state)
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256");

        Ok(Self {
            authorization_url: url.to_string(),
            code_verifier,
            state,
        })
    }
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    token_type: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    email: Option<String>,
}

async fn exchange_auth_code(
    config: &OAuthConfig,
    code: &str,
    code_verifier: &str,
) -> AppResult<TokenSet> {
    let mut form = HashMap::from([
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("client_id", config.client_id.clone()),
        ("redirect_uri", config.redirect_uri.clone()),
        ("code_verifier", code_verifier.to_string()),
    ]);

    if let Some(client_secret) = &config.client_secret {
        form.insert("client_secret", client_secret.clone());
    }

    let response = reqwest::Client::new()
        .post(GOOGLE_TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .await?;

    parse_token_response(response).await
}

async fn exchange_refresh_token(config: &OAuthConfig, refresh_token: &str) -> AppResult<TokenSet> {
    let mut form = HashMap::from([
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", config.client_id.clone()),
    ]);

    if let Some(client_secret) = &config.client_secret {
        form.insert("client_secret", client_secret.clone());
    }

    let response = reqwest::Client::new()
        .post(GOOGLE_TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .await?;

    let mut token = parse_token_response(response).await?;
    if token.refresh_token.is_none() {
        token.refresh_token = Some(refresh_token.to_string());
    }
    if token.email.is_none() {
        if let Ok(email) = fetch_email(&token.access_token).await {
            token.email = email;
        }
    }

    Ok(token)
}

async fn parse_token_response(response: reqwest::Response) -> AppResult<TokenSet> {
    if response.status().is_success() {
        let payload: OAuthTokenResponse = response.json().await?;
        return Ok(TokenSet {
            access_token: payload.access_token,
            refresh_token: payload.refresh_token,
            expires_at_unix: expires_at_unix(payload.expires_in),
            token_type: payload.token_type,
            scope: payload.scope,
            email: None,
        });
    }

    let status = response.status();
    let body = response.text().await?;
    if let Ok(err_payload) = serde_json::from_str::<OAuthErrorResponse>(&body) {
        let error = err_payload
            .error
            .unwrap_or_else(|| "unknown_oauth_error".to_string());
        let description = err_payload
            .error_description
            .unwrap_or_else(|| "no description".to_string());
        return Err(AppError::Auth(format!(
            "oauth token exchange failed ({status}): {error} ({description})"
        )));
    }

    Err(AppError::Auth(format!(
        "oauth token exchange failed ({status}): {body}"
    )))
}

fn expires_at_unix(expires_in: Option<u64>) -> Option<u64> {
    let expires_in = expires_in?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(now.saturating_add(expires_in))
}

async fn fetch_email(access_token: &str) -> AppResult<Option<String>> {
    let response = reqwest::Client::new()
        .get(GOOGLE_USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let payload: UserInfoResponse = response.json().await?;
    Ok(payload.email)
}

async fn revoke_token(token: &str) -> AppResult<()> {
    let response = reqwest::Client::new()
        .post(GOOGLE_REVOKE_ENDPOINT)
        .form(&HashMap::from([("token", token.to_string())]))
        .send()
        .await?;

    if response.status().is_success() {
        return Ok(());
    }

    Err(AppError::Auth(format!(
        "revoke endpoint returned {}",
        response.status()
    )))
}

async fn wait_for_auth_callback(
    redirect_uri: &str,
    expected_state: &str,
    timeout: Duration,
) -> AppResult<String> {
    let redirect = Url::parse(redirect_uri)?;
    if redirect.scheme() != "http" {
        return Err(AppError::Config(
            "redirect_uri must use http for local callback capture".to_string(),
        ));
    }

    let host = redirect
        .host_str()
        .ok_or_else(|| AppError::Config("redirect_uri is missing host".to_string()))?;
    let port = redirect
        .port_or_known_default()
        .ok_or_else(|| AppError::Config("redirect_uri is missing port".to_string()))?;
    let path = redirect.path().to_string();

    let listener = TcpListener::bind((host, port)).await.map_err(|err| {
        AppError::Auth(format!(
            "failed to bind oauth callback listener on {host}:{port}: {err}"
        ))
    })?;

    let callback = time::timeout(timeout, async {
        let (mut stream, _) = listener.accept().await?;

        let mut buf = vec![0_u8; 8192];
        let size = stream.read(&mut buf).await?;
        if size == 0 {
            return Err(AppError::Auth("empty oauth callback request".to_string()));
        }

        let request = String::from_utf8_lossy(&buf[..size]);
        let request_line = request
            .lines()
            .next()
            .ok_or_else(|| AppError::Auth("malformed oauth callback request".to_string()))?;

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default();
        let target = parts.next().unwrap_or_default();

        if method != "GET" {
            write_callback_response(
                &mut stream,
                "405 Method Not Allowed",
                "oauth callback only accepts GET requests",
            )
            .await?;
            return Err(AppError::Auth(
                "oauth callback received non-GET request".to_string(),
            ));
        }

        let code = match extract_callback_code(target, &path, expected_state) {
            Ok(code) => {
                write_callback_response(
                    &mut stream,
                    "200 OK",
                    "gmail auth complete. you can return to the terminal.",
                )
                .await?;
                code
            }
            Err(err) => {
                let _ = write_callback_response(
                    &mut stream,
                    "400 Bad Request",
                    &format!("oauth callback error: {err}"),
                )
                .await;
                return Err(err);
            }
        };

        Ok(code)
    })
    .await
    .map_err(|_| AppError::Auth("timed out waiting for oauth callback".to_string()))??;

    Ok(callback)
}

fn extract_callback_code(
    target: &str,
    expected_path: &str,
    expected_state: &str,
) -> AppResult<String> {
    let callback_url = Url::parse(&format!("http://localhost{target}"))?;
    if callback_url.path() != expected_path {
        return Err(AppError::Auth(format!(
            "oauth callback path mismatch: expected {expected_path}, got {}",
            callback_url.path()
        )));
    }

    let mut code = None;
    let mut state = None;
    let mut oauth_error = None;
    let mut oauth_error_description = None;

    for (key, value) in callback_url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => oauth_error = Some(value.to_string()),
            "error_description" => oauth_error_description = Some(value.to_string()),
            _ => {}
        }
    }

    if let Some(error) = oauth_error {
        let description = oauth_error_description.unwrap_or_else(|| "no description".to_string());
        return Err(AppError::Auth(format!(
            "oauth authorization failed: {error} ({description})"
        )));
    }

    let received_state = state
        .ok_or_else(|| AppError::Auth("oauth callback missing state parameter".to_string()))?;
    if received_state != expected_state {
        return Err(AppError::Auth(
            "oauth state mismatch; aborting login".to_string(),
        ));
    }

    code.ok_or_else(|| AppError::Auth("oauth callback missing code parameter".to_string()))
}

async fn write_callback_response(
    stream: &mut tokio::net::TcpStream,
    status: &str,
    message: &str,
) -> AppResult<()> {
    let body = format!(
        "<!doctype html><html><body><p>{}</p></body></html>",
        escape_html(message)
    );

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

fn random_token(len: usize) -> String {
    let mut bytes = vec![0_u8; len];
    rand::thread_rng().fill(bytes.as_mut_slice());
    URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("open")
            .arg(url)
            .status()
            .is_ok_and(|status| status.success());
    }

    #[cfg(target_os = "linux")]
    {
        return std::process::Command::new("xdg-open")
            .arg(url)
            .status()
            .is_ok_and(|status| status.success());
    }

    #[cfg(target_os = "windows")]
    {
        return std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
            .is_ok_and(|status| status.success());
    }

    #[allow(unreachable_code)]
    false
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_callback_code() {
        let code = extract_callback_code("/callback?code=abc123&state=xyz", "/callback", "xyz")
            .expect("callback should parse");
        assert_eq!(code, "abc123");
    }

    #[test]
    fn rejects_state_mismatch() {
        let result =
            extract_callback_code("/callback?code=abc123&state=wrong", "/callback", "expected");
        assert!(result.is_err());
    }

    #[test]
    fn builds_pkce_challenge() {
        let verifier = "test_verifier_value";
        let challenge = pkce_challenge(verifier);
        assert!(!challenge.is_empty());
    }

    #[test]
    fn random_token_is_non_empty() {
        let token = random_token(32);
        assert!(token.len() >= 43);
    }
}
