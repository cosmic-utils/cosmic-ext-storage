use crate::app::Message;
use crate::message::dialogs::{
    BtrfsCreateSnapshotMessage, BtrfsCreateSubvolumeMessage, ChangePassphraseMessage,
    CreateMessage, EditEncryptionOptionsMessage, EditFilesystemLabelMessage,
    EditMountOptionsMessage, EditPartitionMessage, ResizePartitionMessage, TakeOwnershipMessage,
    UnlockMessage,
};
use crate::state::volumes::DetailTab;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumesControlMessage {
    SegmentSelected(usize),
    SelectDetailTab(DetailTab),
    SelectVolume {
        segment_index: usize,
        device_path: String,
    },
    Mount,
    Unmount,
    ChildMount(String),
    ChildUnmount(String),
    LockContainer,
    Delete,
    OpenFormatPartition,
    OpenEditPartition,
    OpenResizePartition,
    OpenEditFilesystemLabel,
    OpenEditMountOptions,
    OpenCheckFilesystem,
    CheckFilesystemConfirm,
    OpenRepairFilesystem,
    RepairFilesystemConfirm,
    OpenTakeOwnership,
    OpenChangePassphrase,
    OpenEditEncryptionOptions,
    OpenBtrfsCreateSubvolume,
    OpenBtrfsCreateSnapshot,
    CreateMessage(CreateMessage),
    UnlockMessage(UnlockMessage),
    EditPartitionMessage(EditPartitionMessage),
    ResizePartitionMessage(ResizePartitionMessage),
    EditFilesystemLabelMessage(EditFilesystemLabelMessage),
    EditMountOptionsMessage(EditMountOptionsMessage),
    TakeOwnershipMessage(TakeOwnershipMessage),
    ChangePassphraseMessage(ChangePassphraseMessage),
    EditEncryptionOptionsMessage(EditEncryptionOptionsMessage),
    BtrfsCreateSubvolumeMessage(BtrfsCreateSubvolumeMessage),
    BtrfsCreateSnapshotMessage(BtrfsCreateSnapshotMessage),
}

impl From<CreateMessage> for VolumesControlMessage {
    fn from(val: CreateMessage) -> Self {
        VolumesControlMessage::CreateMessage(val)
    }
}

impl From<EditMountOptionsMessage> for VolumesControlMessage {
    fn from(val: EditMountOptionsMessage) -> Self {
        VolumesControlMessage::EditMountOptionsMessage(val)
    }
}

impl From<EditEncryptionOptionsMessage> for VolumesControlMessage {
    fn from(val: EditEncryptionOptionsMessage) -> Self {
        VolumesControlMessage::EditEncryptionOptionsMessage(val)
    }
}

impl From<UnlockMessage> for VolumesControlMessage {
    fn from(val: UnlockMessage) -> Self {
        VolumesControlMessage::UnlockMessage(val)
    }
}

impl From<EditMountOptionsMessage> for Message {
    fn from(val: EditMountOptionsMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::EditMountOptionsMessage(val))
    }
}

impl From<EditEncryptionOptionsMessage> for Message {
    fn from(val: EditEncryptionOptionsMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::EditEncryptionOptionsMessage(val))
    }
}

impl From<EditPartitionMessage> for VolumesControlMessage {
    fn from(val: EditPartitionMessage) -> Self {
        VolumesControlMessage::EditPartitionMessage(val)
    }
}

impl From<ResizePartitionMessage> for VolumesControlMessage {
    fn from(val: ResizePartitionMessage) -> Self {
        VolumesControlMessage::ResizePartitionMessage(val)
    }
}

impl From<EditFilesystemLabelMessage> for VolumesControlMessage {
    fn from(val: EditFilesystemLabelMessage) -> Self {
        VolumesControlMessage::EditFilesystemLabelMessage(val)
    }
}

impl From<TakeOwnershipMessage> for VolumesControlMessage {
    fn from(val: TakeOwnershipMessage) -> Self {
        VolumesControlMessage::TakeOwnershipMessage(val)
    }
}

impl From<ChangePassphraseMessage> for VolumesControlMessage {
    fn from(val: ChangePassphraseMessage) -> Self {
        VolumesControlMessage::ChangePassphraseMessage(val)
    }
}

impl From<CreateMessage> for Message {
    fn from(val: CreateMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::CreateMessage(val))
    }
}

impl From<UnlockMessage> for Message {
    fn from(val: UnlockMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::UnlockMessage(val))
    }
}

impl From<EditPartitionMessage> for Message {
    fn from(val: EditPartitionMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::EditPartitionMessage(val))
    }
}

impl From<ResizePartitionMessage> for Message {
    fn from(val: ResizePartitionMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::ResizePartitionMessage(val))
    }
}

impl From<EditFilesystemLabelMessage> for Message {
    fn from(val: EditFilesystemLabelMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::EditFilesystemLabelMessage(val))
    }
}

impl From<TakeOwnershipMessage> for Message {
    fn from(val: TakeOwnershipMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::TakeOwnershipMessage(val))
    }
}

impl From<ChangePassphraseMessage> for Message {
    fn from(val: ChangePassphraseMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::ChangePassphraseMessage(val))
    }
}

impl From<BtrfsCreateSubvolumeMessage> for VolumesControlMessage {
    fn from(val: BtrfsCreateSubvolumeMessage) -> Self {
        VolumesControlMessage::BtrfsCreateSubvolumeMessage(val)
    }
}

impl From<BtrfsCreateSubvolumeMessage> for Message {
    fn from(val: BtrfsCreateSubvolumeMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::BtrfsCreateSubvolumeMessage(val))
    }
}

impl From<BtrfsCreateSnapshotMessage> for VolumesControlMessage {
    fn from(val: BtrfsCreateSnapshotMessage) -> Self {
        VolumesControlMessage::BtrfsCreateSnapshotMessage(val)
    }
}

impl From<BtrfsCreateSnapshotMessage> for Message {
    fn from(val: BtrfsCreateSnapshotMessage) -> Self {
        Message::VolumesMessage(VolumesControlMessage::BtrfsCreateSnapshotMessage(val))
    }
}

impl From<VolumesControlMessage> for Message {
    fn from(val: VolumesControlMessage) -> Self {
        Message::VolumesMessage(val)
    }
}
