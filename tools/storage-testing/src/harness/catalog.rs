use std::{collections::BTreeSet, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::errors::{Result, TestingError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyClass {
    NonDestructive,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    Nondestructive,
    FullLab,
}

impl FromStr for Profile {
    type Err = TestingError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "nondestructive" => Ok(Self::Nondestructive),
            "full-lab" => Ok(Self::FullLab),
            _ => Err(TestingError::InvalidProfile(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseDefinition {
    pub id: &'static str,
    pub suite: &'static str,
    pub fixture_requirements: &'static [&'static str],
    pub safety: SafetyClass,
}

/// Complete source-suite catalog.  Fixture requirements are intentionally
/// ordered: setup uses them as a deterministic acquisition order.
pub const CASES: &[CaseDefinition] = &[
    CaseDefinition {
        id: "btrfs.default_subvolume.set_get",
        suite: "btrfs",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "btrfs.snapshot.create_delete.roundtrip",
        suite: "btrfs",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "btrfs.subvolume.create_delete.roundtrip",
        suite: "btrfs",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "disk.get_disk_info.for_known_device",
        suite: "disk",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "disk.list_disks.non_empty_or_empty_ok",
        suite: "disk",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "disk.list_volumes.schema_integrity",
        suite: "disk",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "filesystem.check.readonly_path",
        suite: "filesystem",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "filesystem.mount.unmount.roundtrip",
        suite: "filesystem",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "filesystem.mount_options.read_write_roundtrip",
        suite: "filesystem",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "filesystem.usage.scan_basic",
        suite: "filesystem",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "image.backup_restore.drive_smoke",
        suite: "image",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "image.loop_setup.valid_image",
        suite: "image",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "logical.btrfs.add_remove_member",
        suite: "logical",
        fixture_requirements: &["3disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "logical.list_entities.schema_integrity",
        suite: "logical",
        fixture_requirements: &["3disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "logical.lvm.create_resize_delete_lv",
        suite: "logical",
        fixture_requirements: &["3disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "logical.mdraid.create_start_stop_delete",
        suite: "logical",
        fixture_requirements: &["3disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "luks.options.read_write_roundtrip",
        suite: "luks",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "luks.unlock_lock.roundtrip",
        suite: "luks",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "partition.create_delete.roundtrip",
        suite: "partition",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "partition.list_partitions.expected_from_spec",
        suite: "partition",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "partition.set_name_type_flags.roundtrip",
        suite: "partition",
        fixture_requirements: &["2disk"],
        safety: SafetyClass::Destructive,
    },
    CaseDefinition {
        id: "rclone.list_remotes.basic",
        suite: "rclone",
        fixture_requirements: &[],
        safety: SafetyClass::NonDestructive,
    },
    CaseDefinition {
        id: "rclone.mount_status.query",
        suite: "rclone",
        fixture_requirements: &[],
        safety: SafetyClass::NonDestructive,
    },
];

pub fn validate_catalog() -> Result<()> {
    let mut ids = BTreeSet::new();
    for case in CASES {
        if !ids.insert(case.id) {
            return Err(TestingError::DuplicateCaseId(case.id.into()));
        }
    }
    Ok(())
}

pub fn suites() -> BTreeSet<&'static str> {
    CASES.iter().map(|case| case.suite).collect()
}

pub fn select_cases(
    profile: Profile,
    suite: Option<&str>,
    case_ids: &[String],
) -> Result<Vec<&'static CaseDefinition>> {
    validate_catalog()?;
    if let Some(suite) = suite
        && !suites().contains(suite)
    {
        return Err(TestingError::UnknownSuite(suite.into()));
    }
    let known_ids: BTreeSet<_> = CASES.iter().map(|case| case.id).collect();
    for id in case_ids {
        if !known_ids.contains(id.as_str()) {
            return Err(TestingError::UnknownCase(id.clone()));
        }
    }
    let explicit: BTreeSet<_> = case_ids.iter().map(String::as_str).collect();
    let mut selected: Vec<_> = CASES
        .iter()
        .filter(|case| suite.is_none_or(|suite| case.suite == suite))
        .filter(|case| explicit.is_empty() || explicit.contains(case.id))
        .filter(|case| profile == Profile::FullLab || case.safety == SafetyClass::NonDestructive)
        .collect();
    for case in CASES.iter().filter(|case| explicit.contains(case.id)) {
        if profile != Profile::FullLab && case.safety == SafetyClass::Destructive {
            return Err(TestingError::DestructiveProfileRequired {
                case_id: case.id.into(),
            });
        }
    }
    selected.sort_by_key(|case| case.id);
    if selected.is_empty() {
        return Err(TestingError::EmptySelection);
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::{Profile, SafetyClass, select_cases};

    #[test]
    fn nondestructive_selection_is_a_profile_filter_not_a_skip() {
        let selected = select_cases(Profile::Nondestructive, None, &[]).unwrap();
        assert!(
            selected
                .iter()
                .all(|case| case.safety == SafetyClass::NonDestructive)
        );
    }
}
