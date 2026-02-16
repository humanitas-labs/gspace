pub fn message_endpoint(id: &str) -> String {
    format!("/gmail/v1/users/me/messages/{id}")
}

pub fn list_endpoint() -> &'static str {
    "/gmail/v1/users/me/messages"
}

pub fn send_endpoint() -> &'static str {
    "/gmail/v1/users/me/messages/send"
}

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

pub fn list_query(limit: u32, query: Option<&str>) -> Vec<(String, String)> {
    let mut params = vec![("maxResults".to_string(), limit.to_string())];
    if let Some(query) = query {
        params.push(("q".to_string(), query.to_string()));
    }
    params
}
