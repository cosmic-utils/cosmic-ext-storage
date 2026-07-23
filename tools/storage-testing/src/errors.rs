use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestingError {
    #[error("invalid harness profile '{0}'")]
    InvalidProfile(String),
    #[error("unknown harness suite '{0}'")]
    UnknownSuite(String),
    #[error("unknown harness case '{0}'")]
    UnknownCase(String),
    #[error("the selected harness case catalog is empty")]
    EmptySelection,
    #[error("duplicate harness case id '{0}'")]
    DuplicateCaseId(String),
    #[error("destructive case '{case_id}' requires the full-lab profile")]
    DestructiveProfileRequired { case_id: String },
    #[error("destructive fixture execution requires STORAGE_TESTING_ENABLE_DESTRUCTIVE=1")]
    DestructiveEnvironmentRequired,
    #[error("STORAGE_TESTING_ARTIFACT_DIR must name an accessible run-artifact directory: {0}")]
    InvalidArtifactDirectory(PathBuf),
    #[error("fixture target is not owned by the current ledger: {0}")]
    UnownedFixtureTarget(PathBuf),
    #[error("fixture target must be a loop-backed device: {0}")]
    NonLoopFixtureTarget(PathBuf),
    #[error("fixture command is not in the reviewed allow-list: {0}")]
    UnapprovedFixtureCommand(&'static str),
    #[error("fixture command failed: {command}; {reason}")]
    FixtureCommandFailed { command: String, reason: String },
    #[error("fixture ledger I/O failed for {path:?}: {reason}")]
    LedgerIo { path: PathBuf, reason: String },
    #[error("lab spec '{spec_name}' is invalid: {reason}")]
    SpecInvalid { spec_name: String, reason: String },
    #[error("lab spec '{0}' was not found")]
    SpecNotFound(String),
    #[error("required execution did not complete successfully: {0}")]
    RequiredExecution(String),
    #[error("harness argument error: {0}")]
    Argument(String),
}

pub type Result<T> = std::result::Result<T, TestingError>;
