mod error {
    pub use gmail::error::*;
}

mod labels {
    pub use gmail::api::labels::*;
}

mod messages {
    pub use gmail::api::messages::*;
}

mod models {
    pub use gmail::api::models::*;
}

mod client_under_test {
    #![allow(dead_code)]

    include!("../src/api/client.rs");

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
