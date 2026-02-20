mod config {
    pub use gmail::config::*;
}

mod error {
    pub use gmail::error::*;
}

mod token {
    pub use gmail::auth::token::*;
}

mod token_store {
    pub use gmail::auth::token_store::*;
}

mod oauth_under_test {
    #![allow(dead_code)]

    include!("../src/auth/oauth.rs");

    #[test]
    fn parses_callback_code() {
        let code = extract_callback_code("/callback?code=abc123&state=xyz", "/callback", "xyz")
            .expect("callback should parse");
        assert_eq!(code, "abc123");
    }

    #[test]
    fn rejects_state_mismatch() {
        let result =
            extract_callback_code("/callback?code=abc123&state=wrong", "/callback", "expected");
        assert!(result.is_err());
    }

    #[test]
    fn builds_pkce_challenge() {
        let verifier = "test_verifier_value";
        let challenge = pkce_challenge(verifier);
        assert!(!challenge.is_empty());
    }

    #[test]
    fn random_token_is_non_empty() {
        let token = random_token(32);
        assert!(token.len() >= 43);
    }
}
