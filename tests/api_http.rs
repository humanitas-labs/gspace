mod error {
    pub use gspace::error::*;
}

mod http_under_test {
    #![allow(dead_code)]

    include!("../src/api/http.rs");

    fn gmail_client() -> JsonClient {
        JsonClient::new(
            "https://gmail.googleapis.com",
            "gmail",
            "run `gmail auth login`",
        )
    }

    #[test]
    fn maps_unauthorized_as_auth_error() {
        let error = gmail_client().map_api_error(
            StatusCode::UNAUTHORIZED,
            r#"{"error":{"code":401,"message":"Request had invalid authentication credentials.","status":"UNAUTHENTICATED"}}"#,
        );

        match error {
            AppError::Auth(message) => {
                assert!(message.contains("invalid authentication credentials"));
                assert!(message.contains("run `gmail auth login`"));
            }
            other => panic!("expected auth error, got {other:?}"),
        }
    }

    #[test]
    fn maps_not_found_as_api_error() {
        let error = gmail_client().map_api_error(
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

    #[test]
    fn auth_error_carries_service_label_and_hint() {
        let client = JsonClient::new(
            "https://www.googleapis.com/calendar/v3",
            "calendar",
            "re-run `gmail auth login` to grant the calendar scope",
        );
        let error = client.map_api_error(
            StatusCode::FORBIDDEN,
            r#"{"error":{"code":403,"message":"Request had insufficient authentication scopes.","status":"PERMISSION_DENIED","errors":[{"reason":"insufficientPermissions"}]}}"#,
        );

        match error {
            AppError::Auth(message) => {
                assert!(message.starts_with("calendar api authorization failed"));
                assert!(message.contains("insufficient authentication scopes"));
                assert!(message.contains("grant the calendar scope"));
            }
            other => panic!("expected auth error, got {other:?}"),
        }
    }

    #[test]
    fn endpoint_url_preserves_base_path() {
        let client = JsonClient::new(
            "https://www.googleapis.com/calendar/v3",
            "calendar",
            "hint",
        );
        let url = client
            .endpoint_url("/calendars/primary/events")
            .expect("url should join");

        assert_eq!(
            url.as_str(),
            "https://www.googleapis.com/calendar/v3/calendars/primary/events"
        );
    }

    #[test]
    fn endpoint_url_without_base_path() {
        let url = gmail_client()
            .endpoint_url("/gmail/v1/users/me/messages")
            .expect("url should join");

        assert_eq!(
            url.as_str(),
            "https://gmail.googleapis.com/gmail/v1/users/me/messages"
        );
    }
}
