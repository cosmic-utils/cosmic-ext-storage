use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigScope {
    #[default]
    User,
    System,
}

impl ConfigScope {
    pub fn config_path(&self) -> PathBuf {
        match self {
            ConfigScope::User => {
                if let Some(home) = std::env::var_os("HOME") {
                    PathBuf::from(home).join(".config/rclone/rclone.conf")
                } else {
                    PathBuf::from("~/.config/rclone/rclone.conf")
                }
            }
            ConfigScope::System => PathBuf::from("/etc/rclone.conf"),
        }
    }

    pub fn mount_prefix(&self) -> PathBuf {
        match self {
            ConfigScope::User => {
                if let Some(home) = std::env::var_os("HOME") {
                    PathBuf::from(home).join("mnt")
                } else {
                    PathBuf::from("~/mnt")
                }
            }
            ConfigScope::System => PathBuf::from("/mnt/rclone"),
        }
    }

    pub fn mount_point(&self, remote_name: &str) -> PathBuf {
        self.mount_prefix().join(remote_name)
    }
}

impl std::fmt::Display for ConfigScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigScope::User => write!(f, "user"),
            ConfigScope::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for ConfigScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(ConfigScope::User),
            "system" => Ok(ConfigScope::System),
            _ => Err(format!("Invalid scope: {}", s)),
        }
    }
}
