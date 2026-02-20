mod cli {
    pub use gmail::cli::*;
}

mod context {
    pub use gmail::context::*;
}

mod error {
    pub use gmail::error::*;
}

mod output {
    pub use gmail::output::*;
}

mod list_under_test {
    #![allow(dead_code)]

    include!("../src/commands/list.rs");

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
