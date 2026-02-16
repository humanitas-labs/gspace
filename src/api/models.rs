use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MessageView {
    pub id: String,
    pub thread_id: Option<String>,
    pub snippet: Option<String>,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub date: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SendRequest {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub thread_id: Option<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendResult {
    pub id: String,
    pub thread_id: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabelView {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabelMutationResult {
    pub id: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub note: String,
}
