use crate::config::Config;
use crate::message::app::Message;
use crate::message::dialogs::ImageOperationDialogMessage;
use crate::operations::ImageClient;
use cosmic::Application;
use cosmic::iced::Subscription;
use cosmic::iced::futures::{SinkExt, StreamExt};
use cosmic::iced::{Event, event, keyboard};
use std::time::Duration;
use storage_types::DeviceEvent;

use crate::state::app::AppModel;

/// Subscription for image operation progress and completion.
struct ImageOperationSubscription;

/// Register subscriptions for this application.
///
/// Subscriptions are long-running async tasks running in the background which
/// emit messages to the application through a channel.
pub(crate) fn subscription(app: &AppModel) -> Subscription<Message> {
    struct DiskEventSubscription;

    let mut subs: Vec<Subscription<Message>> = vec![
        event::listen_with(|event, _, _| match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                Some(Message::UsageSelectionModifiersChanged(modifiers))
            }
            _ => None,
        }),
        // Disk hotplug comes directly from the block backend rather than a
        // project-owned D-Bus signal protocol.
        Subscription::run_with(
            (
                std::any::TypeId::of::<DiskEventSubscription>(),
                OperationContext(app.runtime.operations()),
            ),
            |(_, operations): &(std::any::TypeId, OperationContext)| {
                let operations = operations.0.clone();
                cosmic::iced::stream::channel::<Message>(
                    4,
                    move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
                        let Ok(mut events) = device_messages(operations).await else {
                            return;
                        };
                        while let Some(event) = events.next().await {
                            match event {
                                Ok(message) => {
                                    if output.send(message).await.is_err() {
                                        break;
                                    }
                                }
                                Err(error) => tracing::warn!(%error, "Device event stream failed"),
                            }
                        }
                    },
                )
            },
        ),
        // Watch for application configuration changes.
        app.core
            .watch_config::<Config>(<AppModel as Application>::APP_ID)
            .map(|update| Message::UpdateConfig(update.config)),
    ];

    // The subscription and headless tests use the same status-to-message adapter.
    if let Some(operation_id) = &app.image_op_operation_id {
        subs.push(Subscription::run_with(
            (
                std::any::TypeId::of::<ImageOperationSubscription>(),
                operation_id.clone(),
                OperationContext(app.runtime.operations()),
            ),
            |(_, operation_id, operations): &(std::any::TypeId, String, OperationContext)| {
                let operation_id = operation_id.clone();
                let client = ImageClient::with_operations(operations.0.clone());
                cosmic::iced::stream::channel::<Message>(32, move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
                    loop {
                        let (message, terminal) =
                            image_status_message(&client, &operation_id).await;
                        if output.send(message).await.is_err() || terminal {
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(400)).await;
                    }
                })
            },
        ));
    }

    Subscription::batch(subs)
}

#[derive(Clone)]
struct OperationContext(std::sync::Arc<crate::operations::StorageOperations>);

impl std::hash::Hash for OperationContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(std::sync::Arc::as_ptr(&self.0), state);
    }
}

/// Convert one actual adapter status to a production message. No window or
/// scheduler is needed to test progress, terminal state, errors or stale IDs.
pub(crate) async fn image_status_message(
    client: &ImageClient,
    operation_id: &str,
) -> (Message, bool) {
    use storage_types::WorkflowState;
    let result = match client.workflow_status(operation_id).await {
        Ok(status) => match status.state {
            WorkflowState::Pending | WorkflowState::Running => {
                return (
                    Message::ImageOperationDialog(ImageOperationDialogMessage::Progress(
                        operation_id.into(),
                        status.bytes_completed,
                        status.bytes_total,
                        status.speed_bytes_per_sec,
                    )),
                    false,
                );
            }
            WorkflowState::Completed => Ok(()),
            WorkflowState::Cancelled => Err("Operation cancelled".into()),
            WorkflowState::Failed => Err(status
                .message
                .unwrap_or_else(|| "Image operation failed".into())),
        },
        Err(error) => Err(error.to_string()),
    };
    (
        Message::ImageOperationDialog(ImageOperationDialogMessage::Complete {
            operation_id: operation_id.into(),
            result,
        }),
        true,
    )
}

type DeviceMessages = std::pin::Pin<
    Box<dyn futures_util::Stream<Item = Result<Message, storage_contracts::StorageError>> + Send>,
>;

pub(crate) async fn device_messages(
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Result<DeviceMessages, storage_contracts::StorageError> {
    Ok(Box::pin(
        operations
            .registry
            .block
            .device_events()
            .await?
            .map(|event| {
                event.map(|event| match event {
                    DeviceEvent::Added(device) => Message::DriveAdded(device),
                    DeviceEvent::Removed(device) => Message::DriveRemoved(device),
                    DeviceEvent::Refresh => Message::LoadDrivesIncremental,
                })
            }),
    ))
}
