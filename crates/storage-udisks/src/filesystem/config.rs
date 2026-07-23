// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem mount options configuration
//!
//! This module provides functions for managing persistent mount options
//! stored in fstab configuration via UDisks2.

use std::collections::HashMap;

use anyhow::Result;
use zbus::zvariant::OwnedValue;

use crate::dbus::bytestring as bs;
use crate::disk::resolve;
use crate::infra::options::{
    join_options, remove_prefixed, remove_token, set_prefixed_value, set_token_present,
    split_options, stable_dedup,
};
use crate::infra::udisks_block_config::{ConfigurationItem, UDisks2BlockConfigurationProxy};

// Re-export from storage-types (canonical domain model)
pub use storage_types::MountOptionsSettings;

fn extract_prefixed_value(tokens: &[String], prefix: &str) -> String {
    tokens
        .iter()
        .find_map(|t| t.strip_prefix(prefix).map(|v| v.to_string()))
        .unwrap_or_default()
}

fn find_configuration_item(items: &[ConfigurationItem], kind: &str) -> Option<ConfigurationItem> {
    items.iter().find(|(t, _)| t == kind).cloned()
}

/// Get mount options settings for a volume
///
/// Returns None if no fstab configuration exists for the device.
pub async fn get_mount_options(device: &str) -> Result<Option<MountOptionsSettings>> {
    let connection = crate::manager::shared_connection().await?;
    let object_path = resolve::block_object_path_for_device(device)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let proxy = UDisks2BlockConfigurationProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await?;

    let items = proxy.configuration().await?;
    let Some((_, dict)) = find_configuration_item(&items, "fstab") else {
        return Ok(None);
    };

    let identify_as = dict
        .get("fsname")
        .and_then(bs::owned_value_to_bytestring)
        .unwrap_or_default();
    let mount_point = dict
        .get("dir")
        .and_then(bs::owned_value_to_bytestring)
        .unwrap_or_default();
    let filesystem_type = dict
        .get("type")
        .and_then(bs::owned_value_to_bytestring)
        .unwrap_or_default();
    let opts = dict
        .get("opts")
        .and_then(bs::owned_value_to_bytestring)
        .unwrap_or_default();

    let tokens = split_options(&opts);
    let mount_at_startup = !tokens.iter().any(|t| t == "noauto");
    let require_auth = tokens.iter().any(|t| t == "x-udisks-auth");
    let show_in_ui = tokens.iter().any(|t| t == "x-gvfs-show");

    let display_name = extract_prefixed_value(&tokens, "x-gvfs-name=");
    let icon_name = extract_prefixed_value(&tokens, "x-gvfs-icon=");
    let symbolic_icon_name = extract_prefixed_value(&tokens, "x-gvfs-symbolic-icon=");

    let mut other_tokens = tokens;
    other_tokens = remove_token(other_tokens, "noauto");
    other_tokens = remove_token(other_tokens, "x-udisks-auth");
    other_tokens = remove_token(other_tokens, "x-gvfs-show");
    other_tokens = remove_prefixed(other_tokens, "x-gvfs-name=");
    other_tokens = remove_prefixed(other_tokens, "x-gvfs-icon=");
    other_tokens = remove_prefixed(other_tokens, "x-gvfs-symbolic-icon=");

    let other_options = join_options(&stable_dedup(other_tokens));

    Ok(Some(MountOptionsSettings {
        identify_as,
        mount_point,
        filesystem_type,
        mount_at_startup,
        require_auth,
        show_in_ui,
        other_options,
        display_name,
        icon_name,
        symbolic_icon_name,
    }))
}

/// Set mount options settings for a volume
#[allow(clippy::too_many_arguments)]
pub async fn set_mount_options(
    device: &str,
    mount_at_startup: bool,
    show_in_ui: bool,
    require_auth: bool,
    display_name: Option<String>,
    icon_name: Option<String>,
    symbolic_icon_name: Option<String>,
    options: String,
    mount_point: String,
    identify_as: String,
    filesystem_type: String,
) -> Result<()> {
    if mount_point.trim().is_empty() {
        anyhow::bail!("Mount point must not be empty");
    }
    if identify_as.trim().is_empty() {
        anyhow::bail!("Identify As must not be empty");
    }
    if filesystem_type.trim().is_empty() {
        anyhow::bail!("Filesystem type must not be empty");
    }

    let mut tokens = split_options(&options);
    tokens = set_token_present(tokens, "noauto", !mount_at_startup);
    tokens = set_token_present(tokens, "x-udisks-auth", require_auth);
    tokens = set_token_present(tokens, "x-gvfs-show", show_in_ui);
    tokens = set_prefixed_value(tokens, "x-gvfs-name=", display_name.as_deref());
    tokens = set_prefixed_value(tokens, "x-gvfs-icon=", icon_name.as_deref());
    tokens = set_prefixed_value(
        tokens,
        "x-gvfs-symbolic-icon=",
        symbolic_icon_name.as_deref(),
    );
    let opts = join_options(&stable_dedup(tokens));
    if opts.trim().is_empty() {
        anyhow::bail!("Mount options must not be empty");
    }

    let connection = crate::manager::shared_connection().await?;
    let object_path = resolve::block_object_path_for_device(device)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let proxy = UDisks2BlockConfigurationProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await?;

    let items = proxy.configuration().await?;
    let old_item = find_configuration_item(&items, "fstab");

    let mut dict: HashMap<String, OwnedValue> = HashMap::new();
    dict.insert(
        "fsname".to_string(),
        bs::bytestring_owned_value(identify_as.trim()),
    );
    dict.insert(
        "dir".to_string(),
        bs::bytestring_owned_value(mount_point.trim()),
    );
    dict.insert(
        "type".to_string(),
        bs::bytestring_owned_value(filesystem_type.trim()),
    );
    dict.insert("opts".to_string(), bs::bytestring_owned_value(opts.trim()));
    dict.insert("freq".to_string(), OwnedValue::from(0i32));
    dict.insert("passno".to_string(), OwnedValue::from(0i32));

    let new_item: ConfigurationItem = ("fstab".to_string(), dict);

    if let Some(old_item) = old_item {
        proxy
            .update_configuration_item(old_item, new_item, HashMap::new())
            .await?;
    } else {
        proxy
            .add_configuration_item(new_item, HashMap::new())
            .await?;
    }

    Ok(())
}

/// Reset mount options to defaults (remove fstab entry)
pub async fn reset_mount_options(device: &str) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let object_path = resolve::block_object_path_for_device(device)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let proxy = UDisks2BlockConfigurationProxy::builder(&connection)
        .path(&object_path)?
        .build()
        .await?;

    let items = proxy.configuration().await?;
    if let Some(old_item) = find_configuration_item(&items, "fstab") {
        proxy
            .remove_configuration_item(old_item, HashMap::new())
            .await?;
    }

    Ok(())
}
