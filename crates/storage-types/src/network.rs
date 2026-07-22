// SPDX-License-Identifier: GPL-3.0-only

//! Backend-neutral network-drive models.
//!
//! These values are deliberately independent of rclone.  A backend owns its
//! command syntax and configuration files while the application renders this
//! schema and routes values by stable backend/configuration identifiers.

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NetworkBackendId(pub String);

impl NetworkBackendId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn rclone() -> Self {
        Self::new("rclone")
    }
}

impl std::fmt::Display for NetworkBackendId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkBackendAvailability {
    Available,
    Unavailable { reason: String },
}

impl NetworkBackendAvailability {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveCapabilities {
    pub supports_mount_on_login: bool,
    pub supports_connection_test: bool,
}

impl Default for NetworkDriveCapabilities {
    fn default() -> Self {
        Self {
            supports_mount_on_login: true,
            supports_connection_test: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveConfig {
    pub backend_id: NetworkBackendId,
    pub id: String,
    pub name: String,
    pub provider_id: String,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    #[serde(default)]
    pub has_secrets: bool,
}

impl NetworkDriveConfig {
    pub fn validate_name(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Network drive name cannot be empty".into());
        }
        if !self
            .name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '-' || character == '_')
        {
            return Err(
                "Network drive name must contain only alphanumeric characters, dashes, or underscores"
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveList {
    pub configs: Vec<NetworkDriveConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NetworkDriveStatus {
    #[default]
    Unmounted,
    Mounting,
    Mounted,
    Unmounting,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveMount {
    pub backend_id: NetworkBackendId,
    pub config_id: String,
    pub mount_point: PathBuf,
    pub status: NetworkDriveStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveConfigurationSchema {
    pub providers: Vec<NetworkDriveProviderSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveProviderSchema {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub fields: Vec<NetworkDriveField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkDriveFieldInputKind {
    Boolean,
    Integer,
    Choice,
    Text { validation_format: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveFieldExample {
    pub value: String,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDriveField {
    pub key: String,
    pub label: String,
    pub help: String,
    pub section: String,
    pub input_kind: NetworkDriveFieldInputKind,
    pub default_value: String,
    #[serde(default)]
    pub examples: Vec<NetworkDriveFieldExample>,
    #[serde(default)]
    pub choices: Vec<String>,
    pub required: bool,
    pub secret: bool,
    pub advanced: bool,
    pub visible: bool,
}
