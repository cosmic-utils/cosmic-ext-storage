use crate::models::{UiDrive, UiVolume};
use storage_types::{
    CreatePartitionInfo, FilesystemToolInfo, PartitionTypeInfo, ProcessInfo, SmartAttribute,
    SmartStatus, VolumeInfo,
};

#[derive(Debug, Clone)]
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
    BtrfsCreateSubvolume(BtrfsCreateSubvolumeDialog),
    BtrfsCreateSnapshot(BtrfsCreateSnapshotDialog),
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

#[derive(Debug, Clone)]
pub struct BtrfsCreateSubvolumeDialog {
    pub mount_point: String,
    pub block_path: String,
    pub name: String,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BtrfsCreateSnapshotDialog {
    pub mount_point: String,
    pub block_path: String,
    pub subvolumes: Vec<storage_types::BtrfsSubvolume>,
    pub selected_source_index: usize,
    pub snapshot_name: String,
    pub read_only: bool,
    pub running: bool,
    pub error: Option<String>,
}
