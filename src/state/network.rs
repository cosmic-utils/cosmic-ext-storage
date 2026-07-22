// SPDX-License-Identifier: GPL-3.0-only

//! State for network mount management

use std::collections::{HashMap, HashSet};
use storage_types::rclone::{ConfigScope, MountStatus, RemoteConfig};

/// Runtime state of a network mount
#[derive(Debug, Clone)]
pub struct NetworkMountState {
    /// The remote configuration
    pub config: RemoteConfig,
    /// Current mount status
    pub status: MountStatus,
    /// Whether an operation is in progress
    pub loading: bool,
    /// Last error message if any
    pub error: Option<String>,
}

impl NetworkMountState {
    pub fn new(config: RemoteConfig) -> Self {
        Self {
            config,
            status: MountStatus::Unmounted,
            loading: false,
            error: None,
        }
    }

    /// Check if this mount is currently mounted
    pub fn is_mounted(&self) -> bool {
        self.status.is_mounted()
    }
}

/// Wizard step for guided remote creation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardStep {
    /// Step 1: Pick provider type from grid
    SelectType,
    /// Step 2: Name the remote
    NameAndScope,
    /// Step 3: Connection settings (host, port, endpoint, etc.)
    Connection,
    /// Step 4: Authentication settings (user, pass, key, etc.)
    Authentication,
    /// Step 5: Review summary and create
    Review,
}

impl WizardStep {
    /// Get the step number (1-based)
    pub fn number(&self) -> usize {
        match self {
            WizardStep::SelectType => 1,
            WizardStep::NameAndScope => 2,
            WizardStep::Connection => 3,
            WizardStep::Authentication => 4,
            WizardStep::Review => 5,
        }
    }

    /// Get display label for the step
    pub fn label(&self) -> &'static str {
        match self {
            WizardStep::SelectType => "Select Type",
            WizardStep::NameAndScope => "Name",
            WizardStep::Connection => "Connection",
            WizardStep::Authentication => "Authentication",
            WizardStep::Review => "Review",
        }
    }
}

/// Quick-setup provider definition for the wizard type selection grid
pub struct QuickSetupProvider {
    /// rclone type name (e.g. "s3", "ftp")
    pub type_name: &'static str,
    /// Display label (e.g. "Amazon S3")
    pub label: &'static str,
    /// Short description
    pub description: &'static str,
}

/// List of providers shown in the wizard quick-setup grid
pub const QUICK_SETUP_PROVIDERS: &[QuickSetupProvider] = &[
    QuickSetupProvider {
        type_name: "s3",
        label: "Amazon S3",
        description: "S3 compatible storage",
    },
    QuickSetupProvider {
        type_name: "ftp",
        label: "FTP",
        description: "File Transfer Protocol",
    },
    QuickSetupProvider {
        type_name: "sftp",
        label: "SSH / SFTP",
        description: "Secure file transfer",
    },
    QuickSetupProvider {
        type_name: "smb",
        label: "SMB / CIFS",
        description: "Windows file sharing",
    },
    QuickSetupProvider {
        type_name: "drive",
        label: "Google Drive",
        description: "Cloud storage by Google",
    },
    QuickSetupProvider {
        type_name: "dropbox",
        label: "Dropbox",
        description: "Cloud storage by Dropbox",
    },
    QuickSetupProvider {
        type_name: "onedrive",
        label: "OneDrive",
        description: "Cloud storage by Microsoft",
    },
    QuickSetupProvider {
        type_name: "webdav",
        label: "WebDAV",
        description: "Web-based file access",
    },
    QuickSetupProvider {
        type_name: "b2",
        label: "Backblaze B2",
        description: "Cloud object storage",
    },
    QuickSetupProvider {
        type_name: "protondrive",
        label: "Proton Drive",
        description: "Encrypted cloud storage",
    },
];

/// State for the creation wizard
#[derive(Debug, Clone)]
pub struct NetworkWizardState {
    /// Current wizard step
    pub step: WizardStep,
    /// Selected remote type
    pub remote_type: String,
    /// Remote name
    pub name: String,
    /// Configuration scope
    pub scope: ConfigScope,
    /// Options filled in during wizard steps
    pub options: HashMap<String, String>,
    /// Error message if any
    pub error: Option<String>,
    /// Whether a save operation is in progress
    pub running: bool,
}

impl NetworkWizardState {
    /// Create a new wizard starting at the type selection step
    pub fn new() -> Self {
        Self {
            step: WizardStep::SelectType,
            remote_type: String::new(),
            name: String::new(),
            scope: ConfigScope::User,
            options: HashMap::new(),
            error: None,
            running: false,
        }
    }

    /// Check if we can advance to the next step
    pub fn can_advance(&self) -> bool {
        match self.step {
            WizardStep::SelectType => !self.remote_type.is_empty(),
            WizardStep::NameAndScope => {
                !self.name.is_empty()
                    && self
                        .name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            }
            WizardStep::Connection => true, // Connection fields are optional for some providers
            WizardStep::Authentication => true, // Auth fields are optional for OAuth providers
            WizardStep::Review => !self.running,
        }
    }

    /// Advance to the next step
    pub fn next_step(&mut self) {
        self.error = None;
        self.step = match self.step {
            WizardStep::SelectType => WizardStep::NameAndScope,
            WizardStep::NameAndScope => WizardStep::Connection,
            WizardStep::Connection => WizardStep::Authentication,
            WizardStep::Authentication => WizardStep::Review,
            WizardStep::Review => WizardStep::Review, // No next from review
        };
    }

    /// Go back to the previous step
    pub fn prev_step(&mut self) {
        self.error = None;
        self.step = match self.step {
            WizardStep::SelectType => WizardStep::SelectType, // No prev from first
            WizardStep::NameAndScope => WizardStep::SelectType,
            WizardStep::Connection => WizardStep::NameAndScope,
            WizardStep::Authentication => WizardStep::Connection,
            WizardStep::Review => WizardStep::Authentication,
        };
    }
}

/// Ordered list of sections for display
pub const SECTION_ORDER: &[&str] = &[
    "authentication",
    "connection",
    "security",
    "storage",
    "transfers",
    "behavior",
    "other",
];

/// Get display name for a section ID
pub fn section_display_name(section: &str) -> &'static str {
    match section {
        "authentication" => "Authentication",
        "connection" => "Connection",
        "security" => "Security",
        "storage" => "Storage",
        "transfers" => "Transfers",
        "behavior" => "Behavior",
        "other" => "Other",
        _ => "Other",
    }
}

/// State for the network section of the sidebar
#[derive(Debug, Default)]
pub struct NetworkState {
    /// All configured remotes with their state
    pub mounts: HashMap<(String, ConfigScope), NetworkMountState>,

    /// Currently selected remote (name, scope)
    pub selected: Option<(String, ConfigScope)>,

    /// Whether remotes are being loaded
    pub loading: bool,

    /// Whether RClone is available
    pub rclone_available: bool,

    /// Last error message
    pub error: Option<String>,

    /// Active editor state (for viewing/editing existing remotes)
    pub editor: Option<NetworkEditorState>,

    /// Active wizard state (for guided creation of new remotes)
    pub wizard: Option<NetworkWizardState>,
}

impl NetworkState {
    /// Create new network state
    pub fn new() -> Self {
        Self::default()
    }

    /// Start editing a remote configuration
    pub fn start_edit(&mut self, config: RemoteConfig) {
        self.editor = Some(NetworkEditorState::from_config(config));
    }

    /// Close the editor
    pub fn clear_editor(&mut self) {
        self.editor = None;
    }

    /// Start the creation wizard
    pub fn start_wizard(&mut self) {
        self.editor = None;
        self.wizard = Some(NetworkWizardState::new());
    }

    /// Close the wizard
    pub fn clear_wizard(&mut self) {
        self.wizard = None;
    }

    /// Convert wizard state to editor state (for "Advanced..." flow)
    pub fn wizard_to_editor(&mut self) {
        if let Some(wizard) = self.wizard.take() {
            let remote_type = if wizard.remote_type.is_empty() {
                storage_types::rclone::supported_remote_types()
                    .first()
                    .cloned()
                    .unwrap_or_default()
            } else {
                wizard.remote_type
            };
            let mut editor = NetworkEditorState::new(remote_type);
            editor.name = wizard.name;
            editor.scope = wizard.scope;
            editor.options = wizard.options;
            self.editor = Some(editor);
        }
    }

    /// Set remotes from loaded configuration
    pub fn set_remotes(&mut self, remotes: Vec<RemoteConfig>) {
        // Preserve existing mount status where possible
        let old_mounts = std::mem::take(&mut self.mounts);

        for config in remotes {
            let key = (config.name.clone(), config.scope);
            let state = if let Some(existing) = old_mounts.get(&key) {
                // Preserve status and loading state
                NetworkMountState {
                    config,
                    status: existing.status.clone(),
                    loading: existing.loading,
                    error: None,
                }
            } else {
                NetworkMountState::new(config)
            };
            self.mounts.insert(key, state);
        }
    }

    /// Get a mount by name and scope
    pub fn get_mount(&self, name: &str, scope: ConfigScope) -> Option<&NetworkMountState> {
        self.mounts.get(&(name.to_string(), scope))
    }

    /// Get a mutable mount by name and scope
    #[allow(dead_code)]
    pub fn get_mount_mut(
        &mut self,
        name: &str,
        scope: ConfigScope,
    ) -> Option<&mut NetworkMountState> {
        self.mounts.get_mut(&(name.to_string(), scope))
    }

    /// Update mount status
    pub fn set_mount_status(&mut self, name: &str, scope: ConfigScope, status: MountStatus) {
        if let Some(mount) = self.mounts.get_mut(&(name.to_string(), scope)) {
            mount.status = status;
            mount.loading = false;
        }
    }

    /// Set loading state for a mount
    pub fn set_loading(&mut self, name: &str, scope: ConfigScope, loading: bool) {
        if let Some(mount) = self.mounts.get_mut(&(name.to_string(), scope)) {
            mount.loading = loading;
        }
    }

    /// Set error for a mount
    pub fn set_error(&mut self, name: &str, scope: ConfigScope, error: Option<String>) {
        if let Some(mount) = self.mounts.get_mut(&(name.to_string(), scope)) {
            mount.error = error;
        }
    }

    /// Check if a remote is selected
    pub fn is_selected(&self, name: &str, scope: ConfigScope) -> bool {
        self.selected == Some((name.to_string(), scope))
    }

    /// Select a remote
    pub fn select(&mut self, name: Option<String>, scope: Option<ConfigScope>) {
        self.selected = name.zip(scope);
    }

    /// Get list of remotes sorted by name
    #[allow(dead_code)]
    pub fn sorted_mounts(&self) -> Vec<&NetworkMountState> {
        let mut mounts: Vec<_> = self.mounts.values().collect();
        mounts.sort_by(|a, b| {
            // Sort by scope first (User before System), then by name
            match (a.config.scope, b.config.scope) {
                (ConfigScope::User, ConfigScope::System) => std::cmp::Ordering::Less,
                (ConfigScope::System, ConfigScope::User) => std::cmp::Ordering::Greater,
                _ => a.config.name.cmp(&b.config.name),
            }
        });
        mounts
    }

    /// Get user-scope remotes only
    #[allow(dead_code)]
    pub fn user_mounts(&self) -> Vec<&NetworkMountState> {
        let mut mounts: Vec<_> = self
            .mounts
            .values()
            .filter(|m| m.config.scope == ConfigScope::User)
            .collect();
        mounts.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        mounts
    }

    /// Get system-scope remotes only
    #[allow(dead_code)]
    pub fn system_mounts(&self) -> Vec<&NetworkMountState> {
        let mut mounts: Vec<_> = self
            .mounts
            .values()
            .filter(|m| m.config.scope == ConfigScope::System)
            .collect();
        mounts.sort_by(|a, b| a.config.name.cmp(&b.config.name));
        mounts
    }
}

#[derive(Debug, Clone)]
pub struct NetworkEditorState {
    pub name: String,
    pub remote_type: String,
    pub scope: ConfigScope,
    pub options: HashMap<String, String>,
    pub original_name: Option<String>,
    pub original_scope: Option<ConfigScope>,
    pub is_new: bool,
    pub running: bool,
    pub error: Option<String>,
    pub new_option_key: String,
    pub new_option_value: String,
    pub show_advanced: bool,
    pub show_hidden: bool,
    pub mount_on_boot: Option<bool>,
    /// Which sections are currently expanded in the editor
    pub expanded_sections: HashSet<String>,
}

impl NetworkEditorState {
    /// Default sections to expand in a new editor
    fn default_expanded_sections() -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("authentication".to_string());
        set.insert("connection".to_string());
        set
    }

    pub fn new(default_type: String) -> Self {
        Self {
            name: String::new(),
            remote_type: default_type,
            scope: ConfigScope::User,
            options: HashMap::new(),
            original_name: None,
            original_scope: None,
            is_new: true,
            running: false,
            error: None,
            new_option_key: String::new(),
            new_option_value: String::new(),
            show_advanced: false,
            show_hidden: false,
            mount_on_boot: None,
            expanded_sections: Self::default_expanded_sections(),
        }
    }

    pub fn from_config(config: RemoteConfig) -> Self {
        Self {
            name: config.name.clone(),
            remote_type: config.remote_type.clone(),
            scope: config.scope,
            options: config.options.clone(),
            original_name: Some(config.name),
            original_scope: Some(config.scope),
            is_new: false,
            running: false,
            error: None,
            new_option_key: String::new(),
            new_option_value: String::new(),
            show_advanced: false,
            show_hidden: false,
            mount_on_boot: None,
            expanded_sections: Self::default_expanded_sections(),
        }
    }
}
