use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::Rng;

use crate::api::models::SendRequest;

pub fn build_raw_message(request: &SendRequest) -> String {
    let mut headers = build_base_headers(request);

    let payload = if request.attachments.is_empty() {
        headers.push("Content-Type: text/plain; charset=utf-8".to_string());
        format!("{}\r\n\r\n{}", headers.join("\r\n"), request.body)
    } else {
        let boundary = random_boundary();
        headers.push(format!(
            "Content-Type: multipart/mixed; boundary=\"{boundary}\""
        ));
        format!(
            "{}\r\n\r\n{}",
            headers.join("\r\n"),
            multipart_body(request, &boundary)
        )
    };

    URL_SAFE_NO_PAD.encode(payload.as_bytes())
}

fn build_base_headers(request: &SendRequest) -> Vec<String> {
    let mut headers = Vec::new();
    headers.push(format!("To: {}", request.to.join(", ")));

    if !request.cc.is_empty() {
        headers.push(format!("Cc: {}", request.cc.join(", ")));
    }

    if !request.bcc.is_empty() {
        headers.push(format!("Bcc: {}", request.bcc.join(", ")));
    }

    headers.push(format!("Subject: {}", request.subject));
    if let Some(in_reply_to) = &request.in_reply_to {
        headers.push(format!("In-Reply-To: {in_reply_to}"));
    }
    if let Some(references) = &request.references {
        headers.push(format!("References: {references}"));
    }

    headers
}

fn multipart_body(request: &SendRequest, boundary: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("--{boundary}\r\n"));
    out.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
    out.push_str(&request.body);
    out.push_str("\r\n");

    for attachment in &request.attachments {
        out.push_str(&format!("--{boundary}\r\n"));
        out.push_str(&format!(
            "Content-Type: {}; name=\"{}\"\r\n",
            attachment.mime_type,
            escape_header_value(&attachment.filename)
        ));
        out.push_str("Content-Transfer-Encoding: base64\r\n");
        out.push_str(&format!(
            "Content-Disposition: attachment; filename=\"{}\"\r\n\r\n",
            escape_header_value(&attachment.filename)
        ));

        let encoded = STANDARD.encode(&attachment.data);
        out.push_str(&fold_base64_lines(&encoded));
        out.push_str("\r\n");
    }

    out.push_str(&format!("--{boundary}--\r\n"));
    out
}

fn fold_base64_lines(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + input.len() / 76 + 8);
    let mut start = 0;
    while start < input.len() {
        let end = (start + 76).min(input.len());
        out.push_str(&input[start..end]);
        out.push_str("\r\n");
        start = end;
    }
    out
}

fn random_boundary() -> String {
    let mut bytes = [0_u8; 12];
    rand::thread_rng().fill(&mut bytes);
    let token = STANDARD.encode(bytes);
    format!("gmail-cli-{token}")
}

fn escape_header_value(value: &str) -> String {
    value.replace('"', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{Attachment, SendRequest};

    #[test]
    fn includes_reply_headers() {
        let request = SendRequest {
            to: vec!["dev@example.com".to_string()],
            cc: vec![],
            bcc: vec![],
            subject: "Re: Test".to_string(),
            body: "Hello".to_string(),
            in_reply_to: Some("<id@example.com>".to_string()),
            references: Some("<ref@example.com> <id@example.com>".to_string()),
            thread_id: None,
            attachments: vec![],
        };

        let raw = build_raw_message(&request);
        let decoded = String::from_utf8(URL_SAFE_NO_PAD.decode(raw).expect("base64 decode"))
            .expect("utf8 payload");

        assert!(decoded.contains("In-Reply-To: <id@example.com>"));
        assert!(decoded.contains("References: <ref@example.com> <id@example.com>"));
    }

    #[test]
    fn builds_multipart_when_attachments_exist() {
        let request = SendRequest {
            to: vec!["dev@example.com".to_string()],
            cc: vec![],
            bcc: vec![],
            subject: "Test".to_string(),
            body: "Hello".to_string(),
            in_reply_to: None,
            references: None,
            thread_id: None,
            attachments: vec![Attachment {
                filename: "a.txt".to_string(),
                mime_type: "text/plain".to_string(),
                data: b"hello attachment".to_vec(),
            }],
        };

        let raw = build_raw_message(&request);
        let decoded = String::from_utf8(URL_SAFE_NO_PAD.decode(raw).expect("base64 decode"))
            .expect("utf8 payload");

        assert!(decoded.contains("multipart/mixed"));
        assert!(decoded.contains("Content-Disposition: attachment; filename=\"a.txt\""));
    }
}
