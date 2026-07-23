use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::errors::{Result, TestingError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabSpec {
    pub name: String,
    pub artifacts_root: Option<String>,
    pub images: Vec<ImageSpec>,
    pub partition_table: String,
    pub partitions: Vec<PartitionSpec>,
    pub mounts: Vec<MountSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSpec {
    pub file_name: String,
    pub size_bytes: u64,
    pub loop_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSpec {
    pub index: u32,
    pub start: String,
    pub end: String,
    pub r#type: String,
    pub fs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountSpec {
    pub partition_ref: String,
    pub mount_point: String,
}

pub fn workspace_root() -> PathBuf {
    if let Ok(value) = std::env::var("STORAGE_TESTING_WORKSPACE_ROOT") {
        return PathBuf::from(value);
    }
    let mut directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    while let Some(parent) = directory.parent() {
        if directory.join("resources/lab-specs").is_dir() {
            return directory.to_path_buf();
        }
        directory = parent;
    }
    PathBuf::from(".")
}

pub fn specs_root() -> PathBuf {
    workspace_root().join("resources/lab-specs")
}

pub fn load_by_name(name: &str) -> Result<LabSpec> {
    let path = specs_root().join(format!("{name}.toml"));
    if !path.is_file() {
        return Err(TestingError::SpecNotFound(name.into()));
    }
    let contents = fs::read_to_string(&path).map_err(|error| TestingError::SpecInvalid {
        spec_name: name.into(),
        reason: error.to_string(),
    })?;
    let spec = toml::from_str::<LabSpec>(&contents).map_err(|error| TestingError::SpecInvalid {
        spec_name: name.into(),
        reason: error.to_string(),
    })?;
    validate(&spec)?;
    Ok(spec)
}

pub fn validate(spec: &LabSpec) -> Result<()> {
    let invalid = |reason| TestingError::SpecInvalid {
        spec_name: spec.name.clone(),
        reason,
    };
    if spec.name.trim().is_empty() {
        return Err(invalid("name must not be empty".into()));
    }
    if spec.images.is_empty() {
        return Err(invalid("images must not be empty".into()));
    }
    if !matches!(spec.partition_table.as_str(), "gpt" | "dos") {
        return Err(invalid("partition_table must be gpt or dos".into()));
    }
    if spec.partitions.is_empty() {
        return Err(invalid("partitions must not be empty".into()));
    }
    Ok(())
}
