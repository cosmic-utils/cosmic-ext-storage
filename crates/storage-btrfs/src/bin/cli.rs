// SPDX-License-Identifier: GPL-3.0-only

//! CLI wrapper around storage-btrfs library for testing and manual operations

use anyhow::Result;
use clap::{Parser, Subcommand};
use disks_btrfs::{SubvolumeList, SubvolumeManager, get_filesystem_usage};
use std::path::PathBuf;

/// Privileged helper for BTRFS subvolume operations
#[derive(Parser)]
#[command(name = "storage-btrfs-cli")]
#[command(about = "CLI tool for BTRFS operations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all subvolumes in a BTRFS filesystem
    List {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
    },
    /// Create a new subvolume
    Create {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
        /// Name of the subvolume to create
        name: String,
    },
    /// Delete a subvolume
    Delete {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
        /// Path to the subvolume to delete
        path: PathBuf,
        /// Delete recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Create a snapshot of a subvolume
    Snapshot {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
        /// Source subvolume path
        source: PathBuf,
        /// Destination snapshot path
        dest: PathBuf,
        /// Make the snapshot read-only
        #[arg(long)]
        readonly: bool,
        /// Create snapshot recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Set or unset the read-only flag on a subvolume
    SetReadonly {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
        /// Path to the subvolume
        path: PathBuf,
        /// Whether to set read-only (true) or writable (false)
        readonly: bool,
    },
    /// Set a subvolume as the default
    SetDefault {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
        /// Path to the subvolume
        path: PathBuf,
    },
    /// Get the default subvolume
    GetDefault {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
    },
    /// List deleted subvolumes pending cleanup
    ListDeleted {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
    },
    /// Get filesystem usage information
    Usage {
        /// Mount point of the BTRFS filesystem
        mount_point: PathBuf,
    },
}

fn main() -> Result<()> {
    // Initialize tracing to stderr
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::List { mount_point } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            let subvolumes = manager.list_all()?;
            let default_id = manager.get_default().unwrap_or(5);

            let output = SubvolumeList {
                subvolumes,
                default_id,
            };

            let json = serde_json::to_string(&output)?;
            println!("{}", json);
        }
        Commands::Create { mount_point, name } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            manager.create(&name)?;
            println!("{{\"success\": true}}");
        }
        Commands::Delete {
            mount_point,
            path,
            recursive,
        } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            manager.delete(&path, recursive)?;
            println!("{{\"success\": true}}");
        }
        Commands::Snapshot {
            mount_point,
            source,
            dest,
            readonly,
            recursive,
        } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            manager.snapshot(&source, &dest, readonly, recursive)?;
            println!("{{\"success\": true}}");
        }
        Commands::SetReadonly {
            mount_point,
            path,
            readonly,
        } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            manager.set_readonly(&path, readonly)?;
            println!("{{\"success\": true}}");
        }
        Commands::SetDefault { mount_point, path } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            manager.set_default(&path)?;
            println!("{{\"success\": true}}");
        }
        Commands::GetDefault { mount_point } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            let id = manager.get_default()?;
            println!("{{\"id\": {}}}", id);
        }
        Commands::ListDeleted { mount_point } => {
            let manager = SubvolumeManager::new(&mount_point)?;
            let deleted = manager.list_deleted()?;
            let json = serde_json::to_string(&deleted)?;
            println!("{}", json);
        }
        Commands::Usage { mount_point } => {
            let usage = get_filesystem_usage(&mount_point)?;
            let json = serde_json::to_string(&usage)?;
            println!("{}", json);
        }
    }

    Ok(())
}
