// SPDX-License-Identifier: GPL-3.0-only

//! Messages for network mount management

use storage_types::rclone::{ConfigScope, RemoteConfig};

/// Messages for network mount operations
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    /// Load all configured remotes
    LoadRemotes,
    /// Remotes loaded from service
    RemotesLoaded(Result<Vec<RemoteConfig>, String>),
    /// Select a remote in the sidebar
    SelectRemote { name: String, scope: ConfigScope },
    /// Start creating a new remote (opens wizard)
    BeginCreateRemote,
    /// Update remote name in editor
    EditorNameChanged(String),
    /// Update remote type in editor (by index)
    EditorTypeIndexChanged(usize),
    /// Update remote scope in editor
    /// Update a remote option field
    EditorFieldChanged { key: String, value: String },
    /// Update the new custom option key
    EditorNewOptionKeyChanged(String),
    /// Update the new custom option value
    EditorNewOptionValueChanged(String),
    /// Add a custom option to the editor
    EditorAddCustomOption,
    /// Remove a custom option from the editor
    EditorRemoveCustomOption { key: String },
    /// Toggle advanced option visibility
    EditorShowAdvanced(bool),
    /// Toggle hidden option visibility
    EditorShowHidden(bool),
    /// Toggle a section expander in the editor
    EditorToggleSection(String),
    /// Save remote configuration
    SaveRemote,
    /// Save completed
    SaveCompleted(Result<(), String>),

    // -- Wizard messages --
    /// User selected a provider type in the wizard grid
    WizardSelectType(String),
    /// User clicked "Advanced..." to switch to the full editor
    WizardAdvanced,
    /// Wizard name field changed
    WizardSetName(String),
    /// Wizard scope dropdown changed
    /// Wizard field changed (connection or auth step)
    WizardFieldChanged { key: String, value: String },
    /// Advance to next wizard step
    WizardNext,
    /// Go back to previous wizard step
    WizardBack,
    /// Create remote from wizard (final step)
    WizardCreate,
    /// Cancel and close the wizard
    WizardCancel,
    /// Wizard create completed (with name and scope on success)
    WizardCreateCompleted(Result<(String, ConfigScope), String>),
    /// Load mount-on-boot status for a remote
    LoadMountOnBoot { name: String, scope: ConfigScope },
    /// Mount-on-boot status loaded
    MountOnBootLoaded {
        name: String,
        scope: ConfigScope,
        result: Result<bool, String>,
    },
    /// Toggle mount-on-boot
    ToggleMountOnBoot(bool),
    /// Mount-on-boot updated
    MountOnBootUpdated {
        name: String,
        scope: ConfigScope,
        enabled: bool,
        previous: bool,
        result: Result<(), String>,
    },
    /// Open the mount path in file manager
    OpenMountPath(String),
    /// Mount a remote
    MountRemote { name: String, scope: ConfigScope },
    /// Unmount a remote
    UnmountRemote { name: String, scope: ConfigScope },
    /// Restart a remote (unmount then mount)
    RestartRemote { name: String, scope: ConfigScope },
    /// Mount operation completed
    MountCompleted {
        name: String,
        scope: ConfigScope,
        result: Result<(), String>,
    },
    /// Unmount operation completed
    UnmountCompleted {
        name: String,
        scope: ConfigScope,
        result: Result<(), String>,
    },
    /// Test remote configuration
    TestRemote { name: String, scope: ConfigScope },
    /// Test result received
    TestCompleted { result: Result<String, String> },
    /// Refresh mount status for a remote
    RefreshStatus { name: String, scope: ConfigScope },
    /// Status refreshed
    StatusRefreshed {
        name: String,
        scope: ConfigScope,
        mounted: bool,
    },
    /// Delete remote (with confirmation)
    DeleteRemote { name: String, scope: ConfigScope },
    /// Confirm delete remote
    ConfirmDeleteRemote { name: String, scope: ConfigScope },
    /// Delete completed
    DeleteCompleted {
        name: String,
        result: Result<(), String>,
    },
}
