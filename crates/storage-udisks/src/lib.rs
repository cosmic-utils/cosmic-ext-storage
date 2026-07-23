mod dbus;
mod infra;

pub mod backend;

// Error types
pub mod error;

// Domain-based modules (GAP-001.b)
pub mod disk;
pub mod encryption;
pub mod filesystem;
pub mod gpt;
pub mod image;
pub mod lvm;
pub mod manager;
pub mod partition;
pub mod smart;
pub mod volume;

// Re-export storage-types models (canonical domain models)
pub use storage_types;
pub use storage_types::ProcessInfo;

// Re-export format utilities from storage-types
pub use storage_types::{bytes_to_pretty, get_numeric, get_step, pretty_to_bytes};

// Re-export partition type catalog from storage-types
pub use storage_types::{
    COMMON_DOS_TYPES, COMMON_GPT_TYPES, PartitionTypeInfo, PartitionTypeInfoFlags,
    get_all_partition_type_infos, get_valid_partition_names, make_partition_flags_bits,
};

// Re-export volume types from storage-types
pub use storage_types::{VolumeKind, VolumeType};

// Re-export CreatePartitionInfo from storage-types
pub use storage_types::CreatePartitionInfo;

// Re-export GPT alignment from storage-types
pub use storage_types::GPT_ALIGNMENT_BYTES;

// Re-export key types from new modules
// Discovery and operations return storage_types types only
pub use manager::{DeviceEvent, DeviceEventStream, DiskManager};
pub use smart::{SmartInfo, SmartSelfTestKind};

// Re-export error types
pub use backend::UdisksBackend;
pub use error::DiskError;

// Re-export configuration types
pub use encryption::config::EncryptionOptionsSettings;
pub use filesystem::MountOptionsSettings;

// Re-export operations from new domain modules
pub use disk::{
    device_apis::{loop_setup_device_path, open_for_backup_by_device, open_for_restore_by_device},
    discovery::{
        block_object_path_for_device, get_disk_info_for_drive_path, get_disks,
        get_disks_with_partitions, get_disks_with_volumes,
    },
    format::format_disk,
    power::{
        eject_drive_by_device, power_off_drive_by_device, remove_drive_by_device,
        standby_drive_by_device, wakeup_drive_by_device,
    },
};
pub use gpt::{fallback_gpt_usable_range_bytes, probe_gpt_usable_range_bytes};
pub use infra::process::{find_processes_using_mount, kill_processes};
pub use lvm::list_lvs_for_pv;

// Partition operations (from new partition module)
pub use partition::{
    create_partition, create_partition_table, create_partition_with_filesystem, delete_partition,
    edit_partition, resize_partition, set_partition_flags, set_partition_name, set_partition_type,
};

// Filesystem operations (from new filesystem module)
pub use filesystem::{
    check_filesystem, format_filesystem, get_filesystem_label, get_mount_options, get_mount_point,
    mount_filesystem, repair_filesystem, reset_mount_options, set_filesystem_label,
    set_mount_options, take_filesystem_ownership, unmount_filesystem,
};

// Encryption operations (from new encryption module)
pub use encryption::config::{
    clear_encryption_options, get_encryption_options, set_encryption_options,
};
pub use encryption::{
    change_luks_passphrase, format_luks, list_luks_devices, lock_luks, unlock_luks,
};

// SMART operations (from new smart module)
pub use smart::{get_smart_info_by_device, start_drive_smart_selftest_by_device};

// Explicit exports from options module (mount/encryption option parsing)
pub use infra::options::{
    join_options, merge_other_with_managed, normalize_options, remove_prefixed, remove_token,
    set_prefixed_value, set_token_present, split_options, stable_dedup,
};

// Explicit exports from usage module (filesystem usage statistics)
pub use infra::usage::{Usage, usage_for_mount_point};
