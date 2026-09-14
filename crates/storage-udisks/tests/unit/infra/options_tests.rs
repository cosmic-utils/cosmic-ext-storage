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
