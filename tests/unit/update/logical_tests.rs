use super::executes_from_single_step_form;
use storage_contracts::LogicalAction;
use storage_types::LogicalEntityId;

#[test]
fn non_destructive_btrfs_creations_skip_the_second_confirmation_dialog() {
    let filesystem = LogicalEntityId::new("btrfs:fixture").expect("fixture ID is valid");
    assert!(executes_from_single_step_form(
        &LogicalAction::CreateBtrfsSubvolume {
            filesystem: filesystem.clone(),
            name: "home".into(),
        }
    ));
    assert!(executes_from_single_step_form(
        &LogicalAction::CreateBtrfsSnapshot {
            filesystem: filesystem.clone(),
            source: storage_types::BtrfsSubvolumeRef {
                filesystem: filesystem.clone(),
                id: std::num::NonZeroU64::new(256).expect("non-zero fixture ID"),
                expected_relative_path: storage_types::BtrfsRelativePath::new("home")
                    .expect("valid fixture path"),
                expected_parent_id: None,
                observed_topology_epoch: 1,
            },
            destination: "home-snapshot".into(),
            readonly: true,
        }
    ));
    assert!(!executes_from_single_step_form(
        &LogicalAction::SetBtrfsLabel {
            filesystem,
            label: "Storage".into(),
        }
    ));
}
