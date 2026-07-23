// SPDX-License-Identifier: GPL-3.0-only

pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

pub(crate) use crate::message::app::Message;
pub(crate) use crate::state::app::{AppModel, ContextPage};

use crate::config::Config;
use crate::models::load_all_drives;
use crate::operations::FilesystemsClient;
use crate::operations::RcloneClient;
use crate::state::network::NetworkState;
use crate::state::sidebar::SidebarState;
use cosmic::app::{Core, Task};
use cosmic::widget::nav_bar;
use cosmic::{Application, Element};

pub(crate) const APP_ID: &str = "com.cosmic.ext.Storage";

impl Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav: nav_bar::Model::default(),
            sidebar: SidebarState::default(),
            dialog: None,
            image_op_operation_id: None,
            filesystem_tools: vec![],
            network: NetworkState::new(),
            config: Config::load(Self::APP_ID),
        };

        let command = app.update_title();

        let nav_command = Task::perform(
            async {
                match load_all_drives().await {
                    Ok(drives) => Some(drives),
                    Err(e) => {
                        tracing::error!(%e, "failed to load drives");
                        None
                    }
                }
            },
            |drives| match drives {
                None => Message::None.into(),
                Some(drives) => Message::UpdateNav(drives, None).into(),
            },
        );

        let tools_command = Task::perform(
            async {
                match FilesystemsClient::new().await {
                    Ok(client) => match client.get_filesystem_tools().await {
                        Ok(tools) => Some(tools),
                        Err(e) => {
                            tracing::error!(%e, "failed to load filesystem tools");
                            None
                        }
                    },
                    Err(e) => {
                        tracing::error!(%e, "failed to create filesystems client");
                        None
                    }
                }
            },
            |tools| match tools {
                None => Message::None.into(),
                Some(tools) => Message::FilesystemToolsLoaded(tools).into(),
            },
        );

        let network_command = Task::perform(
            async {
                match RcloneClient::new().await {
                    Ok(client) => match client.list_remotes().await {
                        Ok(list) => Some(list.remotes),
                        Err(e) => {
                            tracing::warn!(%e, "failed to load network remotes");
                            None
                        }
                    },
                    Err(e) => {
                        tracing::info!(%e, "RClone client not available, network features disabled");
                        None
                    }
                }
            },
            |remotes| {
                Message::NetworkRemotesLoaded(
                    remotes.ok_or_else(|| "RClone not available".to_string()),
                )
                .into()
            },
        );

        (
            app,
            command
                .chain(nav_command)
                .chain(tools_command)
                .chain(network_command),
        )
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        crate::views::app::header_start(self)
    }

    fn header_center(&self) -> Vec<Element<'_, Self::Message>> {
        crate::views::app::header_center(self)
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        crate::views::app::header_end(self)
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        crate::views::app::dialog(self)
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Self::Message>>> {
        crate::views::app::nav_bar(self)
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        crate::views::app::nav_model(self)
    }

    fn context_drawer(
        &self,
    ) -> Option<cosmic::app::context_drawer::ContextDrawer<'_, Self::Message>> {
        crate::views::app::context_drawer(self)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        crate::views::app::view(self)
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        crate::subscriptions::app::subscription(self)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        crate::update::update(self, message)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        crate::update::on_nav_select(self, id)
    }
}
