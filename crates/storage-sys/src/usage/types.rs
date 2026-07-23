// SPDX-License-Identifier: GPL-3.0-only

pub type Category = storage_types::UsageCategory;
pub type CategoryTotal = storage_types::UsageCategoryTotal;
pub type TopFileEntry = storage_types::UsageTopFileEntry;
pub type CategoryTopFiles = storage_types::UsageCategoryTopFiles;
pub type ScanResult = storage_types::UsageScanResult;

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub threads: Option<usize>,
    pub top_files_per_category: usize,
    pub show_all_files: bool,
    pub caller_uid: Option<u32>,
    pub caller_gids: Option<Vec<u32>>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            threads: None,
            top_files_per_category: 20,
            show_all_files: false,
            caller_uid: None,
            caller_gids: None,
        }
    }
}
