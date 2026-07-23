use crate::config::Config;
use crate::message::dialogs::{
    AttachDiskImageDialogMessage, FormatDiskMessage, ImageOperationDialogMessage,
    NewDiskImageDialogMessage, SmartDialogMessage, UnmountBusyMessage,
};
use crate::message::network::NetworkMessage;
use crate::message::volumes::VolumesControlMessage;
use crate::models::UiDrive;
use crate::state::app::ContextPage;
use crate::state::dialogs::ShowDialog;
use storage_types::{
    FilesystemToolInfo, UsageCategory, UsageDeleteResult, UsageScanParallelismPreset,
    UsageScanResult,
};

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names, dead_code)]
pub enum Message {
    OpenRepositoryUrl,
    OpenPath(String),
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    VolumesMessage(VolumesControlMessage),
    FormatDisk(FormatDiskMessage),
    DriveRemoved(String),
    DriveAdded(String),
    None,
    UpdateNav(Vec<UiDrive>, Option<String>),
    UpdateNavWithChildSelection(Vec<UiDrive>, Option<String>),
    Dialog(Box<ShowDialog>),
    CloseDialog,
    Eject,
    PowerOff,
    Format,
    SmartData,
    StandbyNow,
    Wakeup,
    FilesystemToolsLoaded(Vec<FilesystemToolInfo>),
    UsageScanLoad {
        scan_id: String,
        top_files_per_category: u32,
        mount_points: Vec<String>,
        show_all_files: bool,
        parallelism_preset: UsageScanParallelismPreset,
    },
    UsageScanLoaded {
        scan_id: String,
        result: Result<UsageScanResult, String>,
    },
    UsageScanProgress {
        scan_id: String,
        processed_bytes: u64,
        estimated_total_bytes: u64,
    },
    UsageCategoryFilterToggled(UsageCategory),
    UsageShowAllFilesToggled(bool),
    UsageShowAllFilesAuthCompleted {
        result: Result<(), String>,
    },
    UsageTopFilesPerCategoryChanged(u32),
    UsageRefreshRequested,
    UsageConfigureRequested,
    UsageWizardMountPointsLoaded {
        result: Result<Vec<String>, String>,
    },
    UsageWizardMountToggled {
        mount_point: String,
        selected: bool,
    },
    UsageWizardShowAllFilesToggled(bool),
    UsageWizardParallelismChanged(usize),
    UsageWizardCancel,
    UsageWizardStartScan,
    UsageWizardShowAllFilesAuthCompleted {
        result: Result<(), String>,
    },
    UsageSelectionSingle {
        path: String,
        index: usize,
    },
    UsageSelectionCtrl {
        path: String,
        index: usize,
    },
    UsageSelectionShift {
        index: usize,
    },
    UsageSelectionModifiersChanged(cosmic::iced::keyboard::Modifiers),
    UsageSelectionClear,
    UsageDeleteStart,
    UsageDeleteCompleted {
        result: Result<UsageDeleteResult, String>,
    },

    // Sidebar (custom treeview)
    SidebarSelectDrive {
        device_path: String,
    },
    SidebarSelectChild {
        device_path: String,
    },
    SidebarClearChildSelection,
    SidebarToggleExpanded(crate::state::sidebar::SidebarNodeKey),
    SidebarDriveEject {
        device_path: String,
    },
    SidebarVolumeUnmount {
        drive: String,
        device_path: String,
    },
    SmartDialog(SmartDialogMessage),
    NewDiskImage,
    AttachDisk,
    CreateDiskFrom,
    RestoreImageTo,
    CreateDiskFromPartition,
    RestoreImageToPartition,
    NewDiskImageDialog(NewDiskImageDialogMessage),
    AttachDiskImageDialog(AttachDiskImageDialogMessage),
    ImageOperationDialog(ImageOperationDialogMessage),
    /// Emitted when Phase 1 completes; store operation_id and start progress subscription.
    ImageOperationStarted(String),
    UnmountBusy(UnmountBusyMessage),
    RetryUnmountAfterKill(String),
    OpenImagePathPicker(ImagePathPickerKind),
    ImagePathPicked(ImagePathPickerKind, Option<String>),
    ToggleShowReserved(bool),
    UsageScanParallelismChanged(usize),
    ToggleLogToDisk(bool),
    LogLevelChanged(usize),

    // BTRFS management
    BtrfsLoadSubvolumes {
        block_path: String,
        mount_point: String,
    },
    BtrfsSubvolumesLoaded {
        mount_point: String,
        result: Result<Vec<storage_types::BtrfsSubvolume>, String>,
    },
    BtrfsDeleteSubvolume {
        block_path: String,
        mount_point: String,
        path: String,
    },
    BtrfsDeleteSubvolumeConfirm {
        block_path: String,
        mount_point: String,
        path: String,
    },
    BtrfsLoadUsage {
        block_path: String,
        mount_point: String,
    },
    BtrfsUsageLoaded {
        mount_point: String,
        used_space: Result<u64, String>,
    },
    BtrfsToggleSubvolumeExpanded {
        mount_point: String,
        subvolume_id: u64,
    },
    BtrfsLoadDefaultSubvolume {
        mount_point: String,
    },
    BtrfsDefaultSubvolumeLoaded {
        mount_point: String,
        result: Result<storage_types::BtrfsSubvolume, String>,
    },
    BtrfsSetDefaultSubvolume {
        mount_point: String,
        subvolume_id: u64,
    },
    BtrfsToggleReadonly {
        mount_point: String,
        subvolume_id: u64,
    },
    BtrfsReadonlyToggled {
        mount_point: String,
        result: Result<(), String>,
    },
    BtrfsShowProperties {
        mount_point: String,
        subvolume_id: u64,
    },
    BtrfsCloseProperties {
        mount_point: String,
    },
    BtrfsLoadDeletedSubvolumes {
        mount_point: String,
    },
    BtrfsDeletedSubvolumesLoaded {
        mount_point: String,
        result: Result<Vec<storage_types::DeletedSubvolume>, String>,
    },
    BtrfsToggleShowDeleted {
        mount_point: String,
    },
    BtrfsRefreshAll {
        mount_point: String,
    },

    // Network mounts (RClone, Samba, FTP)
    Network(NetworkMessage),
    LoadNetworkRemotes,
    NetworkRemotesLoaded(Result<Vec<storage_types::rclone::RemoteConfig>, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePathPickerKind {
    NewDiskImage,
    AttachDiskImage,
    ImageOperationCreate,
    ImageOperationRestore,
}

impl From<FormatDiskMessage> for Message {
    fn from(val: FormatDiskMessage) -> Self {
        Message::FormatDisk(val)
    }
}

impl From<SmartDialogMessage> for Message {
    fn from(val: SmartDialogMessage) -> Self {
        Message::SmartDialog(val)
    }
}

impl From<NewDiskImageDialogMessage> for Message {
    fn from(val: NewDiskImageDialogMessage) -> Self {
        Message::NewDiskImageDialog(val)
    }
}

impl From<AttachDiskImageDialogMessage> for Message {
    fn from(val: AttachDiskImageDialogMessage) -> Self {
        Message::AttachDiskImageDialog(val)
    }
}

impl From<ImageOperationDialogMessage> for Message {
    fn from(val: ImageOperationDialogMessage) -> Self {
        Message::ImageOperationDialog(val)
    }
}

impl From<UnmountBusyMessage> for Message {
    fn from(val: UnmountBusyMessage) -> Self {
        Message::UnmountBusy(val)
    }
}

impl From<NetworkMessage> for Message {
    fn from(val: NetworkMessage) -> Self {
        Message::Network(val)
    }
}
