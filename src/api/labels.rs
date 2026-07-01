/// Endpoint path for listing the account's labels.
pub fn list_labels_endpoint() -> &'static str {
    "/gmail/v1/users/me/labels"
}

/// Endpoint path for modifying label ids on a message.
pub fn modify_labels_endpoint(id: &str) -> String {
    format!("/gmail/v1/users/me/messages/{id}/modify")
}
