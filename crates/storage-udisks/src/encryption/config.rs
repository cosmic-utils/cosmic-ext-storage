// SPDX-License-Identifier: GPL-3.0-only

//! LUKS encryption options configuration
//!
//! This module provides functions for managing persistent encryption options
//! stored in crypttab configuration via UDisks2.

use std::collections::HashMap;

use anyhow::Result;
use zbus::zvariant::OwnedValue;

use crate::dbus::bytestring as bs;
use crate::disk::resolve;
use crate::error::DiskError;
use crate::infra::options::{
    join_options, remove_prefixed, remove_token, set_token_present, split_options, stable_dedup,
};
use crate::infra::udisks_block_config::{ConfigurationItem, UDisks2BlockConfigurationProxy};

// Re-export from storage-types (canonical domain model)
pub use storage_types::EncryptionOptionsSettings;

/// Get encryption options settings for a LUKS device
///
/// Returns None if no crypttab configuration exists for the device.
pub async fn get_encryption_options(device: &str) -> Result<Option<EncryptionOptionsSettings>> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let object_path = resolve::block_object_path_for_device(device)
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let proxy = UDisks2BlockConfigurationProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let items = proxy
        .configuration()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let Some(crypttab_item) = items.iter().find(|(t, _)| *t == "crypttab") else {
        return Ok(None);
    };

    // Extract fields from crypttab entry
    let (_, dict) = crypttab_item;
    let name_str = dict.get("name").and_then(bs::owned_value_to_bytestring);
    let opts_str = dict.get("options").and_then(bs::owned_value_to_bytestring);
    let _passphrase_path_str = dict
        .get("passphrase-path")
        .and_then(bs::owned_value_to_bytestring);
    let _passphrase_contents_str = dict
        .get("passphrase-contents")
        .and_then(bs::owned_value_to_bytestring);

    // Build EncryptionOptionsSettings from crypttab entry
    let settings = EncryptionOptionsSettings {
        name: name_str.unwrap_or_default().trim().to_string(),
        unlock_at_startup: false,
        require_auth: opts_str
            .as_ref()
            .map(|o| split_options(o).iter().any(|t| t == "x-udisks-auth"))
            .unwrap_or(false),
        other_options: opts_str
            .as_ref()
            .map(|o| {
                let mut tokens = split_options(o);
                // Remove standard options we manage
                tokens = remove_prefixed(tokens, "x-udisks-auth");
                tokens = remove_token(tokens, "noauto");
                join_options(&tokens)
            })
            .unwrap_or_default()
            .trim()
            .to_string(),
        passphrase: None, // Don't expose passphrase when reading from crypttab
    };

    Ok(Some(settings))
}

/// Set encryption options settings for a LUKS device
///
/// Creates or updates a crypttab entry via UDisks2 BlockConfiguration.
///
/// # Arguments
/// * `device` - Device path (e.g., "/dev/sda1")
/// * `settings` - Encryption options to write to crypttab
///
/// # Behavior
/// * Creates crypttab entry if one does not exist
/// * Updates existing entry if crypttab entry already exists
/// * Validates required fields (name must not be empty)
pub async fn set_encryption_options(
    device: &str,
    settings: &EncryptionOptionsSettings,
) -> Result<()> {
    if settings.name.trim().is_empty() {
        anyhow::bail!("Name must not be empty");
    }

    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let object_path = resolve::block_object_path_for_device(device)
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let proxy = UDisks2BlockConfigurationProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    // Get block UUID for device identification
    let block_proxy = udisks2::block::BlockProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let id_uuid = block_proxy
        .id_uuid()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .trim()
        .to_string();

    if id_uuid.is_empty() {
        anyhow::bail!("Missing block UUID; cannot write crypttab entry");
    }

    let device_uuid = format!("UUID={}", id_uuid);

    // Build mount options from settings
    let mut tokens = split_options(&settings.other_options);

    // Extract existing passphrase path before modifying tokens further
    let existing_passphrase_path = tokens
        .iter()
        .find(|t| t.starts_with("persist="))
        .and_then(|t| t.split('=').nth(1))
        .map(|s| s.to_string());

    tokens = set_token_present(tokens, "noauto", !settings.unlock_at_startup);
    tokens = set_token_present(tokens, "x-udisks-auth", settings.require_auth);
    let opts = join_options(&stable_dedup(tokens));

    // Build crypttab options dictionary
    let mut dict: HashMap<String, OwnedValue> = HashMap::new();
    dict.insert(
        "device".to_string(),
        bs::bytestring_owned_value(&device_uuid),
    );
    dict.insert(
        "name".to_string(),
        bs::bytestring_owned_value(&settings.name),
    );
    dict.insert("options".to_string(), bs::bytestring_owned_value(&opts));

    // Add passphrase fields if provided
    if let Some(passphrase) = &settings.passphrase {
        // Default passphrase file location: /etc/luks-keys/{name}
        let default_passphrase_path = format!("/etc/luks-keys/{}", settings.name.trim());

        // Use existing passphrase path if one was set in options
        let passphrase_path = existing_passphrase_path.unwrap_or(default_passphrase_path);

        dict.insert(
            "passphrase-path".to_string(),
            bs::bytestring_owned_value(&passphrase_path),
        );
        dict.insert(
            "passphrase-contents".to_string(),
            bs::bytestring_owned_value(passphrase),
        );
    }

    let new_item: ConfigurationItem = ("crypttab".to_string(), dict);

    let items = proxy
        .configuration()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    if let Some(old_item) = items.iter().find(|(t, _)| *t == "crypttab") {
        proxy
            .update_configuration_item(old_item.clone(), new_item, HashMap::new())
            .await
            .map_err(|e| DiskError::DBusError(e.to_string()))?;
    } else {
        proxy
            .add_configuration_item(new_item, HashMap::new())
            .await
            .map_err(|e| DiskError::DBusError(e.to_string()))?;
    }

    Ok(())
}

/// Clear encryption options (remove crypttab entry) for a LUKS device
///
/// Removes the crypttab entry for the device if one exists.
pub async fn clear_encryption_options(device: &str) -> Result<()> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let object_path = resolve::block_object_path_for_device(device)
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let proxy = UDisks2BlockConfigurationProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let items = proxy
        .configuration()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    if let Some(crypttab_item) = items.iter().find(|(t, _)| *t == "crypttab") {
        proxy
            .remove_configuration_item(crypttab_item.clone(), HashMap::new())
            .await
            .map_err(|e| DiskError::DBusError(e.to_string()))?;
    }

    Ok(())
}
