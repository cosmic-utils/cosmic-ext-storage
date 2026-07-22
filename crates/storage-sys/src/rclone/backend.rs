// SPDX-License-Identifier: GPL-3.0-only

//! Per-user rclone implementation of the generic network-drive contract.

use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;
use storage_contracts::{NetworkDriveBackend, StorageError, StorageErrorKind};
use storage_types::{
    NetworkBackendId, NetworkDriveCapabilities, NetworkDriveConfig,
    NetworkDriveConfigurationSchema, NetworkDriveField, NetworkDriveFieldExample,
    NetworkDriveFieldInputKind, NetworkDriveList, NetworkDriveMount, NetworkDriveProviderSchema,
    NetworkDriveStatus, NetworkDriveTestResult,
};

use super::{RCloneCli, is_mount_on_login_enabled, set_mount_on_login};
use crate::error::SysError;
use storage_types::rclone::{rclone_provider, rclone_providers};

/// Rclone's user configuration and user systemd unit adapter.  The public
/// contract has no scope: this adapter never reads `/etc/rclone.conf` or
/// manages a system unit.
#[derive(Debug)]
pub struct RcloneNetworkBackend {
    cli: RCloneCli,
    home: PathBuf,
}

impl RcloneNetworkBackend {
    pub fn new() -> Result<Self, SysError> {
        let cli = RCloneCli::new()?;
        let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
            SysError::OperationFailed(
                "Could not determine the current user's home directory".into(),
            )
        })?;
        Ok(Self { cli, home })
    }

    fn config_path(&self) -> PathBuf {
        self.home.join(".config/rclone/rclone.conf")
    }

    fn mount_point(&self, name: &str) -> PathBuf {
        self.home.join("mnt").join(name)
    }

    fn storage_error(error: SysError) -> StorageError {
        let kind = match error {
            SysError::RCloneConfigParse(_) => StorageErrorKind::InvalidInput,
            SysError::RCloneConfigNotFound | SysError::RCloneNotFound => {
                StorageErrorKind::Unavailable
            }
            SysError::RCloneRemoteNotFound(_) => StorageErrorKind::NotFound,
            SysError::PermissionDenied(_) => StorageErrorKind::PermissionDenied,
            SysError::RCloneAlreadyMounted(_) | SysError::RCloneNotMounted(_) => {
                StorageErrorKind::Conflict
            }
            _ => StorageErrorKind::Internal,
        };
        StorageError::new(kind, error.to_string())
    }

    fn read(&self) -> Result<HashMap<String, HashMap<String, Option<String>>>, StorageError> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }
        self.cli.read_config(&path).map_err(Self::storage_error)
    }

    fn config_from(
        &self,
        name: String,
        values: HashMap<String, Option<String>>,
    ) -> NetworkDriveConfig {
        let provider_id = values
            .get("type")
            .and_then(|value| value.as_deref())
            .unwrap_or("unknown")
            .to_string();
        let has_secrets = rclone_provider(&provider_id).is_some_and(|provider| {
            provider.options.iter().any(|field| {
                field.is_secure()
                    && values
                        .get(&field.name)
                        .and_then(|value| value.as_deref())
                        .is_some_and(|value| !value.trim().is_empty())
            })
        });
        NetworkDriveConfig {
            backend_id: NetworkBackendId::rclone(),
            id: name.clone(),
            name,
            provider_id,
            options: values
                .into_iter()
                .filter(|(key, _)| key != "type")
                .filter_map(|(key, value)| value.map(|value| (key, value)))
                .collect(),
            has_secrets,
        }
    }

    fn get(&self, config_id: &str) -> Result<NetworkDriveConfig, StorageError> {
        self.read()?
            .remove(config_id)
            .map(|values| self.config_from(config_id.to_string(), values))
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    format!("Network drive not found: {config_id}"),
                )
            })
    }

    fn write_config(&self, config: &NetworkDriveConfig, replace: bool) -> Result<(), StorageError> {
        config
            .validate_name()
            .map_err(|message| StorageError::new(StorageErrorKind::InvalidInput, message))?;
        if config.backend_id != NetworkBackendId::rclone() {
            return Err(StorageError::new(
                StorageErrorKind::InvalidInput,
                "Configuration belongs to a different backend",
            ));
        }
        let mut all = self.read()?;
        if !replace && all.contains_key(&config.id) {
            return Err(StorageError::new(
                StorageErrorKind::Conflict,
                "A network drive with that name already exists",
            ));
        }
        let mut fields: HashMap<String, Option<String>> = config
            .options
            .iter()
            .map(|(key, value)| (key.clone(), Some(value.clone())))
            .collect();
        fields.insert("type".into(), Some(config.provider_id.clone()));
        all.insert(config.id.clone(), fields);
        self.cli
            .write_config(&self.config_path(), &all)
            .map_err(Self::storage_error)
    }

    fn field_kind(value_type: &str, has_examples: bool) -> NetworkDriveFieldInputKind {
        match value_type.to_ascii_lowercase().as_str() {
            "bool" | "boolean" => NetworkDriveFieldInputKind::Boolean,
            "int" | "integer" | "number" => NetworkDriveFieldInputKind::Integer,
            _ if has_examples => NetworkDriveFieldInputKind::Choice,
            format => NetworkDriveFieldInputKind::Text {
                validation_format: (!format.is_empty()).then(|| format.to_string()),
            },
        }
    }
}

#[async_trait]
impl NetworkDriveBackend for RcloneNetworkBackend {
    fn id(&self) -> NetworkBackendId {
        NetworkBackendId::rclone()
    }

    fn capabilities(&self) -> NetworkDriveCapabilities {
        NetworkDriveCapabilities::default()
    }

    fn configuration_schema(&self) -> NetworkDriveConfigurationSchema {
        NetworkDriveConfigurationSchema {
            providers: rclone_providers()
                .iter()
                .filter(|provider| !provider.hide)
                .map(|provider| NetworkDriveProviderSchema {
                    id: provider.name.clone(),
                    label: provider.prefix.clone().if_empty_then(&provider.name),
                    description: provider.description.clone(),
                    fields: provider
                        .options
                        .iter()
                        .map(|field| NetworkDriveField {
                            key: field.name.clone(),
                            label: field.name.clone(),
                            help: field.help.clone(),
                            section: field.section.clone(),
                            input_kind: Self::field_kind(
                                &field.value_type,
                                !field.examples.is_empty(),
                            ),
                            default_value: field.default_value.clone(),
                            examples: field
                                .examples
                                .iter()
                                .map(|example| NetworkDriveFieldExample {
                                    value: example.value.clone(),
                                    help: example.help.clone(),
                                })
                                .collect(),
                            choices: field
                                .examples
                                .iter()
                                .map(|example| example.value.clone())
                                .collect(),
                            required: field.required,
                            secret: field.is_secure(),
                            advanced: field.advanced,
                            visible: !field.is_hidden(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    async fn list_configs(&self) -> Result<NetworkDriveList, StorageError> {
        let configs = self
            .read()?
            .into_iter()
            .map(|(name, values)| self.config_from(name, values))
            .collect();
        Ok(NetworkDriveList { configs })
    }

    async fn create_config(&self, config: &NetworkDriveConfig) -> Result<(), StorageError> {
        self.write_config(config, false)
    }

    async fn update_config(&self, config: &NetworkDriveConfig) -> Result<(), StorageError> {
        self.write_config(config, true)
    }

    async fn delete_config(&self, config_id: &str) -> Result<(), StorageError> {
        let mut configs = self.read()?;
        if configs.remove(config_id).is_none() {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                format!("Network drive not found: {config_id}"),
            ));
        }
        self.cli
            .write_config(&self.config_path(), &configs)
            .map_err(Self::storage_error)
    }

    async fn test_config(&self, config_id: &str) -> Result<NetworkDriveTestResult, StorageError> {
        let config = self.get(config_id)?;
        let (success, message, latency_ms) = self
            .cli
            .test_remote(&config.name, &self.config_path())
            .map_err(Self::storage_error)?;
        Ok(NetworkDriveTestResult {
            success,
            message,
            latency_ms: Some(latency_ms),
        })
    }

    async fn mount(&self, config_id: &str) -> Result<(), StorageError> {
        let config = self.get(config_id)?;
        self.cli
            .mount(
                &config.name,
                &self.mount_point(&config.name),
                &self.config_path(),
            )
            .map_err(Self::storage_error)
    }

    async fn unmount(&self, config_id: &str) -> Result<(), StorageError> {
        let config = self.get(config_id)?;
        self.cli
            .unmount(&self.mount_point(&config.name))
            .map_err(Self::storage_error)
    }

    async fn mount_status(&self, config_id: &str) -> Result<NetworkDriveMount, StorageError> {
        let config = self.get(config_id)?;
        let mount_point = self.mount_point(&config.name);
        let status = if RCloneCli::is_mounted(&mount_point).map_err(Self::storage_error)? {
            NetworkDriveStatus::Mounted
        } else {
            NetworkDriveStatus::Unmounted
        };
        Ok(NetworkDriveMount {
            backend_id: NetworkBackendId::rclone(),
            config_id: config.id,
            mount_point,
            status,
        })
    }

    async fn mount_on_login(&self, config_id: &str) -> Result<bool, StorageError> {
        let config = self.get(config_id)?;
        is_mount_on_login_enabled(&config.name, &self.home).map_err(Self::storage_error)
    }

    async fn set_mount_on_login(&self, config_id: &str, enabled: bool) -> Result<(), StorageError> {
        let config = self.get(config_id)?;
        set_mount_on_login(&config.name, enabled, &self.home).map_err(Self::storage_error)
    }
}

trait IfEmptyThen {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl IfEmptyThen for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
