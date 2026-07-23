// SPDX-License-Identifier: GPL-3.0-only

use storage_types::rclone::ConfigScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderBrandIcon {
    GoogleDrive,
    Dropbox,
    OneDrive,
    AmazonS3,
    Backblaze,
    ProtonDrive,
}

impl ProviderBrandIcon {
    pub(crate) fn svg_bytes(self) -> &'static [u8] {
        match self {
            Self::GoogleDrive => {
                include_bytes!("../../resources/icons/providers/googledrive.svg")
            }
            Self::Dropbox => include_bytes!("../../resources/icons/providers/dropbox.svg"),
            Self::OneDrive => include_bytes!("../../resources/icons/providers/onedrive.svg"),
            Self::AmazonS3 => include_bytes!("../../resources/icons/providers/s3.svg"),
            Self::Backblaze => include_bytes!("../../resources/icons/providers/backblaze.svg"),
            Self::ProtonDrive => {
                include_bytes!("../../resources/icons/providers/protondrive.svg")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderIcon {
    pub(crate) branded: Option<ProviderBrandIcon>,
    pub(crate) fallback_symbolic: &'static str,
    pub(crate) text_fallback: Option<&'static str>,
}

pub(crate) fn resolve_provider_icon(provider: &str) -> ProviderIcon {
    let provider = provider.trim().to_ascii_lowercase();

    match provider.as_str() {
        "drive" => ProviderIcon {
            branded: Some(ProviderBrandIcon::GoogleDrive),
            fallback_symbolic: "folder-remote-symbolic",
            text_fallback: None,
        },
        "dropbox" => ProviderIcon {
            branded: Some(ProviderBrandIcon::Dropbox),
            fallback_symbolic: "folder-remote-symbolic",
            text_fallback: None,
        },
        "onedrive" => ProviderIcon {
            branded: Some(ProviderBrandIcon::OneDrive),
            fallback_symbolic: "folder-remote-symbolic",
            text_fallback: None,
        },
        "s3" => ProviderIcon {
            branded: Some(ProviderBrandIcon::AmazonS3),
            fallback_symbolic: "network-server-symbolic",
            text_fallback: None,
        },
        "b2" => ProviderIcon {
            branded: Some(ProviderBrandIcon::Backblaze),
            fallback_symbolic: "network-server-symbolic",
            text_fallback: None,
        },
        "protondrive" => ProviderIcon {
            branded: Some(ProviderBrandIcon::ProtonDrive),
            fallback_symbolic: "network-server-symbolic",
            text_fallback: None,
        },
        "smb" | "cifs" => ProviderIcon {
            branded: None,
            fallback_symbolic: "network-server-symbolic",
            text_fallback: Some("SMB"),
        },
        "ssh" => ProviderIcon {
            branded: None,
            fallback_symbolic: "network-server-symbolic",
            text_fallback: Some("SSH"),
        },
        "sftp" => ProviderIcon {
            branded: None,
            fallback_symbolic: "network-server-symbolic",
            text_fallback: Some("SFTP"),
        },
        "ftp" => ProviderIcon {
            branded: None,
            fallback_symbolic: "network-server-symbolic",
            text_fallback: Some("FTP"),
        },
        "webdav" => ProviderIcon {
            branded: None,
            fallback_symbolic: "folder-remote-symbolic",
            text_fallback: Some("DAV"),
        },
        _ => ProviderIcon {
            branded: None,
            fallback_symbolic: "folder-remote-symbolic",
            text_fallback: None,
        },
    }
}

pub(crate) fn scope_icon(scope: ConfigScope) -> &'static str {
    match scope {
        ConfigScope::User => "user-home-symbolic",
        ConfigScope::System => "computer-symbolic",
    }
}

pub(crate) fn scope_label(scope: ConfigScope) -> &'static str {
    match scope {
        ConfigScope::User => "User",
        ConfigScope::System => "System",
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_provider_icon;

    #[test]
    fn known_provider_has_primary_with_fallback() {
        let icon = resolve_provider_icon("dropbox");
        assert!(icon.branded.is_some());
        assert_eq!(icon.fallback_symbolic, "folder-remote-symbolic");
    }

    #[test]
    fn unknown_provider_uses_generic_fallback() {
        let icon = resolve_provider_icon("unknown-provider");
        assert!(icon.branded.is_none());
        assert_eq!(icon.fallback_symbolic, "folder-remote-symbolic");
    }
}
