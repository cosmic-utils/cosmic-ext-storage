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
    let connection = crate::manager::shared_connection().await?;
    start_drive_smart_selftest_by_device_with_connection(connection.as_ref(), device, kind).await
}

pub(crate) async fn start_drive_smart_selftest_by_device_with_connection(
    connection: &zbus::Connection,
    device: &str,
    kind: SmartSelfTestKind,
) -> Result<()> {
    let drive_path =
        crate::disk::resolve::drive_object_path_for_device_with_connection(connection, device)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    crate::smart::test::start_drive_smart_selftest_with_connection(connection, drive_path, kind)
        .await
}

/// Start a SMART self-test on a drive
///
/// Tries NVMe interface first, falls back to ATA if not supported.
pub async fn start_drive_smart_selftest(
    drive_path: OwnedObjectPath,
    kind: SmartSelfTestKind,
) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    start_drive_smart_selftest_with_connection(connection.as_ref(), drive_path, kind).await
}

pub(crate) async fn start_drive_smart_selftest_with_connection(
    connection: &zbus::Connection,
    drive_path: OwnedObjectPath,
    kind: SmartSelfTestKind,
) -> Result<()> {
    match crate::smart::test::start_nvme_selftest_with_connection(connection, &drive_path, kind)
        .await
    {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => {
            match crate::smart::test::start_ata_selftest_with_connection(
                connection,
                &drive_path,
                kind,
            )
            .await
            {
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
    let connection = crate::manager::shared_connection().await?;
    abort_drive_smart_selftest_with_connection(connection.as_ref(), drive_path).await
}

pub(crate) async fn abort_drive_smart_selftest_with_connection(
    connection: &zbus::Connection,
    drive_path: OwnedObjectPath,
) -> Result<()> {
    match crate::smart::test::abort_nvme_selftest_with_connection(connection, &drive_path).await {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => {
            match crate::smart::test::abort_ata_selftest_with_connection(connection, &drive_path)
                .await
            {
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

pub(crate) async fn start_nvme_selftest_with_connection(
    connection: &zbus::Connection,
    drive_path: &OwnedObjectPath,
    kind: SmartSelfTestKind,
) -> Result<()> {
    let proxy = zbus::Proxy::new(
        connection,
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

pub(crate) async fn abort_nvme_selftest_with_connection(
    connection: &zbus::Connection,
    drive_path: &OwnedObjectPath,
) -> Result<()> {
    let proxy = zbus::Proxy::new(
        connection,
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

pub(crate) async fn start_ata_selftest_with_connection(
    connection: &zbus::Connection,
    drive_path: &OwnedObjectPath,
    kind: SmartSelfTestKind,
) -> Result<()> {
    let proxy = zbus::Proxy::new(
        connection,
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

pub(crate) async fn abort_ata_selftest_with_connection(
    connection: &zbus::Connection,
    drive_path: &OwnedObjectPath,
) -> Result<()> {
    let proxy = zbus::Proxy::new(
        connection,
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
