//! App-level state types (extracted from `crate::app`).

use crate::config::Config;
use crate::fl;
use crate::message::app::Message;
use crate::runtime::AppRuntime;
use crate::state::dialogs::ShowDialog;
use crate::state::logical::LogicalState;
use crate::state::network::NetworkState;
use crate::state::sidebar::SidebarState;
#[cfg(feature = "test-backend")]
use crate::workflows::ApplicationWorkflowState;
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
    /// The explicitly selected storage composition for this application run.
    #[allow(dead_code)]
    pub(crate) runtime: AppRuntime,
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
    pub(crate) logical: LogicalState,
    /// Reducer state for deterministic application workflows. It contains no
    /// widget, renderer, task, or backend object.
    #[cfg(feature = "test-backend")]
    pub(crate) workflows: ApplicationWorkflowState,
}

impl AppModel {
    #[cfg(feature = "test-backend")]
    pub(crate) fn for_workflow_test(runtime: AppRuntime) -> Self {
        Self {
            runtime,
            core: Core::default(),
            context_page: ContextPage::default(),
            nav: nav_bar::Model::default(),
            sidebar: SidebarState::default(),
            config: Config::default(),
            image_op_operation_id: None,
            dialog: None,
            filesystem_tools: Vec::new(),
            network: NetworkState::new(),
            logical: LogicalState::default(),
            workflows: ApplicationWorkflowState::default(),
        }
    }

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

    #[cfg(feature = "test-backend")]
    pub(crate) fn reduce_logical_workflow(
        &mut self,
        intent: crate::workflows::logical::LogicalIntent,
    ) -> Vec<crate::workflows::logical::Effect> {
        crate::workflows::logical::reduce_intent(&mut self.workflows.logical, intent)
    }

    #[cfg(feature = "test-backend")]
    pub(crate) fn reduce_physical_workflow(
        &mut self,
        intent: crate::workflows::physical::PhysicalIntent,
        generation: u64,
    ) -> Vec<crate::workflows::physical::Effect> {
        crate::workflows::physical::reduce_intent(&mut self.workflows.physical, intent, generation)
    }

    #[cfg(feature = "test-backend")]
    pub(crate) fn reduce_network_workflow(
        &mut self,
        intent: crate::workflows::network::NetworkIntent,
        generation: u64,
    ) -> Vec<crate::workflows::network::Effect> {
        crate::workflows::network::reduce_intent(&mut self.workflows.network, intent, generation)
    }

    #[cfg(feature = "test-backend")]
    pub(crate) fn reduce_image_usage_workflow(
        &mut self,
        intent: crate::workflows::image_usage::ImageUsageIntent,
        generation: u64,
    ) -> Vec<crate::workflows::image_usage::Effect> {
        crate::workflows::image_usage::reduce_intent(
            &mut self.workflows.image_usage,
            intent,
            generation,
        )
    }

    #[cfg(feature = "test-backend")]
    pub(crate) fn reduce_reload_workflow(
        &mut self,
        intent: crate::workflows::reload::ReloadIntent,
        generation: u64,
    ) -> Vec<crate::workflows::reload::Effect> {
        crate::workflows::reload::reduce_intent(&mut self.workflows.reload, intent, generation)
    }
}
