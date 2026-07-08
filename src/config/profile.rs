use crate::error::{AppError, AppResult};

/// Environment variable that overrides the default profile (below an explicit `--profile`).
pub const PROFILE_ENV: &str = "GMAIL_PROFILE";

/// Profile name used as the ultimate fallback and for legacy single-profile setups.
pub const FALLBACK_PROFILE: &str = "default";

/// Resolve which profile to use, in precedence order:
/// explicit `--profile` flag > `GMAIL_PROFILE` env > configured default profile >
/// the sole profile if only one exists > the `default` profile if present.
///
/// Errors only when several profiles exist, none is named `default`, and no
/// default has been selected — an ambiguous state the caller must resolve.
pub fn resolve_profile(
    flag: Option<&str>,
    env: Option<&str>,
    config_default: Option<&str>,
    available: &[String],
) -> AppResult<String> {
    for candidate in [flag, env, config_default] {
        if let Some(name) = candidate.map(str::trim).filter(|name| !name.is_empty()) {
            return Ok(name.to_string());
        }
    }

    match available {
        [] => Ok(FALLBACK_PROFILE.to_string()),
        [only] => Ok(only.clone()),
        many if many.iter().any(|name| name == FALLBACK_PROFILE) => {
            Ok(FALLBACK_PROFILE.to_string())
        }
        many => Err(AppError::Config(format!(
            "multiple profiles found ({}) but no default is set. run `gmail profile use <name>` or pass --profile <name>",
            many.join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profiles(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn flag_wins_over_everything() {
        let got = resolve_profile(
            Some("flag"),
            Some("env"),
            Some("config"),
            &profiles(&["default", "flag"]),
        )
        .unwrap();
        assert_eq!(got, "flag");
    }

    #[test]
    fn env_beats_config_and_disk() {
        let got =
            resolve_profile(None, Some("env"), Some("config"), &profiles(&["a", "b"])).unwrap();
        assert_eq!(got, "env");
    }

    #[test]
    fn config_default_used_when_no_flag_or_env() {
        let got = resolve_profile(
            None,
            None,
            Some("personal"),
            &profiles(&["personal", "work"]),
        )
        .unwrap();
        assert_eq!(got, "personal");
    }

    #[test]
    fn blank_candidates_are_skipped() {
        let got = resolve_profile(Some("  "), Some(""), None, &profiles(&["solo"])).unwrap();
        assert_eq!(got, "solo");
    }

    #[test]
    fn no_profiles_falls_back_to_default() {
        assert_eq!(resolve_profile(None, None, None, &[]).unwrap(), "default");
    }

    #[test]
    fn single_profile_is_used_implicitly() {
        let got = resolve_profile(None, None, None, &profiles(&["personal"])).unwrap();
        assert_eq!(got, "personal");
    }

    #[test]
    fn many_profiles_prefer_default_when_present() {
        let got = resolve_profile(None, None, None, &profiles(&["default", "other"])).unwrap();
        assert_eq!(got, "default");
    }

    #[test]
    fn many_profiles_without_default_is_ambiguous() {
        let err = resolve_profile(None, None, None, &profiles(&["a", "b"])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }
}
