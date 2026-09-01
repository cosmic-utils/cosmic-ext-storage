// SPDX-License-Identifier: GPL-3.0-only

//! Portable values used by long-running UI workflows.
//!
//! These values intentionally describe a user-visible operation rather than a
//! file descriptor, process, or host path.  Production adapters may keep those
//! implementation details privately; scenario adapters only ever receive the
//! synthetic identifiers below.

use serde::{Deserialize, Serialize};

use crate::{UsageDeleteResult, UsageScanParallelismPreset, UsageScanResult};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImageAssetRef(String);

impl ImageAssetRef {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let Some(id) = value.strip_prefix("asset:") else {
            return Err("image assets must use the asset:<id> form".into());
        };
        if id.is_empty()
            || id.len() > 63
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err("image asset IDs may contain only letters, numbers, '-' and '_'".into());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ImageAssetRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageCopyKind {
    Backup,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageCopyRequest {
    pub kind: ImageCopyKind,
    pub device: String,
    pub asset: ImageAssetRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAttachmentRequest {
    pub asset: ImageAssetRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub device: String,
    pub mounted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageWorkflowStatus {
    pub operation_id: String,
    pub state: WorkflowState,
    pub bytes_completed: u64,
    pub bytes_total: u64,
    pub speed_bytes_per_sec: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageWorkflowRequest {
    pub scan_id: String,
    pub mounts: Vec<String>,
    pub top_files_per_category: u32,
    pub show_all_files: bool,
    pub parallelism_preset: UsageScanParallelismPreset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageWorkflowStatus {
    pub scan_id: String,
    pub state: WorkflowState,
    pub processed_bytes: u64,
    pub estimated_total_bytes: u64,
    pub result: Option<UsageScanResult>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageDeleteRequest {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageDeleteResponse {
    pub result: UsageDeleteResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DesktopImageSelection {
    Asset(ImageAssetRef),
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_asset_refs_are_capability_free() {
        assert!(ImageAssetRef::new("asset:fixture-image").is_ok());
        assert!(ImageAssetRef::new("/tmp/image.img").is_err());
        assert!(ImageAssetRef::new("asset:../../etc/passwd").is_err());
    }
}
