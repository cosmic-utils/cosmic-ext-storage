use crate::models::{UiDrive, UiVolume};
use storage_contracts::{
    BtrfsResizeRequest, ConfirmedLogicalAction, LogicalAction, LogicalDeviceCandidate,
};
use storage_types::{
    BtrfsSubvolumeRef, CreatePartitionInfo, FilesystemToolInfo, LogicalEntityId, PartitionTypeInfo,
    ProcessInfo, SmartAttribute, SmartStatus, VolumeInfo,
};

use crate::state::logical::LogicalDevicePickerAction;

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ShowDialog {
    DeletePartition(DeletePartitionDialog),
    AddPartition(CreatePartitionDialog),
    FormatPartition(FormatPartitionDialog),
    EditPartition(EditPartitionDialog),
    ResizePartition(ResizePartitionDialog),
    EditFilesystemLabel(EditFilesystemLabelDialog),
    EditMountOptions(EditMountOptionsDialog),
    ConfirmAction(ConfirmActionDialog),
    TakeOwnership(TakeOwnershipDialog),
    ChangePassphrase(ChangePassphraseDialog),
    EditEncryptionOptions(EditEncryptionOptionsDialog),
    UnlockEncrypted(UnlockEncryptedDialog),
    FormatDisk(FormatDiskDialog),
    SmartData(SmartDataDialog),
    NewDiskImage(Box<NewDiskImageDialog>),
    AttachDiskImage(Box<AttachDiskImageDialog>),
    ImageOperation(Box<ImageOperationDialog>),
    UnmountBusy(UnmountBusyDialog),
    LogicalActionForm(LogicalActionFormDialog),
    LogicalDevicePicker(LogicalDevicePickerDialog),
    LogicalActionConfirmation(LogicalActionConfirmationDialog),
    Info {
        title: String,
        body: String,
    },
    ConfirmDeleteRemote {
        name: String,
        scope: storage_types::rclone::ConfigScope,
    },
}

#[derive(Debug, Clone)]
pub struct FormatPartitionDialog {
    pub volume: VolumeInfo,
    pub info: CreatePartitionInfo,
    pub step: FormatPartitionStep,
    pub running: bool,
    pub filesystem_tools: Vec<FilesystemToolInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatPartitionStep {
    Basics,
    Options,
}

impl FormatPartitionStep {
    pub const fn number(self) -> usize {
        match self {
            Self::Basics => 1,
            Self::Options => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditPartitionDialog {
    pub volume: VolumeInfo,
    pub step: EditPartitionStep,
    pub partition_types: Vec<PartitionTypeInfo>,
    pub selected_type_index: usize,
    pub name: String,
    pub legacy_bios_bootable: bool,
    pub system_partition: bool,
    pub hidden: bool,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditPartitionStep {
    Basics,
    Flags,
    Review,
}

impl EditPartitionStep {
    pub const fn number(self) -> usize {
        match self {
            Self::Basics => 1,
            Self::Flags => 2,
            Self::Review => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResizePartitionDialog {
    pub volume: VolumeInfo,
    pub step: ResizePartitionStep,
    pub min_size_bytes: u64,
    pub max_size_bytes: u64,
    pub new_size_bytes: u64,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizePartitionStep {
    Sizing,
    Review,
}

impl ResizePartitionStep {
    pub const fn number(self) -> usize {
        match self {
            Self::Sizing => 1,
            Self::Review => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FilesystemTarget {
    Volume(VolumeInfo),
    Node(UiVolume),
}

#[derive(Debug, Clone)]
pub struct ConfirmActionDialog {
    pub title: String,
    pub body: String,
    pub target: FilesystemTarget,
    pub ok_message: crate::app::Message,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct EditFilesystemLabelDialog {
    pub target: FilesystemTarget,
    pub label: String,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct TakeOwnershipDialog {
    pub target: FilesystemTarget,
    pub recursive: bool,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct ChangePassphraseDialog {
    pub volume: VolumeInfo,
    pub current_passphrase: String,
    pub new_passphrase: String,
    pub confirm_passphrase: String,
    pub error: Option<String>,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct EditMountOptionsDialog {
    pub target: FilesystemTarget,
    pub step: EditMountOptionsStep,
    pub use_defaults: bool,
    pub mount_at_startup: bool,
    pub require_auth: bool,
    pub show_in_ui: bool,
    pub other_options: String,
    pub display_name: String,
    pub icon_name: String,
    pub symbolic_icon_name: String,
    pub mount_point: String,
    pub identify_as_options: Vec<String>,
    pub identify_as_index: usize,
    pub filesystem_type: String,
    pub error: Option<String>,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMountOptionsStep {
    Behavior,
    Details,
    Review,
}

impl EditMountOptionsStep {
    pub const fn number(self) -> usize {
        match self {
            Self::Behavior => 1,
            Self::Details => 2,
            Self::Review => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditEncryptionOptionsDialog {
    pub volume: VolumeInfo,
    pub step: EditEncryptionOptionsStep,
    pub use_defaults: bool,
    pub unlock_at_startup: bool,
    pub require_auth: bool,
    pub other_options: String,
    pub name: String,
    pub passphrase: String,
    pub show_passphrase: bool,
    pub error: Option<String>,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditEncryptionOptionsStep {
    Behavior,
    Credentials,
    Review,
}

impl EditEncryptionOptionsStep {
    pub const fn number(self) -> usize {
        match self {
            Self::Behavior => 1,
            Self::Credentials => 2,
            Self::Review => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewDiskImageDialog {
    pub path: String,
    pub size_bytes: u64,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttachDiskImageDialog {
    pub path: String,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageOperationKind {
    CreateFromDrive,
    RestoreToDrive,
    CreateFromPartition,
    RestoreToPartition,
}

#[derive(Debug, Clone)]
pub struct ImageOperationDialog {
    pub kind: ImageOperationKind,
    pub drive: UiDrive,
    pub partition: Option<VolumeInfo>,
    pub image_path: String,
    pub running: bool,
    /// Set when operation has been started (for cancel).
    pub operation_id: Option<String>,
    /// Progress: (bytes_completed, total_bytes, speed_bytes_per_sec).
    pub progress: Option<(u64, u64, u64)>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SmartDataDialog {
    pub drive: UiDrive,
    pub running: bool,
    pub info: Option<(SmartStatus, Vec<SmartAttribute>)>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeletePartitionDialog {
    pub name: String,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct CreatePartitionDialog {
    pub info: CreatePartitionInfo,
    pub step: CreatePartitionStep,
    pub running: bool,
    pub error: Option<String>,
    pub filesystem_tools: Vec<FilesystemToolInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatePartitionStep {
    Basics,
    Sizing,
    Options,
}

impl CreatePartitionStep {
    pub const fn number(self) -> usize {
        match self {
            Self::Basics => 1,
            Self::Sizing => 2,
            Self::Options => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormatDiskDialog {
    pub drive: UiDrive,
    pub erase_index: usize,
    pub partitioning_index: usize,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct UnlockEncryptedDialog {
    pub partition_path: String,
    pub partition_name: String,
    pub passphrase: String,
    pub error: Option<String>,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct UnmountBusyDialog {
    pub device: String,
    pub mount_point: String,
    pub processes: Vec<ProcessInfo>,
    pub device_path: String,
}

/// The input portion of a logical operation.  It contains semantic values
/// only: selected device and subvolume identities are already typed values
/// captured from the current topology, never strings read from a form.
#[derive(Debug, Clone)]
pub enum LogicalActionForm {
    CreateLvmLogicalVolume {
        volume_group: LogicalEntityId,
        name: String,
        size_bytes: String,
    },
    ResizeLvmLogicalVolume {
        logical_volume: LogicalEntityId,
        size_bytes: String,
    },
    ResizeBtrfsFilesystem {
        filesystem: LogicalEntityId,
        size_bytes: String,
    },
    SetBtrfsLabel {
        filesystem: LogicalEntityId,
        label: String,
    },
    CreateBtrfsSubvolume {
        filesystem: LogicalEntityId,
        name: String,
    },
    CreateBtrfsSnapshot {
        filesystem: LogicalEntityId,
        source: BtrfsSubvolumeRef,
        destination: String,
        readonly: bool,
    },
}

impl LogicalActionForm {
    /// Reopen a failed input-taking operation without rebuilding identity from
    /// display strings. Actions not represented by a form return `None`.
    pub fn from_action(action: &LogicalAction) -> Option<Self> {
        match action {
            LogicalAction::CreateLvmLogicalVolume {
                volume_group,
                name,
                size_bytes,
            } => Some(Self::CreateLvmLogicalVolume {
                volume_group: volume_group.clone(),
                name: name.clone(),
                size_bytes: size_bytes.to_string(),
            }),
            LogicalAction::ResizeLvmLogicalVolume {
                logical_volume,
                size_bytes,
            } => Some(Self::ResizeLvmLogicalVolume {
                logical_volume: logical_volume.clone(),
                size_bytes: size_bytes.to_string(),
            }),
            LogicalAction::ResizeBtrfsFilesystem {
                filesystem,
                request: BtrfsResizeRequest::AbsoluteBytes(size_bytes),
            } => Some(Self::ResizeBtrfsFilesystem {
                filesystem: filesystem.clone(),
                size_bytes: size_bytes.to_string(),
            }),
            LogicalAction::SetBtrfsLabel { filesystem, label } => Some(Self::SetBtrfsLabel {
                filesystem: filesystem.clone(),
                label: label.clone(),
            }),
            LogicalAction::CreateBtrfsSubvolume { filesystem, name } => {
                Some(Self::CreateBtrfsSubvolume {
                    filesystem: filesystem.clone(),
                    name: name.clone(),
                })
            }
            LogicalAction::CreateBtrfsSnapshot {
                filesystem,
                source,
                destination,
                readonly,
            } => Some(Self::CreateBtrfsSnapshot {
                filesystem: filesystem.clone(),
                source: source.clone(),
                destination: destination.clone(),
                readonly: *readonly,
            }),
            _ => None,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::CreateLvmLogicalVolume { .. } => "Create logical volume",
            Self::ResizeLvmLogicalVolume { .. } => "Resize logical volume",
            Self::ResizeBtrfsFilesystem { .. } => "Resize Btrfs filesystem",
            Self::SetBtrfsLabel { .. } => "Set Btrfs label",
            Self::CreateBtrfsSubvolume { .. } => "Create Btrfs subvolume",
            Self::CreateBtrfsSnapshot { .. } => "Create Btrfs snapshot",
        }
    }

    pub fn submit_label(&self) -> &'static str {
        match self {
            Self::CreateBtrfsSubvolume { .. } | Self::CreateBtrfsSnapshot { .. } => "Create",
            _ => "Review",
        }
    }

    pub fn set_primary_text(&mut self, text: String) {
        match self {
            Self::CreateLvmLogicalVolume { name, .. }
            | Self::SetBtrfsLabel { label: name, .. }
            | Self::CreateBtrfsSubvolume { name, .. }
            | Self::CreateBtrfsSnapshot {
                destination: name, ..
            } => *name = text,
            Self::ResizeLvmLogicalVolume { .. } | Self::ResizeBtrfsFilesystem { .. } => {}
        }
    }

    pub fn set_size_text(&mut self, text: String) {
        match self {
            Self::CreateLvmLogicalVolume { size_bytes, .. }
            | Self::ResizeLvmLogicalVolume { size_bytes, .. }
            | Self::ResizeBtrfsFilesystem { size_bytes, .. } => *size_bytes = text,
            Self::SetBtrfsLabel { .. }
            | Self::CreateBtrfsSubvolume { .. }
            | Self::CreateBtrfsSnapshot { .. } => {}
        }
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        if let Self::CreateBtrfsSnapshot {
            readonly: value, ..
        } = self
        {
            *value = readonly;
        }
    }

    pub fn action(&self) -> Result<LogicalAction, String> {
        let parse_size = |text: &str| {
            text.parse::<u64>()
                .ok()
                .filter(|size| *size > 0)
                .ok_or_else(|| "Enter a non-zero size in bytes.".to_string())
        };
        let action = match self {
            Self::CreateLvmLogicalVolume {
                volume_group,
                name,
                size_bytes,
            } => LogicalAction::CreateLvmLogicalVolume {
                volume_group: volume_group.clone(),
                name: name.clone(),
                size_bytes: parse_size(size_bytes)?,
            },
            Self::ResizeLvmLogicalVolume {
                logical_volume,
                size_bytes,
            } => LogicalAction::ResizeLvmLogicalVolume {
                logical_volume: logical_volume.clone(),
                size_bytes: parse_size(size_bytes)?,
            },
            Self::ResizeBtrfsFilesystem {
                filesystem,
                size_bytes,
            } => LogicalAction::ResizeBtrfsFilesystem {
                filesystem: filesystem.clone(),
                request: BtrfsResizeRequest::AbsoluteBytes(parse_size(size_bytes)?),
            },
            Self::SetBtrfsLabel { filesystem, label } => LogicalAction::SetBtrfsLabel {
                filesystem: filesystem.clone(),
                label: label.clone(),
            },
            Self::CreateBtrfsSubvolume { filesystem, name } => {
                LogicalAction::CreateBtrfsSubvolume {
                    filesystem: filesystem.clone(),
                    name: name.clone(),
                }
            }
            Self::CreateBtrfsSnapshot {
                filesystem,
                source,
                destination,
                readonly,
            } => LogicalAction::CreateBtrfsSnapshot {
                filesystem: filesystem.clone(),
                source: source.clone(),
                destination: destination.clone(),
                readonly: *readonly,
            },
        };
        action.validate().map_err(|error| error.to_string())?;
        Ok(action)
    }
}

/// A focused input form that precedes the immutable preflight review.
#[derive(Debug, Clone)]
pub struct LogicalActionFormDialog {
    pub form: LogicalActionForm,
    pub error: Option<String>,
}

/// Candidate rows are copied from one current preflight response.  The UI can
/// display blocked rows but only a `Ready` row may emit a block reference.
#[derive(Debug, Clone)]
pub struct LogicalDevicePickerDialog {
    pub picker: LogicalDevicePickerAction,
    pub candidates: Vec<LogicalDeviceCandidate>,
}

/// A typed logical action awaiting an explicit user confirmation.
#[derive(Debug, Clone)]
pub struct LogicalActionConfirmationDialog {
    pub confirmed: ConfirmedLogicalAction,
    pub title: String,
    pub body: String,
    pub running: bool,
}
