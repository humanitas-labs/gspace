use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use pulldown_cmark::{Options, Parser, html};
use rand::Rng;

use crate::api::models::SendRequest;

const EMAIL_HTML_TEMPLATE: &str = r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    body {
      margin: 0;
      padding: 0;
      background: #ffffff;
      color: #202124;
      font-family: Arial, Helvetica, sans-serif;
      font-size: 14px;
      line-height: 1.6;
    }
    .email-body {
      margin: 0;
      padding: 0;
    }
    h1, h2, h3, h4, h5, h6 {
      line-height: 1.3;
      margin-top: 1.2em;
      margin-bottom: 0.5em;
    }
    p, ul, ol, pre, blockquote, table {
      margin-top: 0;
      margin-bottom: 1em;
    }
    a {
      color: #0b57d0;
    }
    img {
      max-width: 100%;
      height: auto;
    }
    pre {
      background: #f1f3f5;
      border-radius: 8px;
      overflow-x: auto;
      padding: 12px;
      white-space: pre-wrap;
      word-break: break-word;
    }
    code {
      font-family: Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    }
    blockquote {
      margin-left: 0;
      padding-left: 12px;
      border-left: 3px solid #d0d7de;
      color: #5f6368;
    }
    table {
      border-collapse: collapse;
      width: 100%;
      display: block;
      overflow-x: auto;
    }
    th, td {
      border: 1px solid #d0d7de;
      padding: 6px 8px;
      text-align: left;
    }
  </style>
</head>
<body>
  <div class="email-body">
__BODY__
  </div>
</body>
</html>
"#;

pub fn markdown_to_html(body_markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(body_markdown, options);
    let mut body_html = String::new();
    html::push_html(&mut body_html, parser);

    if body_html.trim().is_empty() {
        body_html.push_str("<p></p>");
    }

    EMAIL_HTML_TEMPLATE.replacen("__BODY__", &body_html, 1)
}

pub fn build_raw_message(request: &SendRequest) -> String {
    let mut headers = build_base_headers(request);

    let payload = if request.attachments.is_empty() {
        headers.push("Content-Type: text/html; charset=utf-8".to_string());
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

    if let Some(from) = &request.from {
        headers.push(format!("From: {from}"));
    }

    headers.push(format!("To: {}", request.to.join(", ")));

    if !request.cc.is_empty() {
        headers.push(format!("Cc: {}", request.cc.join(", ")));
    }

    if !request.bcc.is_empty() {
        headers.push(format!("Bcc: {}", request.bcc.join(", ")));
    }

    headers.push(format!("Subject: {}", request.subject));
    headers.push("MIME-Version: 1.0".to_string());
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
    out.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
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
