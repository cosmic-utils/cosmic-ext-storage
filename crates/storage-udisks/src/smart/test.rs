// SPDX-License-Identifier: GPL-3.0-only

//! SMART self-test operations

use crate::SmartSelfTestKind;
use anyhow::Result;
use std::collections::HashMap;
use zbus::zvariant::{OwnedObjectPath, Value};

/// Helper to check if error indicates "not supported"
fn is_anyhow_not_supported(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.contains("NotSupported")
        || msg.contains("not supported")
        || msg.contains("No such interface")
}

/// Start a SMART self-test on a drive by device path (e.g. "/dev/sda")
pub async fn start_drive_smart_selftest_by_device(
    device: &str,
    kind: SmartSelfTestKind,
) -> Result<()> {
    let drive_path = crate::disk::resolve::drive_object_path_for_device(device)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    start_drive_smart_selftest(drive_path, kind).await
}

/// Start a SMART self-test on a drive
///
/// Tries NVMe interface first, falls back to ATA if not supported.
pub async fn start_drive_smart_selftest(
    drive_path: OwnedObjectPath,
    kind: SmartSelfTestKind,
) -> Result<()> {
    match start_nvme_selftest(&drive_path, kind).await {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => {
            match start_ata_selftest(&drive_path, kind).await {
                Ok(()) => Ok(()),
                Err(e2) if is_anyhow_not_supported(&e2) => {
                    Err(anyhow::anyhow!("Not supported by this drive"))
                }
                Err(e2) => Err(e2),
            }
        }
        Err(e) => Err(e),
    }
}

/// Abort a SMART self-test on a drive
///
/// Tries NVMe interface first, falls back to ATA if not supported.
pub async fn abort_drive_smart_selftest(drive_path: OwnedObjectPath) -> Result<()> {
    match abort_nvme_selftest(&drive_path).await {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => match abort_ata_selftest(&drive_path).await {
            Ok(()) => Ok(()),
            Err(e2) if is_anyhow_not_supported(&e2) => {
                Err(anyhow::anyhow!("Not supported by this drive"))
            }
            Err(e2) => Err(e2),
        },
        Err(e) => Err(e),
    }
}

async fn start_nvme_selftest(drive_path: &OwnedObjectPath, kind: SmartSelfTestKind) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.NVMe.Controller",
    )
    .await?;

    let _state: String = proxy.get_property("State").await?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let _: () = proxy
        .call("SmartSelftestStart", &(kind.as_udisks_str(), options))
        .await?;
    Ok(())
}

async fn abort_nvme_selftest(drive_path: &OwnedObjectPath) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.NVMe.Controller",
    )
    .await?;

    let _state: String = proxy.get_property("State").await?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let _: () = proxy.call("SmartSelftestAbort", &(options)).await?;
    Ok(())
}

async fn start_ata_selftest(drive_path: &OwnedObjectPath, kind: SmartSelfTestKind) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.Drive.Ata",
    )
    .await?;

    let _smart_enabled: bool = proxy.get_property("SmartEnabled").await?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let _: () = proxy
        .call("SmartSelftestStart", &(kind.as_udisks_str(), options))
        .await?;
    Ok(())
}

async fn abort_ata_selftest(drive_path: &OwnedObjectPath) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.Drive.Ata",
    )
    .await?;

    let _smart_enabled: bool = proxy.get_property("SmartEnabled").await?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let _: () = proxy.call("SmartSelftestAbort", &(options)).await?;
    Ok(())
}
