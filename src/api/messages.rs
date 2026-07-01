/// Endpoint path for a single message by id.
pub fn message_endpoint(id: &str) -> String {
    format!("/gmail/v1/users/me/messages/{id}")
}

/// Endpoint path for fetching a specific attachment's bytes.
pub fn attachment_endpoint(message_id: &str, attachment_id: &str) -> String {
    format!("/gmail/v1/users/me/messages/{message_id}/attachments/{attachment_id}")
}

/// Endpoint path for listing messages.
pub fn list_endpoint() -> &'static str {
    "/gmail/v1/users/me/messages"
}

/// Endpoint path for sending a message.
pub fn send_endpoint() -> &'static str {
    "/gmail/v1/users/me/messages/send"
}

/// Query params requesting `format=metadata` with the common envelope headers.
pub fn get_query() -> Vec<(String, String)> {
    let mut query = vec![("format".to_string(), "metadata".to_string())];

    for header in [
        "Subject",
        "From",
        "Reply-To",
        "Date",
        "Message-ID",
        "In-Reply-To",
        "References",
    ] {
        query.push(("metadataHeaders".to_string(), header.to_string()));
    }

    query
}

/// Query params requesting `format=full` (the complete MIME payload).
pub fn full_query() -> Vec<(String, String)> {
    vec![("format".to_string(), "full".to_string())]
}

/// Query params for a list request: `maxResults` and an optional Gmail search `q`.
pub fn list_query(limit: u32, query: Option<&str>) -> Vec<(String, String)> {
    let mut params = vec![("maxResults".to_string(), limit.to_string())];
    if let Some(query) = query {
        params.push(("q".to_string(), query.to_string()));
    }
    params
}
