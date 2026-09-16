use super::*;

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn refresh_preserves_valid_selection_and_drops_missing_identity(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let task = create(app, CreateMessage::Partition);
    settle(app, task).await;
    let device = app
        .nav
        .active_data::<VolumesControl>()
        .unwrap()
        .segments
        .iter()
        .find_map(|segment| {
            segment
                .volume
                .as_ref()
                .and_then(|volume| volume.device_path.clone())
        })
        .unwrap();
    let task = update(
        app,
        Message::SidebarSelectChild {
            device_path: device.clone(),
        },
    );
    settle(app, task).await;
    let task = update(app, Message::LoadDrivesIncremental);
    settle(app, task).await;
    assert_eq!(
        app.sidebar.selected_child,
        Some(SidebarNodeKey::Volume(device.clone()))
    );
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    assert_eq!(
        control.segments[control.selected_segment]
            .volume
            .as_ref()
            .unwrap()
            .device_path
            .as_deref(),
        Some(device.as_str())
    );
    assert!(
        outputs(update(
            app,
            Message::SidebarSelectChild {
                device_path: "missing".into()
            }
        ))
        .await
        .is_empty()
    );
    assert_eq!(
        app.sidebar.selected_child,
        Some(SidebarNodeKey::Volume(device))
    );
    let drives = app.sidebar.drives.clone();
    let task = update(
        app,
        Message::UpdateNavWithChildSelection(drives, Some("missing".into())),
    );
    settle(app, task).await;
    assert!(app.sidebar.selected_child.is_none());
    let task = update(app, Message::UpdateNav(Vec::new(), None));
    settle(app, task).await;
    assert!(app.nav.active_data::<crate::models::UiDrive>().is_none());
    assert!(app.sidebar.drives.is_empty());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn selected_runtimes_remain_isolated_through_real_handlers() {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let mut first = physical_app().await;
    let mut second = physical_app().await;
    let task = create(&mut first, CreateMessage::Partition);
    settle(&mut first, task).await;
    let task = update(&mut second, Message::LoadDrivesIncremental);
    settle(&mut second, task).await;
    assert_eq!(
        first
            .nav
            .active_data::<VolumesControl>()
            .unwrap()
            .partitions
            .len(),
        1
    );
    assert!(
        second
            .nav
            .active_data::<VolumesControl>()
            .unwrap()
            .partitions
            .is_empty()
    );
    assert!(!std::sync::Arc::ptr_eq(
        &first.runtime.operations(),
        &second.runtime.operations()
    ));
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn scenario_reload_event_refreshes_actual_models_and_rejects_invalid_overlay() {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let root = tempfile::tempdir().unwrap();
    let overlay = root.path().join("overlay.toml");
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui/scenarios");
    let runtime = crate::AppRuntime::scenario(
        fixtures.join("reload/live.toml"),
        Some(overlay.clone()),
        None,
    )
    .unwrap();
    let mut app = AppModel::for_handler_test(runtime);
    let task = update(&mut app, Message::LoadDrivesIncremental);
    settle(&mut app, task).await;
    let selected = app
        .nav
        .active_data::<crate::models::UiDrive>()
        .unwrap()
        .device()
        .to_owned();
    assert_eq!(app.sidebar.drives[0].disk.model, "Before");
    let mut events = crate::subscriptions::app::device_messages(app.runtime.operations())
        .await
        .unwrap();
    std::fs::copy(fixtures.join("reload/after.toml"), &overlay).unwrap();
    assert!(matches!(
        app.runtime
            .scenario_control()
            .unwrap()
            .reload_overlay()
            .await
            .unwrap(),
        storage_contracts::ScenarioReload::Applied(_)
    ));
    let event = tokio::time::timeout(std::time::Duration::from_secs(1), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(event, Message::LoadDrivesIncremental));
    let task = update(&mut app, event);
    settle(&mut app, task).await;
    assert_eq!(app.sidebar.drives[0].disk.model, "After");
    assert_eq!(
        app.nav
            .active_data::<crate::models::UiDrive>()
            .unwrap()
            .device(),
        selected
    );
    std::fs::write(&overlay, "invalid fixture").unwrap();
    assert!(matches!(
        app.runtime
            .scenario_control()
            .unwrap()
            .reload_overlay()
            .await
            .unwrap(),
        storage_contracts::ScenarioReload::Rejected { .. }
    ));
    let task = update(&mut app, Message::LoadDrivesIncremental);
    settle(&mut app, task).await;
    assert_eq!(app.sidebar.drives[0].disk.model, "After");
    assert_eq!(
        app.runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        1
    );
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn duplicate_build_and_old_finish_cannot_publish_over_new_refresh(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let list = outputs(update(app, Message::LoadDrivesIncremental)).await;
    let build = outputs(update(app, list.into_iter().next().unwrap()))
        .await
        .into_iter()
        .next()
        .unwrap();
    let finish = outputs(update(app, build.clone())).await;
    assert!(outputs(update(app, build)).await.is_empty());
    let task = update(app, Message::LoadDrivesIncremental);
    let current_id = app.sidebar.load_id;
    for message in finish {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert_eq!(app.sidebar.load_id, current_id);
    settle(app, task).await;
    assert_eq!(app.sidebar.drives.len(), 1);
    assert!(app.sidebar.drive_load_error.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn duplicate_device_candidates_are_rejected_without_publishing(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let mut list = outputs(update(app, Message::LoadDrivesIncremental))
        .await
        .into_iter()
        .next()
        .unwrap();
    if let Message::DriveListLoaded {
        result: Ok(disks), ..
    } = &mut list
    {
        disks.push(disks[0].clone());
    }
    assert!(outputs(update(app, list)).await.is_empty());
    assert_eq!(app.sidebar.drives.len(), 1);
    assert!(app.sidebar.drive_load_error.is_some());
    assert!(!app.sidebar.drives_loading);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn late_list_cannot_restart_or_replace_completed_refresh(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let stale = outputs(update(app, Message::LoadDrivesIncremental)).await;
    let current = update(app, Message::LoadDrivesIncremental);
    settle(app, current).await;
    assert!(!app.sidebar.drives_loading);
    for message in stale {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert!(!app.sidebar.drives_loading);
    assert_eq!(app.sidebar.drives.len(), 1);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn refresh_keeps_published_models_until_all_builds_succeed(
    #[future(awt)] mut physical_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let before = app.sidebar.drives[0].disk.clone();
    let list = outputs(update(app, Message::LoadDrivesIncremental)).await;
    let build = update(app, list.into_iter().next().unwrap());
    assert_eq!(
        app.sidebar.drives.len(),
        1,
        "loading must not publish an empty/partial replacement"
    );
    assert_eq!(app.sidebar.drives[0].disk, before);
    let completions = outputs(build).await;
    for mut message in completions {
        if let Message::DriveLoaded { result, .. } = &mut message {
            *result = Err("controlled build failure".into());
        }
        let task = update(app, message);
        settle(app, task).await;
    }
    assert!(!app.sidebar.drives_loading);
    assert_eq!(app.sidebar.drives.len(), 1);
    assert_eq!(app.sidebar.drives[0].disk, before);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn background_refresh_preserves_running_dialog(#[future(awt)] mut physical_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    partition_dialog(app).running = true;
    let task = update(app, Message::LoadDrivesIncremental);
    settle(app, task).await;
    assert!(partition_dialog(app).running);
}
