use super::resolve_provider_icon;

#[test]
fn known_provider_has_primary_with_fallback() {
    let icon = resolve_provider_icon("dropbox");
    assert!(icon.branded.is_some());
    assert_eq!(icon.fallback_symbolic, "folder-remote-symbolic");
}

#[test]
fn unknown_provider_uses_generic_fallback() {
    let icon = resolve_provider_icon("unknown-provider");
    assert!(icon.branded.is_none());
    assert_eq!(icon.fallback_symbolic, "folder-remote-symbolic");
}
