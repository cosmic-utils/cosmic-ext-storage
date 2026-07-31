use storage_contracts::LogicalAction;
use storage_types::LogicalEntityId;

pub(super) fn action_entity(action: &LogicalAction) -> Option<LogicalEntityId> {
    match action {
        LogicalAction::DeleteLvmVolumeGroup { volume_group, .. }
        | LogicalAction::AddLvmPhysicalVolume { volume_group, .. }
        | LogicalAction::RemoveLvmPhysicalVolume { volume_group, .. }
        | LogicalAction::CreateLvmLogicalVolume { volume_group, .. } => Some(volume_group.clone()),
        LogicalAction::DeleteLvmLogicalVolume { logical_volume, .. }
        | LogicalAction::ResizeLvmLogicalVolume { logical_volume, .. }
        | LogicalAction::ActivateLvmLogicalVolume { logical_volume }
        | LogicalAction::DeactivateLvmLogicalVolume { logical_volume } => {
            Some(logical_volume.clone())
        }
        LogicalAction::DeleteMdRaidArray { array, .. }
        | LogicalAction::StartMdRaidArray { array }
        | LogicalAction::StopMdRaidArray { array }
        | LogicalAction::AddMdRaidMember { array, .. }
        | LogicalAction::RemoveMdRaidMember { array, .. }
        | LogicalAction::RequestMdRaidSync { array, .. } => Some(array.clone()),
        LogicalAction::AddBtrfsDevice { filesystem, .. }
        | LogicalAction::RemoveBtrfsDevice { filesystem, .. }
        | LogicalAction::ResizeBtrfsFilesystem { filesystem, .. }
        | LogicalAction::SetBtrfsLabel { filesystem, .. }
        | LogicalAction::SetBtrfsDefaultSubvolume { filesystem, .. }
        | LogicalAction::CreateBtrfsSubvolume { filesystem, .. }
        | LogicalAction::DeleteBtrfsSubvolume { filesystem, .. }
        | LogicalAction::CreateBtrfsSnapshot { filesystem, .. } => Some(filesystem.clone()),
        LogicalAction::CreateLvmVolumeGroup { .. } | LogicalAction::CreateMdRaidArray { .. } => {
            None
        }
    }
}

pub(super) fn action_label(action: &LogicalAction) -> &'static str {
    match action.operation() {
        storage_types::LogicalOperation::Create => "create",
        storage_types::LogicalOperation::Delete => "delete",
        storage_types::LogicalOperation::Resize => "resize",
        storage_types::LogicalOperation::AddMember => "add member",
        storage_types::LogicalOperation::RemoveMember => "remove member",
        storage_types::LogicalOperation::Activate => "activate",
        storage_types::LogicalOperation::Deactivate => "deactivate",
        storage_types::LogicalOperation::Start => "start",
        storage_types::LogicalOperation::Stop => "stop",
        storage_types::LogicalOperation::Check => "check",
        storage_types::LogicalOperation::Repair => "repair",
        storage_types::LogicalOperation::SetLabel => "set label",
        storage_types::LogicalOperation::SetDefaultSubvolume => "set default subvolume",
    }
}

/// Creating a Btrfs subvolume or snapshot is a single, non-destructive
/// interaction: the form captures its only user values and a fresh preflight
/// still protects its filesystem identity. Other logical operations retain
/// their explicit confirmation step.
pub(super) fn executes_from_single_step_form(action: &LogicalAction) -> bool {
    matches!(
        action,
        LogicalAction::CreateBtrfsSubvolume { .. } | LogicalAction::CreateBtrfsSnapshot { .. }
    )
}

pub(super) fn action_confirmation_body(action: &LogicalAction) -> String {
    match action {
        LogicalAction::DeleteLvmVolumeGroup { volume_group, .. } => {
            format!("Delete volume group {volume_group} and preserve member signatures.")
        }
        LogicalAction::DeleteLvmLogicalVolume { logical_volume, .. } => {
            format!("Delete logical volume {logical_volume}. This cannot be undone.")
        }
        LogicalAction::DeleteMdRaidArray { array, .. } => {
            format!("Delete MD RAID array {array} and preserve member signatures.")
        }
        LogicalAction::DeactivateLvmLogicalVolume { logical_volume } => {
            format!("Deactivate logical volume {logical_volume}.")
        }
        LogicalAction::StopMdRaidArray { array } => format!("Stop MD RAID array {array}."),
        _ => format!(
            "Apply the {} operation through UDisks.",
            action_label(action)
        ),
    }
}

#[cfg(test)]
mod tests {
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
}
