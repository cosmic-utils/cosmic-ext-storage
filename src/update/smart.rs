use crate::message::dialogs::SmartDialogMessage;
use crate::operations::DisksClient;
use crate::state::dialogs::{ShowDialog, SmartDataDialog};
use cosmic::app::Task;

use crate::message::app::Message;
use crate::state::app::AppModel;

pub(super) fn smart_dialog(app: &mut AppModel, msg: SmartDialogMessage) -> Task<Message> {
    let Some(ShowDialog::SmartData(state)) = app.dialog.clone() else {
        return Task::none();
    };

    match msg {
        SmartDialogMessage::Close => {
            app.dialog = None;
        }
        SmartDialogMessage::Loaded(res) => {
            let mut next = state;
            next.running = false;
            match res {
                Ok(info) => {
                    next.info = Some(info);
                    next.error = None;
                }
                Err(e) => {
                    tracing::error!(%e, "SMART dialog error");
                    next.error = Some(e);
                }
            }
            app.dialog = Some(ShowDialog::SmartData(next));
        }
        SmartDialogMessage::Refresh => {
            let drive = state.drive.clone();
            let info = state.info.clone();
            app.dialog = Some(ShowDialog::SmartData(SmartDataDialog {
                drive: drive.clone(),
                running: true,
                info,
                error: None,
            }));

            return Task::perform(
                async move {
                    let disks_client = DisksClient::new()
                        .await
                        .map_err(|e| format!("Failed to create disks client: {}", e))?;
                    let status = disks_client
                        .get_smart_status(drive.device())
                        .await
                        .map_err(|e| format!("Failed to get SMART status: {}", e))?;
                    let attributes = disks_client
                        .get_smart_attributes(drive.device())
                        .await
                        .map_err(|e| format!("Failed to get SMART attributes: {}", e))?;
                    Ok((status, attributes))
                },
                |res| Message::SmartDialog(SmartDialogMessage::Loaded(res)).into(),
            );
        }
        SmartDialogMessage::SelfTestShort => {
            let drive = state.drive.clone();
            let info = state.info.clone();
            app.dialog = Some(ShowDialog::SmartData(SmartDataDialog {
                drive: drive.clone(),
                running: true,
                info,
                error: None,
            }));
            return Task::perform(
                async move {
                    DisksClient::new()
                        .await
                        .map_err(|e| format!("Failed to create disks client: {}", e))?
                        .start_smart_test(drive.device(), "short")
                        .await
                        .map_err(|e| format!("Failed to start SMART test: {}", e))
                },
                |res| Message::SmartDialog(SmartDialogMessage::ActionComplete(res)).into(),
            );
        }
        SmartDialogMessage::SelfTestExtended => {
            let drive = state.drive.clone();
            let info = state.info.clone();
            app.dialog = Some(ShowDialog::SmartData(SmartDataDialog {
                drive: drive.clone(),
                running: true,
                info,
                error: None,
            }));
            return Task::perform(
                async move {
                    DisksClient::new()
                        .await
                        .map_err(|e| format!("Failed to create disks client: {}", e))?
                        .start_smart_test(drive.device(), "long")
                        .await
                        .map_err(|e| format!("Failed to start SMART test: {}", e))
                },
                |res| Message::SmartDialog(SmartDialogMessage::ActionComplete(res)).into(),
            );
        }
        SmartDialogMessage::AbortSelfTest => {
            let drive = state.drive.clone();
            let info = state.info.clone();
            app.dialog = Some(ShowDialog::SmartData(SmartDataDialog {
                drive: drive.clone(),
                running: true,
                info,
                error: None,
            }));
            return Task::perform(
                async move {
                    DisksClient::new()
                        .await
                        .map_err(|e| format!("Failed to create disks client: {}", e))?
                        .start_smart_test(drive.device(), "abort")
                        .await
                        .map_err(|e| format!("Failed to abort SMART test: {}", e))
                },
                |res| Message::SmartDialog(SmartDialogMessage::ActionComplete(res)).into(),
            );
        }
        SmartDialogMessage::ActionComplete(res) => {
            let drive = state.drive.clone();
            let ok = res.is_ok();

            let mut next = state;
            next.running = false;
            next.error = res.err();
            if let Some(ref e) = next.error {
                tracing::error!(%e, "SMART dialog action error");
            }
            app.dialog = Some(ShowDialog::SmartData(next));

            // After a successful action, refresh SMART data.
            if ok {
                return Task::perform(
                    async move {
                        let disks_client = DisksClient::new()
                            .await
                            .map_err(|e| format!("Failed to create disks client: {}", e))?;
                        let status = disks_client
                            .get_smart_status(drive.device())
                            .await
                            .map_err(|e| format!("Failed to get SMART status: {}", e))?;
                        let attributes = disks_client
                            .get_smart_attributes(drive.device())
                            .await
                            .map_err(|e| format!("Failed to get SMART attributes: {}", e))?;
                        Ok((status, attributes))
                    },
                    |res| Message::SmartDialog(SmartDialogMessage::Loaded(res)).into(),
                );
            }
        }
    }

    Task::none()
}
