//! The only process-spawning boundary in the test tool.
//!
//! Production crates never depend on this module.  Commands are represented by
//! a closed enum so an integration test cannot smuggle an arbitrary program or
//! argument into the fixture lifecycle.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    errors::{Result, TestingError},
    ledger::{FixtureCommandRecord, FixtureLedger},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureCommand {
    CreateImage { path: PathBuf, size_bytes: u64 },
    AttachLoop { image: PathBuf },
    DetachLoop { loop_device: PathBuf },
    RescanPartitions { loop_device: PathBuf },
    UnmountFixture { mount_point: PathBuf },
    ClearSignatures { loop_device: PathBuf },
    RemoveArtifact { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub program: String,
    pub arguments: Vec<String>,
    pub stdout: String,
}

/// Validates and executes an audited fixture command.
pub struct FixtureCommandExecutor<'a> {
    artifact_dir: &'a Path,
    ledger: &'a mut FixtureLedger,
}

impl<'a> FixtureCommandExecutor<'a> {
    pub fn new(artifact_dir: &'a Path, ledger: &'a mut FixtureLedger) -> Self {
        Self {
            artifact_dir,
            ledger,
        }
    }

    pub fn execute(&mut self, command: FixtureCommand) -> Result<CommandOutcome> {
        self.validate(&command)?;
        match command {
            FixtureCommand::RemoveArtifact { path } => self.remove_artifact(path),
            command => self.execute_process(command),
        }
    }

    fn validate(&self, command: &FixtureCommand) -> Result<()> {
        let target = match command {
            FixtureCommand::CreateImage { path, .. }
            | FixtureCommand::AttachLoop { image: path }
            | FixtureCommand::RemoveArtifact { path } => {
                self.require_artifact(path)?;
                return Ok(());
            }
            FixtureCommand::DetachLoop { loop_device }
            | FixtureCommand::RescanPartitions { loop_device }
            | FixtureCommand::ClearSignatures { loop_device } => loop_device,
            FixtureCommand::UnmountFixture { mount_point } => {
                self.require_artifact(mount_point)?;
                return Ok(());
            }
        };
        if self.ledger.owns_loop_device(target) {
            Ok(())
        } else {
            Err(TestingError::UnownedFixtureTarget(target.clone()))
        }
    }

    fn require_artifact(&self, target: &Path) -> Result<()> {
        if target.starts_with(self.artifact_dir) && self.ledger.owns(target) {
            Ok(())
        } else {
            Err(TestingError::UnownedFixtureTarget(target.to_path_buf()))
        }
    }

    fn execute_process(&mut self, command: FixtureCommand) -> Result<CommandOutcome> {
        let (program, arguments, target) = match command {
            FixtureCommand::CreateImage { path, size_bytes } => (
                "truncate",
                vec![
                    "--size".into(),
                    size_bytes.to_string(),
                    path.display().to_string(),
                ],
                Some(path),
            ),
            FixtureCommand::AttachLoop { image } => (
                "losetup",
                vec![
                    "--find".into(),
                    "--show".into(),
                    "--partscan".into(),
                    image.display().to_string(),
                ],
                Some(image),
            ),
            FixtureCommand::DetachLoop { loop_device } => (
                "losetup",
                vec!["--detach".into(), loop_device.display().to_string()],
                Some(loop_device),
            ),
            FixtureCommand::RescanPartitions { loop_device } => (
                "partprobe",
                vec![loop_device.display().to_string()],
                Some(loop_device),
            ),
            FixtureCommand::UnmountFixture { mount_point } => (
                "umount",
                vec![mount_point.display().to_string()],
                Some(mount_point),
            ),
            FixtureCommand::ClearSignatures { loop_device } => (
                "wipefs",
                vec!["--all".into(), loop_device.display().to_string()],
                Some(loop_device),
            ),
            FixtureCommand::RemoveArtifact { .. } => {
                return Err(TestingError::UnapprovedFixtureCommand("rm-artifact"));
            }
        };

        // This is intentionally the sole Command::new in tools/storage-testing.
        let output = Command::new(program)
            .args(&arguments)
            .output()
            .map_err(|error| TestingError::FixtureCommandFailed {
                command: render(program, &arguments),
                reason: error.to_string(),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let success = output.status.success();
        let detail = if success {
            stdout.clone()
        } else {
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        };
        self.ledger.record(FixtureCommandRecord {
            program: program.into(),
            arguments: arguments.clone(),
            target,
            success,
            detail: detail.clone(),
        });
        if !success {
            return Err(TestingError::FixtureCommandFailed {
                command: render(program, &arguments),
                reason: detail,
            });
        }
        if program == "losetup" && arguments.first().is_some_and(|value| value == "--find") {
            self.ledger.register_loop_device(stdout.clone())?;
        }
        Ok(CommandOutcome {
            program: program.into(),
            arguments,
            stdout,
        })
    }

    fn remove_artifact(&mut self, path: PathBuf) -> Result<CommandOutcome> {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| TestingError::FixtureCommandFailed {
                command: format!("remove artifact {}", path.display()),
                reason: error.to_string(),
            })?;
        }
        self.ledger.record(FixtureCommandRecord {
            program: "rm-artifact".into(),
            arguments: vec![path.display().to_string()],
            target: Some(path),
            success: true,
            detail: String::new(),
        });
        Ok(CommandOutcome {
            program: "rm-artifact".into(),
            arguments: Vec::new(),
            stdout: String::new(),
        })
    }
}

pub fn effective_uid_is_root() -> bool {
    // This is process-free and deliberately replaces the source `id -u` probe.
    unsafe { libc::geteuid() == 0 }
}

fn render(program: &str, arguments: &[String]) -> String {
    std::iter::once(program)
        .chain(arguments.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}
