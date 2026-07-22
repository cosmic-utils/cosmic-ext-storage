#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditPartitionMessage {
    PrevStep,
    NextStep,
    SetStep(crate::state::dialogs::EditPartitionStep),
    TypeUpdate(usize),
    NameUpdate(String),
    LegacyBiosBootableUpdate(bool),
    SystemPartitionUpdate(bool),
    HiddenUpdate(bool),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResizePartitionMessage {
    PrevStep,
    NextStep,
    SetStep(crate::state::dialogs::ResizePartitionStep),
    SizeUpdate(u64),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditFilesystemLabelMessage {
    LabelUpdate(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditMountOptionsMessage {
    PrevStep,
    NextStep,
    SetStep(crate::state::dialogs::EditMountOptionsStep),
    UseDefaultsUpdate(bool),
    MountAtStartupUpdate(bool),
    RequireAuthUpdate(bool),
    ShowInUiUpdate(bool),
    OtherOptionsUpdate(String),
    DisplayNameUpdate(String),
    IconNameUpdate(String),
    SymbolicIconNameUpdate(String),
    MountPointUpdate(String),
    IdentifyAsIndexUpdate(usize),
    FilesystemTypeUpdate(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeOwnershipMessage {
    RecursiveUpdate(bool),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangePassphraseMessage {
    CurrentUpdate(String),
    NewUpdate(String),
    ConfirmUpdate(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditEncryptionOptionsMessage {
    PrevStep,
    NextStep,
    SetStep(crate::state::dialogs::EditEncryptionOptionsStep),
    UseDefaultsUpdate(bool),
    UnlockAtStartupUpdate(bool),
    RequireAuthUpdate(bool),
    OtherOptionsUpdate(String),
    NameUpdate(String),
    PassphraseUpdate(String),
    ShowPassphraseUpdate(bool),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateMessage {
    PrevStep,
    NextStep,
    SetStep(crate::state::dialogs::CreatePartitionStep),
    SetFormatStep(crate::state::dialogs::FormatPartitionStep),
    SizeUpdate(u64),
    SizeUnitUpdate(usize),
    NameUpdate(String),
    PasswordUpdate(String),
    ConfirmedPasswordUpdate(String),
    PasswordProtectedUpdate(bool),
    EraseUpdate(bool),
    PartitionTypeUpdate(usize),
    Cancel,
    Partition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockMessage {
    PassphraseUpdate(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatDiskMessage {
    EraseUpdate(usize),
    PartitioningUpdate(usize),
    Cancel,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartDialogMessage {
    Refresh,
    SelfTestShort,
    SelfTestExtended,
    AbortSelfTest,
    Close,
    Loaded(
        Result<
            (
                storage_types::SmartStatus,
                Vec<storage_types::SmartAttribute>,
            ),
            String,
        >,
    ),
    ActionComplete(Result<(), String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewDiskImageDialogMessage {
    SizeUpdate(u64),
    Create,
    Cancel,
    Complete(Result<(), String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachDiskImageDialogMessage {
    Attach,
    Cancel,
    Complete(Result<AttachDiskResult, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachDiskResult {
    pub mounted: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageOperationDialogMessage {
    Start,
    CancelOperation,
    /// Progress update from subscription (operation_id, bytes_completed, total_bytes, speed_bytes_per_sec).
    Progress(String, u64, u64, u64),
    Complete(Result<(), String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnmountBusyMessage {
    Cancel,
    Retry,
    KillAndRetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtrfsCreateSubvolumeMessage {
    NameUpdate(String),
    Create,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtrfsCreateSnapshotMessage {
    SourceIndexUpdate(usize),
    NameUpdate(String),
    ReadOnlyUpdate(bool),
    Create,
    Cancel,
}
