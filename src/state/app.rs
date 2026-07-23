//! App-level state types (extracted from `crate::app`).

use crate::config::Config;
use crate::fl;
use crate::message::app::Message;
use crate::state::dialogs::ShowDialog;
use crate::state::network::NetworkState;
use crate::state::sidebar::SidebarState;
use cosmic::ApplicationExt;
use cosmic::app::{Core, Task};
use cosmic::widget::nav_bar;
use storage_types::FilesystemToolInfo;

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    Settings,
}

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    pub(crate) core: Core,
    /// Display a context drawer with the designated page if defined.
    pub(crate) context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    pub(crate) nav: nav_bar::Model,

    /// Custom sidebar treeview state (rendered instead of the built-in nav_bar widget).
    pub(crate) sidebar: SidebarState,
    // Configuration data that persists between application runs.
    pub(crate) config: Config,

    /// Active image operation id (for progress subscription and cancel).
    pub(crate) image_op_operation_id: Option<String>,

    pub dialog: Option<ShowDialog>,

    /// Cached filesystem tool availability from service
    pub(crate) filesystem_tools: Vec<FilesystemToolInfo>,

    /// Network mounts state (RClone, Samba, FTP)
    pub(crate) network: NetworkState,
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<Message> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}
