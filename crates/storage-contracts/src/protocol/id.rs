// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use serde::{Deserializer, Serializer, de::Error as _};
use uuid::Uuid;

/// Stable, transport-safe operation identity.
///
/// Production IDs remain random, while scenario IDs are generated from the
/// fixture and request sequence.  Keeping the representation textual lets
/// traces be reproducible without leaking implementation handles.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(String);

impl OperationId {
    pub fn new() -> Self {
        Self(format!("op-{}", Uuid::new_v4().simple()))
    }

    pub fn scenario(fixture: &str, operation: &str, sequence: u64) -> Result<Self, String> {
        Self::from_str(&format!("{fixture}:{operation}:{sequence}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for OperationId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.len() > 192
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
        {
            return Err("operation IDs must be lowercase ASCII identifiers".into());
        }
        Ok(Self(value.into()))
    }
}

impl Serialize for OperationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for OperationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}
