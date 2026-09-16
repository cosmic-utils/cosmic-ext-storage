use super::*;
use crate::message::dialogs::UnmountBusyMessage;

#[fixture]
async fn busy_app() -> AppModel {
    let runtime = crate::AppRuntime::scenario(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/scenarios/physical/busy-unmount.toml"),
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
    let control = app.nav.active_data_mut::<VolumesControl>().unwrap();
    control.selected_segment = control
        .segments
        .iter()
        .position(|segment| {
            segment
                .volume
                .as_ref()
                .is_some_and(|volume| volume.device_path.as_deref() == Some("/dev/ui-disk0p1"))
        })
        .unwrap();
    app
}

#[rstest]
#[case::segment(0)]
#[case::child(1)]
#[case::sidebar(2)]
#[tokio::test(flavor = "current_thread")]
async fn busy_result_routes_to_dialog_and_retry_preserves_mounted_state(
    #[future(awt)] mut busy_app: AppModel,
    #[case] route: u8,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut busy_app;
    let message = match route {
        0 => Message::VolumesMessage(VolumesControlMessage::Unmount),
        1 => Message::VolumesMessage(VolumesControlMessage::ChildUnmount(
            "/dev/ui-disk0p1".into(),
        )),
        _ => Message::SidebarVolumeUnmount {
            drive: "/dev/ui-disk0".into(),
            device_path: "/dev/ui-disk0p1".into(),
        },
    };
    let task = update(app, message);
    settle(app, task).await;
    let Some(ShowDialog::UnmountBusy(dialog)) = &app.dialog else {
        panic!("busy result must expose its actionable dialog")
    };
    assert_eq!(dialog.processes.len(), 1);
    assert_eq!(dialog.processes[0].pid, 4242);
    assert_eq!(dialog.mount_point, "/mnt/ui-data");
    let task = update(app, Message::UnmountBusy(UnmountBusyMessage::Retry));
    settle(app, task).await;
    assert!(matches!(app.dialog, Some(ShowDialog::UnmountBusy(_))));
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    let volume =
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1").unwrap();
    assert_eq!(volume.volume.mount_points, ["/mnt/ui-data"]);
    assert_eq!(
        app.runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        0
    );
    let task = update(app, Message::UnmountBusy(UnmountBusyMessage::Cancel));
    settle(app, task).await;
    assert!(app.dialog.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn unsupported_kill_is_an_error_not_a_fake_unmount_success(
    #[future(awt)] mut busy_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut busy_app;
    let task = update(app, Message::VolumesMessage(VolumesControlMessage::Unmount));
    settle(app, task).await;
    let task = update(app, Message::UnmountBusy(UnmountBusyMessage::KillAndRetry));
    settle(app, task).await;
    let Some(ShowDialog::Info { body, .. }) = &app.dialog else {
        panic!("unsupported operation must surface its error")
    };
    assert!(body.contains("unsupported"));
    assert_eq!(
        app.runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        0
    );
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    assert_eq!(
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1")
            .unwrap()
            .volume
            .mount_points,
        ["/mnt/ui-data"]
    );
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn child_mount_and_unmount_refresh_real_state(#[future(awt)] mut physical_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut physical_app;
    let task = create(app, CreateMessage::Partition);
    settle(app, task).await;
    let task = update(
        app,
        Message::VolumesMessage(VolumesControlMessage::ChildMount("/dev/ui-disk0p1".into())),
    );
    settle(app, task).await;
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    assert!(
        !crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1")
            .unwrap()
            .volume
            .mount_points
            .is_empty()
    );
    let task = update(
        app,
        Message::VolumesMessage(VolumesControlMessage::ChildUnmount(
            "/dev/ui-disk0p1".into(),
        )),
    );
    settle(app, task).await;
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    assert!(
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1")
            .unwrap()
            .volume
            .mount_points
            .is_empty()
    );
    assert!(
        outputs(update(
            app,
            Message::VolumesMessage(VolumesControlMessage::ChildUnmount("/dev/missing".into()))
        ))
        .await
        .is_empty()
    );
}
