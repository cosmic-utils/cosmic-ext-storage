use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::errors::{Result, TestingError};

pub const RUN_ARTIFACT_MARKER: &str = ".storage-testing-run-artifact";

/// Creates a fresh, marker-bearing directory owned by one harness invocation.
pub fn create_run_artifact_dir(label: &str) -> Result<PathBuf> {
    let root = crate::spec::workspace_root().join("target/storage-testing/artifacts");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = root.join(format!("{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).map_err(|error| TestingError::LedgerIo {
        path: path.clone(),
        reason: error.to_string(),
    })?;
    fs::write(
        path.join(RUN_ARTIFACT_MARKER),
        b"storage-testing run artifact\n",
    )
    .map_err(|error| TestingError::LedgerIo {
        path: path.join(RUN_ARTIFACT_MARKER),
        reason: error.to_string(),
    })?;
    Ok(path)
}

/// Reads and validates the directory supplied to a harness invocation.
pub fn required_run_artifact_dir() -> Result<PathBuf> {
    let Some(value) = std::env::var_os("STORAGE_TESTING_ARTIFACT_DIR") else {
        return Err(TestingError::InvalidArtifactDirectory(PathBuf::new()));
    };
    let path = PathBuf::from(value);
    ensure_run_artifact_dir(&path)?;
    Ok(path)
}

pub fn ensure_run_artifact_dir(path: &Path) -> Result<()> {
    if path.is_dir() && path.join(RUN_ARTIFACT_MARKER).is_file() {
        return Ok(());
    }
    Err(TestingError::InvalidArtifactDirectory(path.to_path_buf()))
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| TestingError::LedgerIo {
        path: path.to_path_buf(),
        reason: "artifact has no parent directory".into(),
    })?;
    ensure_run_artifact_dir(parent)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|error| TestingError::LedgerIo {
        path: temporary.clone(),
        reason: error.to_string(),
    })?;
    fs::rename(&temporary, path).map_err(|error| TestingError::LedgerIo {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{RUN_ARTIFACT_MARKER, create_run_artifact_dir, ensure_run_artifact_dir};

    #[test]
    fn created_directories_have_the_required_marker() {
        let directory = create_run_artifact_dir("unit").unwrap();
        assert!(directory.join(RUN_ARTIFACT_MARKER).is_file());
        ensure_run_artifact_dir(&directory).unwrap();
    }
}
