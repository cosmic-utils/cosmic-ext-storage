// SPDX-License-Identifier: GPL-3.0-only

//! Shared D-Bus call helpers for image operations
//!
//! This module provides reusable helpers for calling UDisks2 methods,
//! reducing duplication across image submodules (loop_setup, backup).

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use zbus::{
    Connection, Proxy,
    zvariant::{DynamicType, OwnedObjectPath, Type},
};

/// Format a D-Bus object path for display in error messages
pub fn device_for_display(object_path: &OwnedObjectPath) -> String {
    object_path.to_string()
}

/// Call a raw UDisks2 method with typed arguments and response
///
/// This centralizes error handling and D-Bus object path formatting
/// for image operations (loop_setup, backup, restore).
pub async fn call_udisks_raw<R, B>(
    connection: &Connection,
    path: &OwnedObjectPath,
    interface: &str,
    method: &str,
    args: &B,
) -> Result<R>
where
    R: DeserializeOwned + Type,
    B: Serialize + DynamicType,
{
    let proxy = Proxy::new(connection, "org.freedesktop.UDisks2", path, interface).await?;

    match proxy.call_method(method, args).await {
        Ok(reply) => Ok(reply.body().deserialize()?),
        Err(err) => {
            if let zbus::Error::MethodError(name, msg, _info) = &err {
                let device = device_for_display(path);
                let msg = msg.as_deref().unwrap_or("");
                anyhow::bail!(
                    "UDisks2 {interface}.{method} failed for {device}: {}{}{}",
                    name.as_str(),
                    if msg.is_empty() { "" } else { ": " },
                    msg
                );
            }
            Err(err.into())
        }
    }
}
