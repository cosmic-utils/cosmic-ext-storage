use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum MountStatus {
    #[default]
    Unmounted,
    Mounting,
    Mounted,
    Unmounting,
    Error(String),
}

impl MountStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            MountStatus::Unmounted | MountStatus::Mounted | MountStatus::Error(_)
        )
    }

    pub fn is_mounted(&self) -> bool {
        matches!(self, MountStatus::Mounted)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountType {
    #[default]
    RClone,
}
