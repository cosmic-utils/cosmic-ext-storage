use super::*;
use crate::message::dialogs::{
    EditMountOptionsMessage as M, EditPartitionMessage as E, ResizePartitionMessage as R,
};
use crate::state::dialogs::{
    DeletePartitionDialog, EditMountOptionsStep, EditPartitionStep, ResizePartitionStep,
};

fn volume(app: &mut AppModel, message: VolumesControlMessage) -> Task<Message> {
    update(app, Message::VolumesMessage(message))
}

#[fixture]
async fn selected_app(#[future(awt)] mut physical_app: AppModel) -> AppModel {
    let task = create(&mut physical_app, CreateMessage::Partition);
    settle(&mut physical_app, task).await;
    let control = physical_app
        .nav
        .active_data_mut::<VolumesControl>()
        .unwrap();
    control.selected_segment = control
        .segments
        .iter()
        .position(|segment| {
            segment
                .volume
                .as_ref()
                .is_some_and(|v| v.device_path.as_deref() == Some("/dev/ui-disk0p1"))
        })
        .unwrap();
    physical_app
}

#[rstest]
#[case::delete(0)]
#[case::edit(1)]
#[case::resize(2)]
#[case::mount_defaults(3)]
#[case::mount_custom(4)]
#[tokio::test(flavor = "current_thread")]
async fn real_volume_operations_report_unsupported_and_reject_stale_completion(
    #[future(awt)] mut selected_app: AppModel,
    #[case] kind: u8,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut selected_app;
    let confirm = prepare(app, kind).await;
    let task = volume(app, confirm.clone());
    assert!(
        outputs(volume(app, confirm)).await.is_empty(),
        "duplicate submission"
    );
    let mut messages = outputs(task).await;
    assert_eq!(messages.len(), 1);
    let result = messages.remove(0);
    let task = update(app, result.clone());
    settle(app, task).await;
    let Some(ShowDialog::Info { body, .. }) = &app.dialog else {
        panic!("operation failure must be surfaced")
    };
    assert!(body.contains("unsupported"), "{body}");
    // An old completion cannot close or replace a newly opened dialog.
    app.dialog = None;
    let _ = prepare(app, kind).await;
    let before = format!("{:?}", app.dialog);
    assert!(outputs(update(app, result)).await.is_empty());
    assert_eq!(format!("{:?}", app.dialog), before);
    assert_eq!(
        app.runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        2,
        "unsupported effects never mutate the fixture"
    );
}

async fn prepare(app: &mut AppModel, kind: u8) -> VolumesControlMessage {
    match kind {
        0 => {
            let task = update(
                app,
                Message::Dialog(Box::new(ShowDialog::DeletePartition(
                    DeletePartitionDialog {
                        operation_id: None,
                        name: "Scenario data".into(),
                        running: false,
                    },
                ))),
            );
            settle(app, task).await;
            VolumesControlMessage::Delete
        }
        1 => {
            let task = volume(app, VolumesControlMessage::OpenEditPartition);
            settle(app, task).await;
            assert!(matches!(app.dialog, Some(ShowDialog::EditPartition(_))));
            VolumesControlMessage::EditPartitionMessage(E::Confirm)
        }
        2 => {
            let task = volume(app, VolumesControlMessage::OpenResizePartition);
            settle(app, task).await;
            assert!(matches!(app.dialog, Some(ShowDialog::ResizePartition(_))));
            VolumesControlMessage::ResizePartitionMessage(R::Confirm)
        }
        _ => {
            let task = volume(app, VolumesControlMessage::OpenEditMountOptions);
            settle(app, task).await;
            let Some(ShowDialog::EditMountOptions(state)) = &app.dialog else {
                panic!("mount options")
            };
            assert_eq!(state.filesystem_type, "ext4");
            assert_eq!(
                state.identify_as_options[state.identify_as_index],
                "/dev/ui-disk0p1"
            );
            assert!(state.use_defaults);
            if kind == 4 {
                let _ = volume(
                    app,
                    VolumesControlMessage::EditMountOptionsMessage(M::UseDefaultsUpdate(false)),
                );
            }
            VolumesControlMessage::EditMountOptionsMessage(M::Confirm)
        }
    }
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn edit_and_resize_navigation_validate_bounds_and_cancel(
    #[future(awt)] mut selected_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut selected_app;
    prepare(app, 1).await;
    for msg in [
        E::SetStep(EditPartitionStep::Review),
        E::NextStep,
        E::NextStep,
        E::NextStep,
        E::PrevStep,
        E::SetStep(EditPartitionStep::Basics),
        E::NameUpdate("renamed".into()),
        E::LegacyBiosBootableUpdate(true),
        E::SystemPartitionUpdate(true),
        E::HiddenUpdate(true),
        E::TypeUpdate(usize::MAX),
    ] {
        assert!(
            outputs(volume(
                app,
                VolumesControlMessage::EditPartitionMessage(msg)
            ))
            .await
            .is_empty()
        );
    }
    let Some(ShowDialog::EditPartition(state)) = &app.dialog else {
        panic!()
    };
    assert_eq!(state.step, EditPartitionStep::Basics);
    assert_eq!(state.name, "renamed");
    assert!(state.legacy_bios_bootable && state.system_partition && state.hidden);
    assert!(
        outputs(volume(
            app,
            VolumesControlMessage::EditPartitionMessage(E::Confirm)
        ))
        .await
        .is_empty(),
        "invalid type index"
    );
    let task = volume(app, VolumesControlMessage::EditPartitionMessage(E::Cancel));
    settle(app, task).await;
    assert!(app.dialog.is_none());
    prepare(app, 2).await;
    for msg in [
        R::SetStep(ResizePartitionStep::Review),
        R::NextStep,
        R::NextStep,
        R::PrevStep,
        R::SetStep(ResizePartitionStep::Sizing),
        R::SizeUpdate(u64::MAX),
    ] {
        assert!(
            outputs(volume(
                app,
                VolumesControlMessage::ResizePartitionMessage(msg)
            ))
            .await
            .is_empty()
        );
    }
    let Some(ShowDialog::ResizePartition(state)) = &mut app.dialog else {
        panic!()
    };
    assert_eq!(state.step, ResizePartitionStep::Sizing);
    assert_eq!(state.new_size_bytes, state.max_size_bytes);
    state.new_size_bytes = state.max_size_bytes + 1;
    assert!(
        outputs(volume(
            app,
            VolumesControlMessage::ResizePartitionMessage(R::Confirm)
        ))
        .await
        .is_empty(),
        "submission must also validate bounds"
    );
    let task = volume(
        app,
        VolumesControlMessage::ResizePartitionMessage(R::Cancel),
    );
    settle(app, task).await;
    assert!(app.dialog.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn mount_option_form_updates_real_fields_and_cancels(
    #[future(awt)] mut selected_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut selected_app;
    prepare(app, 3).await;
    for msg in [
        M::SetStep(EditMountOptionsStep::Review),
        M::NextStep,
        M::NextStep,
        M::NextStep,
        M::PrevStep,
        M::SetStep(EditMountOptionsStep::Behavior),
        M::UseDefaultsUpdate(false),
        M::MountAtStartupUpdate(false),
        M::RequireAuthUpdate(true),
        M::ShowInUiUpdate(false),
        M::OtherOptionsUpdate("ro".into()),
        M::DisplayNameUpdate("Archive".into()),
        M::IconNameUpdate("disk".into()),
        M::SymbolicIconNameUpdate("disk-symbolic".into()),
        M::MountPointUpdate("/mnt/archive".into()),
        M::IdentifyAsIndexUpdate(0),
        M::FilesystemTypeUpdate("ext4".into()),
    ] {
        assert!(
            outputs(volume(
                app,
                VolumesControlMessage::EditMountOptionsMessage(msg)
            ))
            .await
            .is_empty()
        );
    }
    let Some(ShowDialog::EditMountOptions(state)) = &app.dialog else {
        panic!()
    };
    assert_eq!(state.step, EditMountOptionsStep::Behavior);
    assert!(
        !state.use_defaults && !state.mount_at_startup && state.require_auth && !state.show_in_ui
    );
    assert_eq!(
        (
            &*state.other_options,
            &*state.display_name,
            &*state.icon_name,
            &*state.symbolic_icon_name,
            &*state.mount_point
        ),
        ("ro", "Archive", "disk", "disk-symbolic", "/mnt/archive")
    );
    let task = volume(
        app,
        VolumesControlMessage::EditMountOptionsMessage(M::Cancel),
    );
    settle(app, task).await;
    assert!(app.dialog.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn missing_volume_does_not_start_or_strand_delete(#[future(awt)] mut selected_app: AppModel) {
    let app = &mut selected_app;
    let confirm = prepare(app, 0).await;
    app.nav
        .active_data_mut::<VolumesControl>()
        .unwrap()
        .selected_segment = usize::MAX;
    assert!(outputs(volume(app, confirm)).await.is_empty());
    assert!(matches!(&app.dialog, Some(ShowDialog::DeletePartition(state)) if !state.running));
    app.dialog = None;
    for msg in [
        VolumesControlMessage::OpenEditPartition,
        VolumesControlMessage::OpenResizePartition,
        VolumesControlMessage::OpenFormatPartition,
        VolumesControlMessage::OpenEditMountOptions,
    ] {
        assert!(outputs(volume(app, msg)).await.is_empty());
        assert!(app.dialog.is_none());
    }
}
