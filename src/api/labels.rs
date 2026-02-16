pub fn list_labels_endpoint() -> &'static str {
    "/gmail/v1/users/me/labels"
}

pub fn modify_labels_endpoint(id: &str) -> String {
    format!("/gmail/v1/users/me/messages/{id}/modify")
}
