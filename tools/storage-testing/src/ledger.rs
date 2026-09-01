use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    artifacts::write_atomic,
    errors::{Result, TestingError},
};

pub const FIXTURE_LEDGER_FILE: &str = "fixture-ledger.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureTargetKind {
    LoopDevice,
    RunArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureTarget {
    pub path: PathBuf,
    pub kind: FixtureTargetKind,
}

impl FixtureTarget {
    pub fn loop_device(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.to_string_lossy().starts_with("/dev/loop") {
            return Err(TestingError::NonLoopFixtureTarget(path));
        }
        Ok(Self {
            path,
            kind: FixtureTargetKind::LoopDevice,
        })
    }

    pub fn run_artifact(path: impl Into<PathBuf>, artifact_dir: &Path) -> Result<Self> {
        let path = path.into();
        if !path.starts_with(artifact_dir) {
            return Err(TestingError::UnownedFixtureTarget(path));
        }
        Ok(Self {
            path,
            kind: FixtureTargetKind::RunArtifact,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureCommandRecord {
    pub program: String,
    pub arguments: Vec<String>,
    pub target: Option<PathBuf>,
    pub success: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureLedger {
    pub version: u32,
    pub run_id: String,
    pub targets: Vec<FixtureTarget>,
    pub commands: Vec<FixtureCommandRecord>,
    pub cleanup_completed: bool,
}

impl FixtureLedger {
    pub fn new(run_id: impl Into<String>, artifact_dir: &Path) -> Result<Self> {
        Ok(Self {
            version: 1,
            run_id: run_id.into(),
            targets: vec![FixtureTarget::run_artifact(artifact_dir, artifact_dir)?],
            commands: Vec::new(),
            cleanup_completed: false,
        })
    }

    pub fn register_loop_device(&mut self, path: impl Into<PathBuf>) -> Result<()> {
        let target = FixtureTarget::loop_device(path)?;
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
        Ok(())
    }

    pub fn owns(&self, target: &Path) -> bool {
        self.targets.iter().any(|entry| {
            entry.path == target
                || (entry.kind == FixtureTargetKind::RunArtifact && target.starts_with(&entry.path))
        })
    }

    pub fn owns_loop_device(&self, target: &Path) -> bool {
        self.targets
            .iter()
            .any(|entry| entry.path == target && entry.kind == FixtureTargetKind::LoopDevice)
    }

    pub fn record(&mut self, record: FixtureCommandRecord) {
        self.commands.push(record);
    }

    pub fn mark_cleanup_completed(&mut self) {
        self.cleanup_completed = true;
    }

    pub fn persist(&self, artifact_dir: &Path) -> Result<PathBuf> {
        let path = artifact_dir.join(FIXTURE_LEDGER_FILE);
        let contents = serde_json::to_vec_pretty(self).map_err(|error| TestingError::LedgerIo {
            path: path.clone(),
            reason: error.to_string(),
        })?;
        write_atomic(&path, &contents)?;
        Ok(path)
    }

    pub fn load(artifact_dir: &Path) -> Result<Self> {
        let path = artifact_dir.join(FIXTURE_LEDGER_FILE);
        let contents = fs::read(&path).map_err(|error| TestingError::LedgerIo {
            path: path.clone(),
            reason: error.to_string(),
        })?;
        serde_json::from_slice(&contents).map_err(|error| TestingError::LedgerIo {
            path,
            reason: error.to_string(),
        })
    }
}
