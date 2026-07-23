use super::MountStatus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl TestResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            latency_ms: None,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            latency_ms: None,
        }
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountStatusResult {
    pub status: MountStatus,
    pub mount_point: PathBuf,
}

impl MountStatusResult {
    pub fn new(status: MountStatus, mount_point: PathBuf) -> Self {
        Self {
            status,
            mount_point,
        }
    }
}
