// SPDX-License-Identifier: GPL-3.0-only

//! Generated scenario-operation inventory.
//!
//! The list mirrors the frozen Phase-0a contract inventory.  It is kept in a
//! small, dependency-free public type so the test backend can prove it covers
//! every public storage operation without parsing Rust source at runtime.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenarioOperation(&'static str);

impl ScenarioOperation {
    pub const ALL: &[Self] = &[
        Self("metadata.id"),
        Self("metadata.capabilities"),
        Self("discovery.list_disks"),
        Self("discovery.list_volumes"),
        Self("discovery.device_events"),
        Self("drive.smart_info"),
        Self("drive.start_smart_selftest"),
        Self("drive.eject"),
        Self("drive.power_off"),
        Self("drive.standby"),
        Self("drive.wakeup"),
        Self("drive.safe_remove"),
        Self("partition.list_partitions"),
        Self("partition.create_partition_table"),
        Self("partition.create_partition"),
        Self("partition.create_partition_with_filesystem"),
        Self("partition.delete_partition"),
        Self("partition.resize_partition"),
        Self("partition.set_partition_type"),
        Self("partition.set_partition_flags"),
        Self("partition.set_partition_name"),
        Self("filesystem.list_filesystems"),
        Self("filesystem.format"),
        Self("filesystem.mount"),
        Self("filesystem.get_mount_point"),
        Self("filesystem.unmount"),
        Self("filesystem.blocking_processes"),
        Self("filesystem.kill_processes"),
        Self("filesystem.check"),
        Self("filesystem.label"),
        Self("filesystem.set_label"),
        Self("filesystem.mount_options"),
        Self("filesystem.reset_mount_options"),
        Self("filesystem.set_mount_options"),
        Self("filesystem.take_ownership"),
        Self("encryption.list_luks_devices"),
        Self("encryption.format_luks"),
        Self("encryption.unlock_luks"),
        Self("encryption.lock_luks"),
        Self("encryption.change_passphrase"),
        Self("encryption.options"),
        Self("encryption.set_options"),
        Self("encryption.clear_options"),
        Self("image_device.open_for_backup"),
        Self("image_device.open_for_restore"),
        Self("image_device.loop_setup"),
        Self("btrfs.list_subvolumes"),
        Self("btrfs.create_subvolume"),
        Self("btrfs.create_snapshot"),
        Self("btrfs.delete_subvolume"),
        Self("btrfs.set_readonly"),
        Self("btrfs.set_default"),
        Self("btrfs.default_subvolume"),
        Self("btrfs.deleted_subvolumes"),
        Self("btrfs.filesystem_usage"),
        Self("logical.source"),
        Self("logical.availability"),
        Self("logical.list_entities"),
        Self("logical.capture_candidate"),
        Self("logical.preflight"),
        Self("logical.execute"),
        Self("network.id"),
        Self("network.capabilities"),
        Self("network.configuration_schema"),
        Self("network.list_configs"),
        Self("network.create_config"),
        Self("network.update_config"),
        Self("network.delete_config"),
        Self("network.test_config"),
        Self("network.mount"),
        Self("network.unmount"),
        Self("network.mount_status"),
        Self("network.mount_on_login"),
        Self("network.set_mount_on_login"),
        Self("tools.list_filesystem_tools"),
        Self("usage.list_mounts"),
        Self("usage.authorize_show_all_files"),
        Self("usage.start_scan"),
        Self("usage.scan_status"),
        Self("usage.wait_for_scan"),
        Self("usage.delete_files"),
        Self("image.create"),
        Self("image.attach"),
        Self("image.start_copy"),
        Self("image.copy_status"),
        Self("image.wait_for_copy"),
        Self("image.cancel_copy"),
        Self("image.forget_copy"),
        Self("desktop.select_image"),
        Self("desktop.reveal"),
        Self("desktop.open_url"),
    ];

    pub const fn as_str(self) -> &'static str {
        self.0
    }

    pub fn all() -> impl ExactSizeIterator<Item = Self> + Clone {
        Self::ALL.iter().copied()
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::all().find(|operation| operation.as_str() == value)
    }
}

impl std::fmt::Display for ScenarioOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
