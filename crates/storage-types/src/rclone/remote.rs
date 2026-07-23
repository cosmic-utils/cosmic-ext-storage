use super::{ConfigScope, MountStatus, MountType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub name: String,
    pub remote_type: String,
    pub scope: ConfigScope,
    #[serde(default)]
    pub options: HashMap<String, String>,
    #[serde(default)]
    pub has_secrets: bool,
}

impl RemoteConfig {
    pub fn new(name: String, remote_type: String, scope: ConfigScope) -> Self {
        Self {
            name,
            remote_type,
            scope,
            options: HashMap::new(),
            has_secrets: false,
        }
    }

    pub fn mount_point(&self) -> PathBuf {
        self.scope.mount_point(&self.name)
    }

    pub fn validate_name(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Remote name cannot be empty".to_string());
        }
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(
                "Remote name must contain only alphanumeric characters, dashes, or underscores"
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMount {
    pub remote_name: String,
    pub scope: ConfigScope,
    pub status: MountStatus,
    pub mount_point: PathBuf,
    pub mount_type: MountType,
}

impl NetworkMount {
    pub fn from_config(config: &RemoteConfig) -> Self {
        Self {
            remote_name: config.name.clone(),
            scope: config.scope,
            status: MountStatus::Unmounted,
            mount_point: config.mount_point(),
            mount_type: MountType::RClone,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfigList {
    pub remotes: Vec<RemoteConfig>,
    pub user_config_path: Option<PathBuf>,
    pub system_config_path: Option<PathBuf>,
}

impl RemoteConfigList {
    pub fn empty() -> Self {
        Self {
            remotes: Vec::new(),
            user_config_path: None,
            system_config_path: None,
        }
    }
}
