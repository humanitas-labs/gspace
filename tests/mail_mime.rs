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

fn send_request_with_subject(subject: &str) -> SendRequest {
    SendRequest {
        from: None,
        to: vec!["dev@example.com".to_string()],
        cc: vec![],
        bcc: vec![],
        subject: subject.to_string(),
        body: markdown_to_html("Hello"),
        in_reply_to: None,
        references: None,
        thread_id: None,
        attachments: vec![],
    }
}

fn decoded_payload(request: &SendRequest) -> String {
    let raw = build_raw_message(request);
    String::from_utf8(URL_SAFE_NO_PAD.decode(raw).expect("base64 decode")).expect("utf8 payload")
}

/// Decode every `=?UTF-8?B?...?=` word on the Subject header line(s) back to text.
fn decode_subject_words(payload: &str) -> String {
    use base64::engine::general_purpose::STANDARD;

    let start = payload.find("Subject: ").expect("subject header") + "Subject: ".len();
    let rest = &payload[start..];
    // The header may fold across lines (CRLF + space); it ends at the first
    // CRLF not followed by a space.
    let mut header = String::new();
    for line in rest.split("\r\n") {
        if header.is_empty() {
            header.push_str(line);
        } else if let Some(cont) = line.strip_prefix(' ') {
            // Unfold per RFC 5322: CRLF + WSP collapses to the WSP.
            header.push(' ');
            header.push_str(cont);
        } else {
            break;
        }
    }

    header
        .split_whitespace()
        .map(|word| {
            let b64 = word
                .strip_prefix("=?UTF-8?B?")
                .and_then(|w| w.strip_suffix("?="))
                .expect("encoded word");
            String::from_utf8(STANDARD.decode(b64).expect("b64")).expect("utf8")
        })
        .collect()
}

#[test]
fn ascii_subject_passes_through_unencoded() {
    let payload = decoded_payload(&send_request_with_subject("Plain ascii subject"));

    assert!(payload.contains("Subject: Plain ascii subject\r\n"));
}

#[test]
fn non_ascii_subject_is_rfc2047_encoded() {
    let subject = "Recruiting desk — economics research";
    let payload = decoded_payload(&send_request_with_subject(subject));

    assert!(payload.contains("Subject: =?UTF-8?B?"));
    // No raw non-ASCII bytes may remain in the headers.
    let headers_end = payload.find("\r\n\r\n").expect("header/body split");
    assert!(payload[..headers_end].is_ascii());
    assert_eq!(decode_subject_words(&payload), subject);
}

#[test]
fn long_non_ascii_subject_folds_into_multiple_encoded_words() {
    let subject = "señal — ".repeat(12); // > 45 UTF-8 bytes, multibyte chars throughout
    let payload = decoded_payload(&send_request_with_subject(&subject));

    let headers_end = payload.find("\r\n\r\n").expect("header/body split");
    let header_block = &payload[..headers_end];
    assert!(header_block.is_ascii());
    // Folded continuation: at least two encoded words.
    assert!(header_block.matches("=?UTF-8?B?").count() >= 2);
    // Each header line stays within RFC 5322's 78-char limit.
    for line in header_block.split("\r\n") {
        assert!(line.len() <= 78, "header line too long: {line}");
    }
    assert_eq!(decode_subject_words(&payload), subject);
}
