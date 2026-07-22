use super::PartitionTypeInfo;
use serde::Deserialize;

// Load TOML data at compile time from storage-udisks data directory
const GPT_TOML: &str = include_str!("../../resources/types/gpt_types.toml");
const DOS_TOML: &str = include_str!("../../resources/types/dos_types.toml");
const APM_TOML: &str = include_str!("../../resources/types/apm_types.toml");
const COMMON_GPT_TOML: &str = include_str!("../../resources/types/common_gpt_types.toml");
const COMMON_DOS_TOML: &str = include_str!("../../resources/types/common_dos_types.toml");

#[derive(Deserialize)]
struct PartitionTypeCatalog {
    types: Vec<PartitionTypeInfo>,
}

static PARTITION_TYPES_DATA: std::sync::LazyLock<Vec<PartitionTypeInfo>> =
    std::sync::LazyLock::new(|| {
        let mut all_types = Vec::new();

        if let Ok(gpt) = toml::from_str::<PartitionTypeCatalog>(GPT_TOML) {
            all_types.extend(gpt.types);
        }

        if let Ok(dos) = toml::from_str::<PartitionTypeCatalog>(DOS_TOML) {
            all_types.extend(dos.types);
        }

        if let Ok(apm) = toml::from_str::<PartitionTypeCatalog>(APM_TOML) {
            all_types.extend(apm.types);
        }

        all_types
    });

pub static PARTITION_TYPES: std::sync::LazyLock<Vec<PartitionTypeInfo>> =
    std::sync::LazyLock::new(|| PARTITION_TYPES_DATA.clone());

pub static COMMON_GPT_TYPES: std::sync::LazyLock<Vec<PartitionTypeInfo>> =
    std::sync::LazyLock::new(|| {
        if let Ok(catalog) = toml::from_str::<PartitionTypeCatalog>(COMMON_GPT_TOML) {
            catalog.types
        } else {
            vec![]
        }
    });

pub static COMMON_DOS_TYPES: std::sync::LazyLock<Vec<PartitionTypeInfo>> =
    std::sync::LazyLock::new(|| {
        if let Ok(catalog) = toml::from_str::<PartitionTypeCatalog>(COMMON_DOS_TOML) {
            catalog.types
        } else {
            vec![]
        }
    });
