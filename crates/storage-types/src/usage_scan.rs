use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageCategory {
    Documents,
    Images,
    Audio,
    Video,
    Archives,
    Code,
    Binaries,
    Packages,
    System,
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageScanParallelismPreset {
    Low,
    #[default]
    Balanced,
    High,
}

impl UsageScanParallelismPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            UsageScanParallelismPreset::Low => "low",
            UsageScanParallelismPreset::Balanced => "balanced",
            UsageScanParallelismPreset::High => "high",
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            UsageScanParallelismPreset::Low => 0,
            UsageScanParallelismPreset::Balanced => 1,
            UsageScanParallelismPreset::High => 2,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => UsageScanParallelismPreset::Low,
            2 => UsageScanParallelismPreset::High,
            _ => UsageScanParallelismPreset::Balanced,
        }
    }
}

impl FromStr for UsageScanParallelismPreset {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "low" => Ok(UsageScanParallelismPreset::Low),
            "balanced" => Ok(UsageScanParallelismPreset::Balanced),
            "high" => Ok(UsageScanParallelismPreset::High),
            _ => Err(()),
        }
    }
}

impl UsageCategory {
    pub const ALL: [UsageCategory; 10] = [
        UsageCategory::Documents,
        UsageCategory::Images,
        UsageCategory::Audio,
        UsageCategory::Video,
        UsageCategory::Archives,
        UsageCategory::Code,
        UsageCategory::Binaries,
        UsageCategory::Packages,
        UsageCategory::System,
        UsageCategory::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            UsageCategory::Documents => "Documents",
            UsageCategory::Images => "Images",
            UsageCategory::Audio => "Audio",
            UsageCategory::Video => "Video",
            UsageCategory::Archives => "Archives",
            UsageCategory::Code => "Code",
            UsageCategory::Binaries => "Binaries",
            UsageCategory::Packages => "Packages",
            UsageCategory::System => "System",
            UsageCategory::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageCategoryTotal {
    pub category: UsageCategory,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTopFileEntry {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageCategoryTopFiles {
    pub category: UsageCategory,
    pub files: Vec<UsageTopFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScanRequest {
    pub scan_id: String,
    pub top_files_per_category: usize,
    pub show_all_files: bool,
    pub parallelism_preset: UsageScanParallelismPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDeleteFailure {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDeleteResult {
    pub deleted: Vec<String>,
    pub failed: Vec<UsageDeleteFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScanResult {
    pub categories: Vec<UsageCategoryTotal>,
    pub top_files_by_category: Vec<UsageCategoryTopFiles>,
    pub total_bytes: u64,
    pub total_free_bytes: u64,
    pub files_scanned: u64,
    pub dirs_scanned: u64,
    pub skipped_errors: u64,
    pub mounts_scanned: usize,
    pub elapsed_ms: u128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_scan_request_and_delete_result_roundtrip() {
        let request = UsageScanRequest {
            scan_id: "scan-1".into(),
            top_files_per_category: 20,
            show_all_files: false,
            parallelism_preset: UsageScanParallelismPreset::Balanced,
        };
        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: UsageScanRequest = serde_json::from_str(&json).expect("parse request");
        assert_eq!(parsed.scan_id, "scan-1");
        assert_eq!(
            parsed.parallelism_preset,
            UsageScanParallelismPreset::Balanced
        );

        let result = UsageDeleteResult {
            deleted: vec!["/tmp/a".into()],
            failed: vec![UsageDeleteFailure {
                path: "/tmp/b".into(),
                reason: "permission denied".into(),
            }],
        };
        let json = serde_json::to_string(&result).expect("serialize result");
        let parsed: UsageDeleteResult = serde_json::from_str(&json).expect("parse result");
        assert_eq!(parsed.deleted.len(), 1);
        assert_eq!(parsed.failed.len(), 1);
    }
}
