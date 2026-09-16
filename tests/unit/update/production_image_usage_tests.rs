use super::*;
use crate::message::{app::ImagePathPickerKind, dialogs::ImageOperationDialogMessage as I};

fn usage(app: &mut AppModel) -> &mut crate::state::volumes::UsageTabState {
    &mut app
        .nav
        .active_data_mut::<VolumesControl>()
        .unwrap()
        .usage_state
}

#[rstest]
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn usage_running_adapter_waits_for_virtual_completion(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    usage(app).scan_mount_points = vec!["/mnt/ui-data".into()];
    let start = outputs(update(app, Message::UsageRefreshRequested)).await;
    let task = update(app, start.into_iter().next().unwrap());
    let pending = outputs(task);
    tokio::pin!(pending);
    assert!(
        futures_util::poll!(&mut pending).is_pending(),
        "running scan is not a terminal failure"
    );
    assert!(usage(app).loading);
    app.runtime
        .scenario_control()
        .unwrap()
        .advance_to(1000)
        .await
        .unwrap();
    tokio::time::advance(std::time::Duration::from_millis(100)).await;
    for message in pending.await {
        let task = update(app, message);
        settle(app, task).await;
    }
    assert_eq!(usage(app).result.as_ref().unwrap().total_bytes, 1000);
    assert!(!usage(app).loading);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn usage_unsupported_adapter_surfaces_error_without_success(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    usage(app).scan_mount_points = vec!["/mnt/unsupported".into()];
    let task = update(app, Message::UsageRefreshRequested);
    settle(app, task).await;
    assert!(!usage(app).loading);
    assert!(usage(app).active_scan_id.is_none());
    assert!(usage(app).error.is_some());
    assert!(usage(app).result.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn usage_wizard_scan_selection_delete_and_refresh_use_real_state(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    let task = update(app, Message::UsageConfigureRequested);
    settle(app, task).await;
    assert_eq!(usage(app).wizard_selected_mount_points, ["/mnt/ui-data"]);
    app.runtime
        .scenario_control()
        .unwrap()
        .advance_to(1000)
        .await
        .unwrap();
    let task = update(app, Message::UsageWizardStartScan);
    assert!(usage(app).loading);
    assert!(
        outputs(update(app, Message::UsageWizardStartScan))
            .await
            .is_empty()
    );
    settle(app, task).await;
    assert!(!usage(app).loading);
    assert_eq!(usage(app).result.as_ref().unwrap().total_bytes, 1000);
    let _ = update(
        app,
        Message::UsageSelectionSingle {
            path: "/mnt/ui-data/cache-file".into(),
            index: 0,
        },
    );
    let task = update(app, Message::UsageDeleteStart);
    assert!(usage(app).deleting);
    assert!(
        outputs(update(app, Message::UsageDeleteStart))
            .await
            .is_empty()
    );
    settle(app, task).await;
    assert!(!usage(app).deleting);
    assert_eq!(usage(app).result.as_ref().unwrap().total_bytes, 0);
    assert!(usage(app).selected_paths.is_empty());
    let task = update(app, Message::UsageRefreshRequested);
    settle(app, task).await;
    assert_eq!(usage(app).result.as_ref().unwrap().total_bytes, 0);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn usage_old_scan_and_delete_cannot_change_replacement_state(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    usage(app).scan_mount_points = vec!["/mnt/ui-data".into()];
    app.runtime
        .scenario_control()
        .unwrap()
        .advance_to(1000)
        .await
        .unwrap();
    let start = outputs(update(app, Message::UsageRefreshRequested)).await;
    let old_id = usage(app).active_scan_id.clone().unwrap();
    let completion = outputs(update(app, start.into_iter().next().unwrap())).await;
    // Replacing the actual volume model invalidates its outstanding operations.
    let drives = load_all_drives_with_operations(app.runtime.operations())
        .await
        .unwrap();
    let _ = update(app, Message::UpdateNav(drives, None));
    usage(app).scan_mount_points = vec!["/mnt/ui-data".into()];
    let current = update(app, Message::UsageRefreshRequested);
    let current_id = usage(app).active_scan_id.clone().unwrap();
    assert_ne!(current_id, old_id);
    for message in completion {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert_eq!(
        usage(app).active_scan_id.as_deref(),
        Some(current_id.as_str())
    );
    assert!(usage(app).result.is_none());
    settle(app, current).await;
    let _ = update(
        app,
        Message::UsageSelectionSingle {
            path: "/mnt/ui-data/cache-file".into(),
            index: 0,
        },
    );
    let completion = outputs(update(app, Message::UsageDeleteStart)).await;
    let drives = load_all_drives_with_operations(app.runtime.operations())
        .await
        .unwrap();
    let _ = update(app, Message::UpdateNav(drives, None));
    usage(app).operation_status = Some("replacement".into());
    for message in completion {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert_eq!(usage(app).operation_status.as_deref(), Some("replacement"));
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn usage_cancelled_wizard_and_invalid_selection_never_start(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    let task = update(app, Message::UsageConfigureRequested);
    let _ = update(app, Message::UsageWizardCancel);
    settle(app, task).await;
    assert!(!usage(app).wizard_open);
    assert!(
        outputs(update(app, Message::UsageWizardStartScan))
            .await
            .is_empty()
    );
    let task = update(app, Message::UsageConfigureRequested);
    settle(app, task).await;
    let _ = update(
        app,
        Message::UsageWizardMountToggled {
            mount_point: "/mnt/ui-data".into(),
            selected: false,
        },
    );
    assert!(
        outputs(update(app, Message::UsageWizardStartScan))
            .await
            .is_empty()
    );
    assert!(usage(app).wizard_error.is_some());
    assert!(
        outputs(update(app, Message::UsageDeleteStart))
            .await
            .is_empty()
    );
    assert!(!usage(app).deleting);
}

async fn poll_image(app: &mut AppModel, operation_id: &str) -> bool {
    let client = crate::operations::ImageClient::with_operations(app.runtime.operations());
    let (message, terminal) =
        crate::subscriptions::app::image_status_message(&client, operation_id).await;
    let task = update(app, message);
    settle(app, task).await;
    terminal
}

#[rstest]
#[case::complete(false)]
#[case::cancel(true)]
#[tokio::test(flavor = "current_thread")]
async fn image_progress_terminal_and_cleanup_use_actual_subscription_messages(
    #[future(awt)] mut workflow_app: AppModel,
    #[case] cancel: bool,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    open_image(app).await;
    let task = image(app, I::Start);
    settle(app, task).await;
    let id = image_state(app).operation_id.clone().unwrap();
    app.runtime
        .scenario_control()
        .unwrap()
        .advance_to(250)
        .await
        .unwrap();
    assert!(!poll_image(app, &id).await);
    assert_eq!(
        image_state(app).progress.as_ref().map(|p| (p.0, p.1)),
        Some((250, 1000))
    );
    if cancel {
        let task = image(app, I::CancelOperation);
        settle(app, task).await;
    }
    app.runtime
        .scenario_control()
        .unwrap()
        .advance_to(1000)
        .await
        .unwrap();
    assert!(poll_image(app, &id).await);
    assert!(app.image_op_operation_id.is_none());
    if cancel {
        assert!(!image_state(app).running);
        assert!(image_state(app).error.is_some());
    } else {
        assert!(matches!(app.dialog, Some(ShowDialog::Info { .. })));
    }
    assert!(
        crate::operations::ImageClient::with_operations(app.runtime.operations())
            .workflow_status(&id)
            .await
            .is_err(),
        "terminal operation released"
    );
    open_image(app).await;
    assert!(
        outputs(image(
            app,
            I::Complete {
                operation_id: id.clone(),
                result: Ok(())
            }
        ))
        .await
        .is_empty()
    );
    assert!(
        outputs(image(app, I::Progress(id, 999, 1000, 1)))
            .await
            .is_empty()
    );
    assert!(!image_state(app).running);
    assert!(image_state(app).progress.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn image_cancel_before_start_completion_is_honoured(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    open_image(app).await;
    let task = image(app, I::Start);
    assert!(outputs(image(app, I::CancelOperation)).await.is_empty());
    settle(app, task).await;
    let id = image_state(app).operation_id.clone().unwrap();
    assert!(poll_image(app, &id).await);
    assert!(!image_state(app).running);
    assert!(image_state(app).error.is_some());
}

#[rstest]
#[case::missing_path("")]
#[case::unknown_asset("asset:missing")]
#[case::host_path("/not-a-scenario-image")]
#[tokio::test(flavor = "current_thread")]
async fn image_validation_and_adapter_errors_allow_retry(
    #[future(awt)] mut workflow_app: AppModel,
    #[case] path: &str,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    open_image(app).await;
    image_state(app).image_path = path.into();
    let task = image(app, I::Start);
    settle(app, task).await;
    assert!(!image_state(app).running);
    assert!(image_state(app).error.is_some());
    image_state(app).image_path = "asset:image".into();
    let task = image(app, I::Start);
    settle(app, task).await;
    assert!(image_state(app).running);
    assert!(image_state(app).operation_id.is_some());
}

#[fixture]
async fn workflow_app() -> AppModel {
    let runtime = crate::AppRuntime::scenario(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/scenarios/workflows/image-usage.toml"),
        None,
        None,
    )
    .unwrap();
    let drives = load_all_drives_with_operations(runtime.operations())
        .await
        .unwrap();
    let mut app = AppModel::for_workflow_test(runtime);
    assert!(
        outputs(update(&mut app, Message::UpdateNav(drives, None)))
            .await
            .is_empty()
    );
    app
}

fn image(app: &mut AppModel, message: I) -> Task<Message> {
    update(app, Message::ImageOperationDialog(message))
}

fn image_state(app: &mut AppModel) -> &mut crate::state::dialogs::ImageOperationDialog {
    let Some(ShowDialog::ImageOperation(state)) = app.dialog.as_mut() else {
        panic!("image dialog expected");
    };
    state
}

async fn open_image(app: &mut AppModel) {
    assert!(
        outputs(update(app, Message::CreateDiskFrom))
            .await
            .is_empty()
    );
    assert!(
        outputs(update(
            app,
            Message::ImagePathPicked(
                ImagePathPickerKind::ImageOperationCreate,
                Some("asset:image".into())
            )
        ))
        .await
        .is_empty()
    );
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn image_start_uses_selected_adapter_and_rejects_duplicate(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    open_image(app).await;
    let task = image(app, I::Start);
    assert!(image_state(app).running);
    assert!(outputs(image(app, I::Start)).await.is_empty());
    settle(app, task).await;
    assert!(
        image_state(app).error.is_none(),
        "{:?}",
        image_state(app).error
    );
    assert!(image_state(app).operation_id.is_some());
    let operation_id = image_state(app).operation_id.clone();
    assert_eq!(app.image_op_operation_id, operation_id);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn stale_image_start_cannot_bind_to_replacement_dialog(
    #[future(awt)] mut workflow_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut workflow_app;
    open_image(app).await;
    let completions = outputs(image(app, I::Start)).await;
    open_image(app).await;
    for message in completions {
        let task = update(app, message);
        settle(app, task).await;
    }
    assert!(image_state(app).operation_id.is_none());
    assert!(!image_state(app).running);
    assert!(app.image_op_operation_id.is_none());
}
