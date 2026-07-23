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
            std::any::TypeId::of::<DiskEventSubscription>(),
            |_: &std::any::TypeId| {
                cosmic::iced::stream::channel::<Message>(
                    4,
                    move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
                        let Ok(operations) = crate::operations::shared().await else {
                            return;
                        };
                        let Ok(mut events) = operations.registry.block.device_events().await else {
                            return;
                        };
                        while let Some(Ok(event)) = events.next().await {
                            let message = match event {
                                DeviceEvent::Added(device) => Message::DriveAdded(device),
                                DeviceEvent::Removed(device) => Message::DriveRemoved(device),
                            };
                            _ = output.send(message).await;
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

    // When an image operation is running, poll progress and wait for operation_completed.
    if let Some(ref operation_id) = app.image_op_operation_id {
        let operation_id = operation_id.clone();
        subs.push(Subscription::run_with(
            (std::any::TypeId::of::<ImageOperationSubscription>(), operation_id),
            |(_, operation_id): &(std::any::TypeId, String)| cosmic::iced::stream::channel::<Message>(32, {
                let operation_id = operation_id.clone();
                move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
                    let Ok(client) = ImageClient::new().await else {
                        _ = output
                            .send(Message::ImageOperationDialog(
                                ImageOperationDialogMessage::Complete(Err(
                                    "Failed to create image client".to_string(),
                                )),
                            ))
                            .await;
                        return;
                    };
                    loop {
                        tokio::select! {
                            result = client.wait_for_operation_completion(&operation_id) => {
                                let result = result.map_err(|e| e.to_string());
                                _ = output
                                    .send(Message::ImageOperationDialog(
                                        ImageOperationDialogMessage::Complete(result),
                                    ))
                                    .await;
                                return;
                            }
                            _ = tokio::time::sleep(Duration::from_millis(400)) => {
                                if let Ok(status) = client.get_operation_status(&operation_id).await
                                {
                                    _ = output
                                        .send(Message::ImageOperationDialog(
                                            ImageOperationDialogMessage::Progress(
                                                operation_id.clone(),
                                                status.bytes_completed,
                                                status.total_bytes,
                                                status.speed_bytes_per_sec,
                                            ),
                                        ))
                                        .await;
                                }
                            }
                        }
                    }
                }
            }),
        ));
    }

    Subscription::batch(subs)
}
