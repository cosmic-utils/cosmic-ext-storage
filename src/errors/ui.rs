// SPDX-License-Identifier: GPL-3.0-only

//! Error surfacing for the UI.
//!
//! Error boundary: use `anyhow::Error` inside `Task::perform` async closures so multiple
//! error sources combine cleanly; convert to UI (e.g. this module) in the completion closure.

use crate::app::Message;
use crate::state::dialogs::ShowDialog;

pub(crate) struct UiErrorContext<'a> {
    pub(crate) operation: &'static str,
    pub(crate) device_path: Option<&'a str>,
    pub(crate) device: Option<&'a str>,
    pub(crate) drive_path: Option<&'a str>,
}

impl<'a> UiErrorContext<'a> {
    pub(crate) fn new(operation: &'static str) -> Self {
        Self {
            operation,
            device_path: None,
            device: None,
            drive_path: None,
        }
    }
}

pub(crate) fn log_error_and_show_dialog(
    title: impl Into<String>,
    err: anyhow::Error,
    ctx: UiErrorContext<'_>,
) -> Message {
    tracing::error!(
        ?err,
        operation = ctx.operation,
        device_path = ?ctx.device_path,
        device = ?ctx.device,
        drive_path = ?ctx.drive_path,
        "error surfaced in UI"
    );

    Message::Dialog(Box::new(ShowDialog::Info {
        title: title.into(),
        body: format!("{err:#}"),
    }))
}
