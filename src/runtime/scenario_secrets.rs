//! Test-only startup input. Values travel through the child's anonymous stdin
//! pipe, never launch arguments, environment, fixture files or control commands.
use std::{collections::BTreeMap, io::Read};

use crate::operations::OperationError;

pub(super) fn read(mut input: impl Read) -> Result<BTreeMap<String, String>, OperationError> {
    let rejected = || OperationError::Failed("invalid scenario secret input".into());
    let mut bytes = Vec::new();
    input
        .by_ref()
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| rejected())?;
    if bytes.len() > 16_384 {
        return Err(rejected());
    }
    // A list, not a JSON object: duplicate IDs must fail rather than overwrite.
    let entries: Vec<(String, String)> = serde_json::from_slice(&bytes).map_err(|_| rejected())?;
    if entries.len() > 16 {
        return Err(rejected());
    }
    let mut secrets = BTreeMap::new();
    for (id, value) in entries {
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || value.is_empty()
            || value.len() > 4096
            || value.chars().any(char::is_control)
            || secrets.insert(id, value).is_some()
        {
            return Err(rejected());
        }
    }
    Ok(secrets)
}

#[cfg(test)]
#[path = "../../tests/unit/runtime/scenario_secrets_tests.rs"]
mod tests;
