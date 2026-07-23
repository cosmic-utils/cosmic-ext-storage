//! Disposable lab environment checks kept separate from product runtime code.

use crate::errors::{Result, TestingError};

pub fn destructive_environment_enabled() -> bool {
    std::env::var("STORAGE_TESTING_ENABLE_DESTRUCTIVE")
        .ok()
        .as_deref()
        == Some("1")
}

pub fn require_destructive_environment() -> Result<()> {
    if destructive_environment_enabled() {
        Ok(())
    } else {
        Err(TestingError::DestructiveEnvironmentRequired)
    }
}
