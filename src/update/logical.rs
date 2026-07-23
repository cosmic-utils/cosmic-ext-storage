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
