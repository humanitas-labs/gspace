use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use gmail::api::models::{Attachment, SendRequest};
use gmail::mail::mime::{build_raw_message, markdown_to_html};

#[test]
fn renders_markdown_body_inside_html_template() {
    let html = markdown_to_html("## Hello\n\nVisit **gmail**.");

    assert!(html.contains("<!doctype html>"));
    assert!(
        html.contains("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">")
    );
    assert!(html.contains("<div class=\"email-body\">"));
    assert!(html.contains("<h2>Hello</h2>"));
    assert!(html.contains("<strong>gmail</strong>"));
}

#[test]
fn includes_reply_headers() {
    let request = SendRequest {
        from: Some("Jane Doe <jane@example.org>".to_string()),
        to: vec!["dev@example.com".to_string()],
        cc: vec![],
        bcc: vec![],
        subject: "Re: Test".to_string(),
        body: markdown_to_html("Hello"),
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
    assert!(decoded.contains("From: Jane Doe <jane@example.org>"));
    assert!(decoded.contains("MIME-Version: 1.0"));
    assert!(decoded.contains("Content-Type: text/html; charset=utf-8"));
}

#[test]
fn builds_multipart_when_attachments_exist() {
    let request = SendRequest {
        from: Some("Jane Doe <jane@example.org>".to_string()),
        to: vec!["dev@example.com".to_string()],
        cc: vec![],
        bcc: vec![],
        subject: "Test".to_string(),
        body: markdown_to_html("Hello"),
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
    assert!(decoded.contains("Content-Type: text/html; charset=utf-8"));
    assert!(decoded.contains("Content-Disposition: attachment; filename=\"a.txt\""));
}
