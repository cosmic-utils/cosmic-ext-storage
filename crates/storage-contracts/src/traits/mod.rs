// SPDX-License-Identifier: GPL-3.0-only

pub mod backend;
pub mod btrfs;
pub mod discovery;
pub mod disk;
pub mod filesystem;
pub mod image;
pub mod luks;
pub mod network;
pub mod partition;

pub use backend::{BackendMetadata, BlockStorageBackend, BtrfsBackend};
pub use btrfs::BtrfsOperations;
pub use discovery::{DeviceEventSource, DiskDiscovery};
pub use disk::DriveOperations;
pub use filesystem::FilesystemOperations;
pub use image::ImageDeviceOperations;
pub use luks::EncryptionOperations;
pub use network::NetworkDriveBackend;
pub use partition::PartitionOperations;
