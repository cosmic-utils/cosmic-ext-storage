// SPDX-License-Identifier: GPL-3.0-only

//! The only scenario module allowed to perform host I/O.

use std::path::{Path, PathBuf};

use storage_contracts::{StorageError, StorageErrorKind};

#[derive(Debug, Clone)]
pub struct ScenarioStore {
    fixture: PathBuf,
    overlay: Option<PathBuf>,
    trace: Option<PathBuf>,
}

impl ScenarioStore {
    pub fn new(
        fixture: impl Into<PathBuf>,
        overlay: Option<PathBuf>,
        trace: Option<PathBuf>,
    ) -> Self {
        Self {
            fixture: fixture.into(),
            overlay,
            trace,
        }
    }

    pub fn fixture_path(&self) -> &Path {
        &self.fixture
    }

    pub fn read_fixture(&self) -> Result<Vec<u8>, StorageError> {
        std::fs::read(&self.fixture).map_err(|error| {
            StorageError::new(
                StorageErrorKind::Unavailable,
                format!("cannot read scenario fixture: {error}"),
            )
        })
    }

    pub fn read_overlay(&self) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(path) = &self.overlay else {
            return Ok(None);
        };
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::new(
                StorageErrorKind::Unavailable,
                format!("cannot read scenario overlay: {error}"),
            )),
        }
    }

    pub fn write_overlay_atomically(&self, bytes: &[u8]) -> Result<(), StorageError> {
        let Some(path) = &self.overlay else {
            return Ok(());
        };
        let parent = path.parent().ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario overlay has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))?;
        let temp = parent.join(format!(
            ".{}.tmp",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("scenario")
        ));
        std::fs::write(&temp, bytes)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))?;
        std::fs::rename(&temp, path)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))
    }

    pub fn write_trace(&self, bytes: &[u8]) -> Result<(), StorageError> {
        let Some(path) = &self.trace else {
            return Ok(());
        };
        let parent = path.parent().ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario trace has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))?;
        std::fs::write(path, bytes)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))
    }
}
