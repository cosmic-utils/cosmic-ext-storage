// SPDX-License-Identifier: GPL-3.0-only

//! SMART information retrieval

use crate::SmartInfo;
use anyhow::Result;
use std::collections::HashMap;
use zbus::zvariant::OwnedObjectPath;

/// Helper function to extract a readable value from OwnedValue
fn extract_owned_value(v: &zbus::zvariant::OwnedValue) -> String {
    // Try to convert to common types directly
    if let Ok(s) = <&str>::try_from(v) {
        return s.to_string();
    }
    if let Ok(n) = <i64>::try_from(v) {
        return n.to_string();
    }
    if let Ok(n) = <u64>::try_from(v) {
        return n.to_string();
    }
    if let Ok(n) = <i32>::try_from(v) {
        return n.to_string();
    }
    if let Ok(n) = <u32>::try_from(v) {
        return n.to_string();
    }
    if let Ok(n) = <i16>::try_from(v) {
        return n.to_string();
    }
    if let Ok(n) = <u16>::try_from(v) {
        return n.to_string();
    }
    if let Ok(n) = <f64>::try_from(v) {
        return n.to_string();
    }
    if let Ok(b) = <bool>::try_from(v) {
        return b.to_string();
    }

    // For complex types, parse the debug string more intelligently
    let debug_str = format!("{v:?}");

    // Strip OwnedValue wrapper if present
    let mut s = debug_str.as_str();
    if let Some(stripped) = s.strip_prefix("OwnedValue(")
        && let Some(inner) = stripped.strip_suffix(")")
    {
        s = inner;
    }

    // Handle specific zvariant types
    // U8(value) -> value
    if let Some(rest) = s.strip_prefix("U8(")
        && let Some(num) = rest.strip_suffix(")")
    {
        return num.to_string();
    }

    // I8(value) -> value
    if let Some(rest) = s.strip_prefix("I8(")
        && let Some(num) = rest.strip_suffix(")")
    {
        return num.to_string();
    }

    // Array types - format as [item1, item2, ...]
    if let Some(rest) = s.strip_prefix("Array(")
        && let Some(inner) = rest.strip_suffix(")")
    {
        // Extract elements between "elements: [" and "], signature:"
        if let Some(elements_start) = inner.find("elements:") {
            let after_elements = &inner[elements_start + 9..].trim_start();
            // Find the array content between [ and ]
            if let Some(array_start) = after_elements.find('[') {
                let array_content = &after_elements[array_start + 1..];
                if let Some(array_end) = array_content.find(']') {
                    let elements_str = &array_content[..array_end];
                    // Parse nested types within the array
                    let cleaned = elements_str
                        .replace("U8(", "")
                        .replace("U16(", "")
                        .replace("U32(", "")
                        .replace("U64(", "")
                        .replace("I8(", "")
                        .replace("I16(", "")
                        .replace("I32(", "")
                        .replace("I64(", "")
                        .replace("F64(", "")
                        .replace(")", "");
                    return format!("[{}]", cleaned);
                }
            }
        }
    }

    s.to_string()
}

/// Helper to check if error indicates "not supported"
fn is_anyhow_not_supported(e: &anyhow::Error) -> bool {
    let msg = e.to_string();
    msg.contains("NotSupported")
        || msg.contains("not supported")
        || msg.contains("No such interface")
}

/// Get SMART information for a drive
///
/// Tries NVMe interface first, falls back to ATA if not supported.
pub async fn get_drive_smart_info(drive_path: OwnedObjectPath) -> Result<SmartInfo> {
    match get_nvme_smart_info(&drive_path).await {
        Ok(info) => Ok(info),
        Err(e) if is_anyhow_not_supported(&e) => match get_ata_smart_info(&drive_path).await {
            Ok(info) => Ok(info),
            Err(e2) if is_anyhow_not_supported(&e2) => {
                Err(anyhow::anyhow!("Not supported by this drive"))
            }
            Err(e2) => Err(e2),
        },
        Err(e) => Err(e),
    }
}

/// Get SMART information for a drive by device path (e.g., "/dev/sda")
///
/// This is a convenience wrapper that looks up the UDisks2 object path for the device.
pub async fn get_smart_info_by_device(device: &str) -> Result<SmartInfo> {
    let drive_path = crate::disk::resolve::drive_object_path_for_device(device)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    get_drive_smart_info(drive_path).await
}

async fn get_nvme_smart_info(drive_path: &OwnedObjectPath) -> Result<SmartInfo> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.NVMe.Controller",
    )
    .await?;

    // If the interface isn't present on this drive, properties/methods will error.
    let _state: String = proxy.get_property("State").await?;

    let options: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
    let _: () = proxy.call("SmartUpdate", &(options)).await?;

    let updated_at: Option<u64> = proxy.get_property::<u64>("SmartUpdated").await.ok();
    let temp_k: Option<u16> = proxy.get_property::<u16>("SmartTemperature").await.ok();
    let power_on_hours: Option<u64> = proxy.get_property::<u64>("SmartPowerOnHours").await.ok();
    let selftest_status: Option<String> = proxy
        .get_property::<String>("SmartSelftestStatus")
        .await
        .ok();

    let attrs: HashMap<String, zbus::zvariant::OwnedValue> = proxy
        .call(
            "SmartGetAttributes",
            &(HashMap::<&str, zbus::zvariant::Value<'_>>::new()),
        )
        .await?;

    let mut attributes = std::collections::BTreeMap::new();
    for (k, v) in attrs {
        let value_str = extract_owned_value(&v);
        attributes.insert(k, value_str);
    }

    Ok(SmartInfo {
        device_type: "NVMe".to_string(),
        updated_at,
        temperature_c: temp_k.map(|k| (k as u64).saturating_sub(273)),
        power_on_hours,
        selftest_status,
        attributes,
    })
}

async fn get_ata_smart_info(drive_path: &OwnedObjectPath) -> Result<SmartInfo> {
    let connection = crate::manager::shared_connection().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.UDisks2",
        drive_path.as_str(),
        "org.freedesktop.UDisks2.Drive.Ata",
    )
    .await?;

    // If the interface isn't present on this drive, this will error.
    let _smart_enabled: bool = proxy.get_property("SmartEnabled").await?;

    let options: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
    let _: () = proxy.call("SmartUpdate", &(options)).await?;

    let updated_at: Option<u64> = proxy.get_property::<u64>("SmartUpdated").await.ok();
    let temperature: Option<u64> = proxy.get_property::<u64>("SmartTemperature").await.ok();
    let power_on_seconds: Option<u64> = proxy.get_property::<u64>("SmartPowerOnSeconds").await.ok();
    let selftest_status: Option<String> = proxy
        .get_property::<String>("SmartSelftestStatus")
        .await
        .ok();

    let attrs: HashMap<String, zbus::zvariant::OwnedValue> = proxy
        .call(
            "SmartGetAttributes",
            &(HashMap::<&str, zbus::zvariant::Value<'_>>::new()),
        )
        .await?;

    let mut attributes = std::collections::BTreeMap::new();
    for (k, v) in attrs {
        let value_str = extract_owned_value(&v);
        attributes.insert(k, value_str);
    }

    Ok(SmartInfo {
        device_type: "ATA".to_string(),
        updated_at,
        temperature_c: temperature,
        power_on_hours: power_on_seconds.map(|s| s / 3600),
        selftest_status,
        attributes,
    })
}
