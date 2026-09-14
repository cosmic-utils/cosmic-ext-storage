use super::*;
#[test]
fn a_removed_group_is_skipped_only_after_fresh_discovery_confirms_absence() {
    assert!(
        confirm_vanished_group(
            zbus::fdo::Error::UnknownObject("removed".into()).into(),
            false
        )
        .is_ok()
    );
    let error = confirm_vanished_group(
        zbus::fdo::Error::AccessDenied("private".into()).into(),
        true,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        storage_contracts::StorageErrorKind::PermissionDenied
    );
    assert!(
        confirm_vanished_group(
            zbus::fdo::Error::UnknownObject("still in snapshot".into()).into(),
            true
        )
        .is_err()
    );
}
