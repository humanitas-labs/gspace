use crate::cli::ListArgs;
use crate::context::AppContext;
use crate::error::{AppError, AppResult};
use crate::output::OutputMode;

pub async fn run(ctx: &AppContext, args: ListArgs) -> AppResult<()> {
    if args.limit == 0 {
        return Err(AppError::InvalidInput(
            "--limit must be greater than 0".to_string(),
        ));
    }

    let access_token = ctx.access_token().await?;
    let query = build_query(args.inbox, args.q.as_deref());
    let messages = ctx
        .gmail_client
        .list(&access_token, args.limit, query.as_deref())
        .await?;

    if ctx.output.mode() == OutputMode::Text {
        if messages.is_empty() {
            println!("0 messages");
            return Ok(());
        }

        for (index, message) in messages.iter().enumerate() {
            let from = message.from.as_deref().unwrap_or("(unknown sender)");
            let subject = message.subject.as_deref().unwrap_or("(no subject)");
            let date = message.date.as_deref().unwrap_or("(no date)");
            let preview = format_preview(message.snippet.as_deref());

            println!("{}. {}", index + 1, message.id);
            println!("   from: {from}");
            println!("   subject: {subject}");
            println!("   date: {date}");
            println!();
            println!("   {preview}");

            if index + 1 < messages.len() {
                println!();
            }
        }

        return Ok(());
    }

    let text = format!("{} messages", messages.len());
    ctx.output.emit(&text, &messages)
}

fn format_preview(snippet: Option<&str>) -> String {
    let snippet = snippet.unwrap_or("(no preview)");
    let decoded = html_escape::decode_html_entities(snippet).to_string();
    let compact = decoded.split_whitespace().collect::<Vec<_>>().join(" ");

    if compact.len() <= 120 {
        return compact;
    }

    let mut end = 120;
    while !compact.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &compact[..end])
}

fn build_query(inbox: bool, user_query: Option<&str>) -> Option<String> {
    let user_query = user_query.map(str::trim).filter(|query| !query.is_empty());

    match (inbox, user_query) {
        (true, Some(query)) => Some(format!("in:inbox {query}")),
        (true, None) => Some("in:inbox".to_string()),
        (false, Some(query)) => Some(query.to_string()),
        (false, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_inbox_query() {
        assert_eq!(build_query(true, None).as_deref(), Some("in:inbox"));
    }

    #[test]
    fn combines_inbox_and_user_query() {
        assert_eq!(
            build_query(true, Some("from:alice@example.com")).as_deref(),
            Some("in:inbox from:alice@example.com")
        );
    }

    #[test]
    fn formats_preview_with_truncation() {
        let input = Some(
            "this is a very long preview string that should be truncated at one hundred and twenty characters to keep list output compact and readable",
        );
        let preview = format_preview(input);
        assert!(preview.ends_with("..."));
        assert!(preview.len() <= 123);
    }

    #[test]
    fn decodes_common_html_entities_in_preview() {
        let preview = format_preview(Some("I&#39;ve &amp; you&#x27;ve &lt;done&gt; this"));
        assert_eq!(preview, "I've & you've <done> this");
    }
}
