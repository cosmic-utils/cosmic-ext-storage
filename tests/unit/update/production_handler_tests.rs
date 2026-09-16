//! Exercise the actual message -> operation -> completion path, without iced's
//! window runtime. Unsupported runtime actions fail instead of being simulated.
use super::*;
use crate::message::{dialogs::CreateMessage, volumes::VolumesControlMessage};
use crate::models::load::load_all_drives_with_operations;
use crate::state::dialogs::{CreatePartitionDialog, CreatePartitionStep};
use futures_util::StreamExt;
use rstest::{fixture, rstest};
use storage_types::CreatePartitionInfo;

#[path = "production_logical_tests.rs"]
mod logical;

#[path = "production_encryption_tests.rs"]
mod encryption;

#[path = "production_mount_tests.rs"]
mod mount;

#[path = "production_network_tests.rs"]
mod network;

#[path = "production_image_usage_tests.rs"]
mod image_usage;

#[path = "production_reload_tests.rs"]
mod reload;

#[path = "production_volume_dialog_tests.rs"]
mod volume_dialogs;

async fn outputs(task: Task<Message>) -> Vec<Message> {
    let Some(mut stream) = cosmic::iced::runtime::task::into_stream(task) else {
        return Vec::new();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async move {
        let mut messages = Vec::new();
        while let Some(action) = stream.next().await {
            match action {
                cosmic::iced::runtime::Action::Output(cosmic::Action::App(message)) => {
                    messages.push(message)
                }
                _ => panic!("production logic requested a desktop/runtime action"),
            }
            assert!(messages.len() < 32, "unexpected unbounded message stream");
        }
        messages
    })
    .await
    .expect("bounded production operation")
}

async fn settle(app: &mut AppModel, task: Task<Message>) {
    let mut pending = std::collections::VecDeque::from(outputs(task).await);
    let mut delivered = 0;
    while let Some(message) = pending.pop_front() {
        delivered += 1;
        assert!(delivered < 32, "unexpected completion loop");
        pending.extend(outputs(update(app, message)).await);
    }
}

fn create(app: &mut AppModel, message: CreateMessage) -> Task<Message> {
    update(
        app,
        Message::VolumesMessage(VolumesControlMessage::CreateMessage(message)),
    )
}

#[fixture]
async fn physical_app() -> AppModel {
    let runtime = crate::AppRuntime::scenario(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/scenarios/physical/partition-format.toml"),
        None,
        None,
    )
    .unwrap();
    let drives = load_all_drives_with_operations(runtime.operations())
        .await
        .unwrap();
    let mut app = AppModel::for_handler_test(runtime);
    app.filesystem_tools =
        crate::operations::FilesystemsClient::with_operations(app.runtime.operations())
            .get_filesystem_tools()
            .await
            .unwrap();
    assert!(
        outputs(update(&mut app, Message::UpdateNav(drives, None)))
            .await
            .is_empty()
    );
    app.dialog = Some(ShowDialog::AddPartition(CreatePartitionDialog {
        operation_id: None,
        info: CreatePartitionInfo {
            name: "Scenario data".into(),
            size: 268_435_456,
            max_size: 536_870_912,
            offset: 1_048_576,
            selected_type: "8300".into(),
            table_type: "gpt".into(),
            selected_partition_type_index: storage_types::COMMON_GPT_TYPES
                .iter()
                .position(|kind| kind.filesystem_type == "ext4")
                .unwrap(),
            ..Default::default()
        },
        step: CreatePartitionStep::Options,
        running: false,
        error: None,
        filesystem_tools: app.filesystem_tools.clone(),
    }));
    app
}

fn partition_dialog(app: &mut AppModel) -> &mut CreatePartitionDialog {
    let Some(ShowDialog::AddPartition(state)) = &mut app.dialog else {
        panic!("create dialog expected")
    };
    state
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn create_routes_to_selected_runtime_and_refreshes_actual_models(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let task = create(app, CreateMessage::Partition);
    assert!(partition_dialog(app).running);
    assert!(
        outputs(create(app, CreateMessage::Partition))
            .await
            .is_empty(),
        "duplicate submission"
    );
    settle(app, task).await;
    assert!(app.dialog.is_none(), "success closes the running dialog");
    let volumes = app.nav.active_data::<VolumesControl>().unwrap();
    assert_eq!(volumes.partitions.len(), 1);
    assert_eq!(
        volumes.partitions[0].filesystem_type.as_deref(),
        Some("ext4")
    );
    assert_eq!(volumes.partitions[0].size, 268_435_456);
    let diagnostics = app
        .runtime
        .scenario_control()
        .unwrap()
        .diagnostics()
        .await
        .unwrap();
    assert_eq!(
        diagnostics.generation, 2,
        "one create and one format; no duplicate effects"
    );
}

#[rstest]
#[case::cancel(CreateMessage::Cancel)]
#[case::no_dialog(CreateMessage::Partition)]
#[tokio::test(flavor = "current_thread")]
async fn create_cancel_and_missing_dialog_have_no_effect(
    #[future(awt)] mut physical_app: AppModel,
    #[case] message: CreateMessage,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    if matches!(message, CreateMessage::Partition) {
        physical_app.dialog = None;
    }
    let task = create(&mut physical_app, message);
    settle(&mut physical_app, task).await;
    assert!(physical_app.dialog.is_none());
    assert_eq!(
        physical_app
            .runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        0
    );
}

#[rstest]
#[case::zero_size(0, true, false, "", "")]
#[case::oversized(536_870_913, true, false, "", "")]
#[case::missing_tool(1, false, false, "", "")]
#[case::missing_secret(1, true, true, "", "")]
#[case::mismatched_secret(1, true, true, "first", "second")]
#[tokio::test(flavor = "current_thread")]
async fn create_submission_revalidates_form_without_side_effects(
    #[future(awt)] mut physical_app: AppModel,
    #[case] size: u64,
    #[case] available: bool,
    #[case] encrypted: bool,
    #[case] password: &str,
    #[case] confirmation: &str,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let state = partition_dialog(&mut physical_app);
    state.info.size = size;
    state.filesystem_tools[0].available = available;
    state.info.password_protected = encrypted;
    state.info.password = password.into();
    state.info.confirmed_password = confirmation.into();
    assert!(
        outputs(create(&mut physical_app, CreateMessage::Partition))
            .await
            .is_empty()
    );
    assert!(!partition_dialog(&mut physical_app).running);
    assert_eq!(
        physical_app
            .runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        0
    );
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn create_failure_surfaces_error_without_refreshing_navigation(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    physical_app
        .nav
        .active_data_mut::<VolumesControl>()
        .unwrap()
        .device = "/dev/missing-scenario-disk".into();
    let task = create(&mut physical_app, CreateMessage::Partition);
    settle(&mut physical_app, task).await;
    let Some(ShowDialog::Info { body, .. }) = &physical_app.dialog else {
        panic!("failure must surface an error")
    };
    assert!(body.contains("Failed to create partition"));
    assert!(
        physical_app
            .nav
            .active_data::<VolumesControl>()
            .unwrap()
            .partitions
            .is_empty()
    );
    assert_eq!(
        physical_app
            .runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        0
    );
}

#[rstest]
#[case::success(true, true)]
#[case::missing_device(false, true)]
#[case::missing_tool(true, false)]
#[tokio::test(flavor = "current_thread")]
async fn format_uses_selected_runtime_and_handles_invalid_targets(
    #[future(awt)] mut physical_app: AppModel,
    #[case] device_present: bool,
    #[case] tool_available: bool,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let info = partition_dialog(app).info.clone();
    let task = create(app, CreateMessage::Partition);
    settle(app, task).await;
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    let mut volume =
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1")
            .expect("created volume")
            .volume
            .clone();
    if !device_present {
        volume.device_path = None;
    }
    let mut tools = app.filesystem_tools.clone();
    tools[0].available = tool_available;
    app.dialog = Some(ShowDialog::FormatPartition(
        crate::state::dialogs::FormatPartitionDialog {
            operation_id: None,
            volume,
            info,
            step: crate::state::dialogs::FormatPartitionStep::Options,
            running: false,
            filesystem_tools: tools,
        },
    ));
    assert!(
        outputs(create(app, CreateMessage::NameUpdate("Replacement".into())))
            .await
            .is_empty()
    );
    let task = create(app, CreateMessage::Partition);
    if tool_available {
        assert!(
            outputs(create(app, CreateMessage::Partition))
                .await
                .is_empty(),
            "duplicate format"
        );
    }
    settle(app, task).await;
    let diagnostics = app
        .runtime
        .scenario_control()
        .unwrap()
        .diagnostics()
        .await
        .unwrap();
    if device_present && tool_available {
        assert!(app.dialog.is_none());
        assert_eq!(
            diagnostics.generation, 3,
            "create, initial format, replacement format"
        );
        let filesystems = app
            .runtime
            .operations()
            .registry
            .block
            .list_filesystems()
            .await
            .unwrap();
        assert_eq!(filesystems.len(), 1);
        assert_eq!(filesystems[0].label, "Replacement");
    } else {
        assert_eq!(
            diagnostics.generation, 2,
            "rejected format has no backend effect"
        );
        if tool_available {
            assert!(matches!(app.dialog, Some(ShowDialog::Info { .. })));
        } else {
            assert!(
                matches!(app.dialog, Some(ShowDialog::FormatPartition(ref state)) if !state.running)
            );
        }
    }
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn create_completion_does_not_close_a_replacement_idle_dialog(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let task = create(app, CreateMessage::Partition);
    let completions = outputs(task).await;
    app.dialog = Some(ShowDialog::Info {
        title: "Replacement".into(),
        body: "Keep open".into(),
    });
    for message in completions {
        let task = update(app, message);
        settle(app, task).await;
    }
    assert!(
        matches!(app.dialog, Some(ShowDialog::Info { ref title, .. }) if title == "Replacement")
    );
    assert!(
        app.nav
            .active_data::<VolumesControl>()
            .unwrap()
            .partitions
            .is_empty(),
        "cancelled operation does not apply an old model snapshot"
    );
}

#[rstest]
#[case::success(false)]
#[case::failure(true)]
#[tokio::test(flavor = "current_thread")]
async fn old_partition_completion_cannot_replace_a_new_running_dialog(
    #[future(awt)] mut physical_app: AppModel,
    #[case] failing: bool,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let replacement = partition_dialog(app).clone();
    if failing {
        app.nav.active_data_mut::<VolumesControl>().unwrap().device =
            "/dev/missing-scenario-disk".into();
    }
    let completions = outputs(create(app, CreateMessage::Partition)).await;
    assert_eq!(completions.len(), 1);
    app.nav.active_data_mut::<VolumesControl>().unwrap().device = "/dev/ui-disk0".into();
    app.dialog = Some(ShowDialog::AddPartition(replacement));
    let newer_task = create(app, CreateMessage::Partition);
    let newer_id = partition_dialog(app).operation_id;
    for completion in completions {
        assert!(outputs(update(app, completion)).await.is_empty());
    }
    assert!(partition_dialog(app).running);
    assert_eq!(partition_dialog(app).operation_id, newer_id);
    assert!(
        app.nav
            .active_data::<VolumesControl>()
            .unwrap()
            .partitions
            .is_empty()
    );
    let mut completions = outputs(newer_task).await;
    let duplicate = completions[0].clone();
    settle(app, update_task(completions.remove(0))).await;
    assert!(app.dialog.is_none());
    assert!(
        outputs(update(app, duplicate)).await.is_empty(),
        "duplicate completion ignored"
    );
    assert_eq!(
        app.nav
            .active_data::<VolumesControl>()
            .unwrap()
            .partitions
            .len(),
        if failing { 1 } else { 2 }
    );
}

fn update_task(message: Message) -> Task<Message> {
    Task::done(message.into())
}
