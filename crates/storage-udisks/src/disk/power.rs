// SPDX-License-Identifier: GPL-3.0-only

//! Disk power management operations

use anyhow::Result;
use std::collections::HashMap;
use udisks2::drive::DriveProxy;
use zbus::zvariant::{OwnedObjectPath, Value};

/// Helper to check if error indicates "not supported"
fn is_anyhow_not_supported(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.contains("NotSupported")
        || msg.contains("not supported")
        || msg.contains("No such interface")
}

/// Helper to check if error indicates device is busy
fn is_anyhow_device_busy(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.contains("DeviceBusy") || msg.contains("Device or resource busy")
}

/// Eject a drive by device path (e.g. "/dev/sda")
pub async fn eject_drive_by_device(device: &str, ejectable: bool) -> Result<()> {
    let drive_path = super::resolve::drive_object_path_for_device(device)
        .await
        .map_err(anyhow::Error::msg)?;
    eject_drive(drive_path, ejectable).await
}

/// Eject a drive
pub async fn eject_drive(drive_path: OwnedObjectPath, ejectable: bool) -> Result<()> {
    if !ejectable {
        return Err(anyhow::anyhow!("Not supported by this drive"));
    }

    let connection = crate::manager::shared_connection().await?;
    let proxy = DriveProxy::builder(&connection)
        .path(drive_path)?
        .build()
        .await?;

    match proxy.eject(HashMap::new()).await.map_err(Into::into) {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => {
            Err(anyhow::anyhow!("Not supported by this drive"))
        }
        Err(e) if is_anyhow_device_busy(&e) => Err(anyhow::anyhow!(
            "Device is busy. Unmount any volumes on it and try again."
        )),
        Err(e) => Err(e),
    }
}

/// Power off a drive by device path
pub async fn power_off_drive_by_device(device: &str, can_power_off: bool) -> Result<()> {
    let drive_path = super::resolve::drive_object_path_for_device(device)
        .await
        .map_err(anyhow::Error::msg)?;
    power_off_drive(drive_path, can_power_off).await
}

/// Power off a drive
pub async fn power_off_drive(drive_path: OwnedObjectPath, can_power_off: bool) -> Result<()> {
    if !can_power_off {
        return Err(anyhow::anyhow!("Not supported by this drive"));
    }

    let connection = crate::manager::shared_connection().await?;
    let proxy = DriveProxy::builder(&connection)
        .path(drive_path)?
        .build()
        .await?;

    proxy.power_off(HashMap::new()).await?;
    Ok(())
}

/// Put a drive into standby mode by device path
pub async fn standby_drive_by_device(device: &str) -> Result<()> {
    let drive_path = super::resolve::drive_object_path_for_device(device)
        .await
        .map_err(anyhow::Error::msg)?;
    standby_drive(drive_path).await
}

/// Put a drive into standby mode (spin down)
pub async fn standby_drive(drive_path: OwnedObjectPath) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.Drive.Ata",
    )
    .await?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let res: Result<()> = proxy
        .call("StandbyNow", &(options))
        .await
        .map_err(Into::into);

    match res {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => {
            Err(anyhow::anyhow!("Not supported by this drive"))
        }
        Err(e) => Err(e),
    }
}

/// Wake up a drive from standby by device path
pub async fn wakeup_drive_by_device(device: &str) -> Result<()> {
    let drive_path = super::resolve::drive_object_path_for_device(device)
        .await
        .map_err(anyhow::Error::msg)?;
    wakeup_drive(drive_path).await
}

/// Wake up a drive from standby
pub async fn wakeup_drive(drive_path: OwnedObjectPath) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.Drive.Ata",
    )
    .await?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let res: Result<()> = proxy.call("Wakeup", &(options)).await.map_err(Into::into);

    match res {
        Ok(()) => Ok(()),
        Err(e) if is_anyhow_not_supported(&e) => {
            Err(anyhow::anyhow!("Not supported by this drive"))
        }
        Err(e) => Err(e),
    }
}

/// Remove a drive by device path (loop device delete or removable drive power off)
pub async fn remove_drive_by_device(
    device: &str,
    is_loop: bool,
    removable: bool,
    can_power_off: bool,
) -> Result<()> {
    let block_path = super::resolve::block_object_path_for_device(device)
        .await
        .map_err(anyhow::Error::msg)?;
    let drive_path = if is_loop {
        block_path.clone()
    } else {
        super::resolve::drive_object_path_for_device(device)
            .await
            .map_err(anyhow::Error::msg)?
    };
    remove_drive(
        drive_path,
        block_path.as_str(),
        is_loop,
        removable,
        can_power_off,
    )
    .await
}

/// Remove a drive (loop device delete or removable drive power off)
pub async fn remove_drive(
    drive_path: OwnedObjectPath,
    block_path: &str,
    is_loop: bool,
    removable: bool,
    can_power_off: bool,
) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;

    if is_loop {
        let proxy = zbus::Proxy::new(
            &connection,
            "org.freedesktop.UDisks2",
            block_path,
            "org.freedesktop.UDisks2.Loop",
        )
        .await?;

        let options: HashMap<&str, Value<'_>> = HashMap::new();
        let res: Result<()> = proxy.call("Delete", &(options)).await.map_err(Into::into);

        match res {
            Ok(()) => Ok(()),
            Err(e) if is_anyhow_not_supported(&e) => Err(anyhow::anyhow!(
                "Remove not supported: device does not implement org.freedesktop.UDisks2.Loop"
            )),
            Err(e) if is_anyhow_device_busy(&e) => Err(anyhow::anyhow!(
                "Device is busy. Unmount any volumes on it and try again."
            )),
            Err(e) => Err(e),
        }
    } else if removable {
        // For removable drives, the expected "safe remove" behavior is power off.
        if !can_power_off {
            return Err(anyhow::anyhow!(
                "Remove not supported: drive is removable but does not support power off"
            ));
        }
        power_off_drive(drive_path, can_power_off).await
    } else {
        Err(anyhow::anyhow!(
            "Remove not supported: device is neither a loop-backed image nor a removable drive"
        ))
    }
}
