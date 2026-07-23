// SPDX-License-Identifier: GPL-3.0-only

//! Device path to UDisks2 block object path index.
//! Used during discovery to resolve device paths when building the volume tree.

use std::collections::HashMap;

use anyhow::Result;
use udisks2::block::BlockProxy;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::dbus::bytestring as bs;

#[derive(Debug, Clone)]
pub(crate) struct BlockIndex {
    by_device: HashMap<String, OwnedObjectPath>,
}

impl BlockIndex {
    fn canonicalize_best_effort(p: &str) -> Option<String> {
        std::fs::canonicalize(p)
            .ok()
            .map(|c| c.to_string_lossy().to_string())
    }

    pub(crate) async fn build(
        connection: &Connection,
        block_objects: &[OwnedObjectPath],
    ) -> Result<Self> {
        let mut by_device = HashMap::new();

        for obj in block_objects {
            let proxy = match BlockProxy::builder(connection).path(obj)?.build().await {
                Ok(p) => p,
                Err(_) => continue,
            };

            let preferred_device = bs::decode_c_string_bytes(&proxy.preferred_device().await?);
            let device = if preferred_device.is_empty() {
                bs::decode_c_string_bytes(&proxy.device().await?)
            } else {
                preferred_device
            };

            if !device.is_empty() {
                by_device.insert(device.clone(), obj.clone());

                if let Some(canon) = Self::canonicalize_best_effort(&device) {
                    by_device.entry(canon).or_insert_with(|| obj.clone());
                }
            }
        }

        Ok(Self { by_device })
    }

    pub(crate) fn object_path_for_device(&self, dev: &str) -> Option<OwnedObjectPath> {
        if let Some(p) = self.by_device.get(dev) {
            return Some(p.clone());
        }

        if let Some(canon) = Self::canonicalize_best_effort(dev) {
            return self.by_device.get(&canon).cloned();
        }

        None
    }
}
