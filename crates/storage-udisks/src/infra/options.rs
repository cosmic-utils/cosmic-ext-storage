use std::collections::HashSet;

/// Split a comma-delimited mount/crypt options string into trimmed tokens.
///
/// - Splits on `,`
/// - Trims whitespace
/// - Drops empty tokens
pub fn split_options(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Join tokens into a comma-delimited option string.
///
/// Uses stable order as given.
pub fn join_options(tokens: &[String]) -> String {
    tokens.join(",")
}

/// Stable de-duplication preserving first-seen order.
pub fn stable_dedup(tokens: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(tokens.len());

    for t in tokens {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    }

    out
}

/// Remove exact token matches.
pub fn remove_token(tokens: Vec<String>, token: &str) -> Vec<String> {
    tokens.into_iter().filter(|t| t != token).collect()
}

/// Ensure token exists or does not exist.
pub fn set_token_present(tokens: Vec<String>, token: &str, present: bool) -> Vec<String> {
    let mut tokens = remove_token(tokens, token);
    if present {
        tokens.push(token.to_owned());
    }
    tokens
}

/// Remove any token that starts with `prefix` (for key-value tokens like `x-gvfs-name=`).
pub fn remove_prefixed(tokens: Vec<String>, prefix: &str) -> Vec<String> {
    tokens
        .into_iter()
        .filter(|t| !t.starts_with(prefix))
        .collect()
}

/// Set a key-value token by prefix.
///
/// - If `value` is `Some(v)` and non-empty, ensures a token `prefix + v` exists.
/// - If `value` is `None` or empty, removes all tokens with that prefix.
pub fn set_prefixed_value(tokens: Vec<String>, prefix: &str, value: Option<&str>) -> Vec<String> {
    let mut tokens = remove_prefixed(tokens, prefix);
    if let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) {
        tokens.push(format!("{prefix}{v}"));
    }
    tokens
}

/// Merge user-provided "other options" (comma delimited) with managed tokens.
///
/// - `base_other` is user input.
/// - `managed` are tokens we want to force (e.g. `noauto`, `x-udisks-auth`).
///
/// Returns a stable, de-duplicated list.
pub fn merge_other_with_managed(base_other: &str, managed: Vec<String>) -> Vec<String> {
    let mut tokens = split_options(base_other);
    tokens.extend(managed);
    stable_dedup(tokens)
}

/// Convenience: stable-dedup and then join.
pub fn normalize_options(input: &str) -> String {
    join_options(&stable_dedup(split_options(input)))
}

/// Canonicalize options for deterministic testing/snapshots.
/// Not used for UI (we preserve stable order there).
#[cfg(test)]
fn sort_tokens(tokens: Vec<String>) -> Vec<String> {
    use std::collections::BTreeSet;
    let set: BTreeSet<String> = tokens.into_iter().collect();
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_trims_and_drops_empty() {
        assert_eq!(
            split_options("  noauto, ,x-udisks-auth ,"),
            vec!["noauto".to_string(), "x-udisks-auth".to_string()]
        );
    }

    #[test]
    fn stable_dedup_preserves_first_seen() {
        assert_eq!(
            stable_dedup(vec![
                "a".to_string(),
                "b".to_string(),
                "a".to_string(),
                "c".to_string(),
                "b".to_string()
            ]),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn set_token_present_adds_and_removes() {
        let tokens = vec!["a".to_string(), "noauto".to_string()];
        assert_eq!(
            sort_tokens(set_token_present(tokens.clone(), "noauto", false)),
            vec!["a".to_string()]
        );
        assert_eq!(
            sort_tokens(set_token_present(tokens, "x-udisks-auth", true)),
            vec![
                "a".to_string(),
                "noauto".to_string(),
                "x-udisks-auth".to_string()
            ]
        );
    }

    #[test]
    fn set_prefixed_value_sets_and_clears() {
        let tokens = vec![
            "x-gvfs-name=Old".to_string(),
            "noauto".to_string(),
            "x-gvfs-name=Other".to_string(),
        ];
        assert_eq!(
            sort_tokens(set_prefixed_value(
                tokens.clone(),
                "x-gvfs-name=",
                Some("New")
            )),
            vec!["noauto".to_string(), "x-gvfs-name=New".to_string()]
        );
        assert_eq!(
            sort_tokens(set_prefixed_value(tokens, "x-gvfs-name=", None)),
            vec!["noauto".to_string()]
        );
    }

    #[test]
    fn merge_other_with_managed_stable_dedup() {
        let merged = merge_other_with_managed(
            "nofail, noauto, noauto",
            vec!["noauto".to_string(), "x-udisks-auth".to_string()],
        );
        assert_eq!(
            merged,
            vec![
                "nofail".to_string(),
                "noauto".to_string(),
                "x-udisks-auth".to_string()
            ]
        );
    }
}
