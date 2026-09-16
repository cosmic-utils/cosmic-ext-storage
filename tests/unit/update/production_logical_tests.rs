use super::*;
use storage_contracts::LogicalAction;

#[fixture]
fn logical_app() -> AppModel {
    let runtime = crate::AppRuntime::scenario(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/scenarios/logical/preflight.toml"),
        None,
        None,
    )
    .unwrap();
    AppModel::for_handler_test(runtime)
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn stale_topology_does_not_replace_current_candidate_resolution(mut logical_app: AppModel) {
    let app = &mut logical_app;
    let first = app.logical.begin_load();
    let second = app.logical.begin_load();
    let result = |resolution| storage_types::LogicalLoadResult {
        topology: storage_types::LogicalTopology::new(Vec::new(), Vec::new()).unwrap(),
        candidate_resolution: resolution,
    };
    let current = storage_types::LogicalCandidateResolution::Missing;
    assert!(
        outputs(update(
            app,
            Message::LogicalEntitiesLoaded {
                generation: second,
                result: Ok(result(current.clone()))
            }
        ))
        .await
        .is_empty()
    );
    assert!(
        outputs(update(
            app,
            Message::LogicalEntitiesLoaded {
                generation: first,
                result: Ok(result(
                    storage_types::LogicalCandidateResolution::Unavailable {
                        source: "stale".into(),
                        reason: "old result".into()
                    }
                )),
            }
        ))
        .await
        .is_empty()
    );
    assert_eq!(app.logical.candidate_resolution, Some(current));
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn operation_failure_clears_pending_without_success_refresh(mut logical_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut logical_app;
    let confirmed = prompt(app).await;
    // A concurrent actor advances the backend epoch after the user's review.
    app.runtime
        .operations()
        .execute_logical_action(confirmed.clone())
        .await
        .unwrap();
    let task = update(app, Message::LogicalActionConfirmed(confirmed));
    let mut messages = outputs(task).await;
    assert_eq!(messages.len(), 1);
    assert!(matches!(
        &messages[0],
        Message::LogicalActionFinished { result: Err(_), .. }
    ));
    assert!(outputs(update(app, messages.remove(0))).await.is_empty());
    assert!(app.logical.pending.is_none());
    assert!(app.dialog.is_none());
    assert!(
        app.logical
            .action_status
            .as_ref()
            .unwrap()
            .contains("stale")
    );
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
async fn action_form_validates_input_and_cancel_clears_actual_dialog(mut logical_app: AppModel) {
    use crate::message::dialogs::LogicalActionFormMessage;
    use crate::state::dialogs::LogicalActionForm;
    let action = storage_contracts::LogicalAction::ResizeBtrfsFilesystem {
        filesystem: storage_types::LogicalEntityId::new("btrfs-fs:fixture").unwrap(),
        request: storage_contracts::BtrfsResizeRequest::AbsoluteBytes(4096),
    };
    let reopened = LogicalActionForm::from_action(&action).unwrap();
    assert_eq!(reopened.action().unwrap(), action);
    let app = &mut logical_app;
    let form = LogicalActionForm::CreateLvmLogicalVolume {
        volume_group: storage_types::LogicalEntityId::new("lvm-vg:fixture").unwrap(),
        name: "data".into(),
        size_bytes: "invalid".into(),
    };
    assert!(
        outputs(update(app, Message::LogicalActionFormRequested(form)))
            .await
            .is_empty()
    );
    assert!(
        outputs(update(
            app,
            Message::LogicalActionForm(LogicalActionFormMessage::Submit)
        ))
        .await
        .is_empty()
    );
    assert!(
        matches!(app.dialog, Some(ShowDialog::LogicalActionForm(ref dialog)) if dialog.error.is_some())
    );
    assert!(
        outputs(update(
            app,
            Message::LogicalActionForm(LogicalActionFormMessage::SizeUpdate("1024".into()))
        ))
        .await
        .is_empty()
    );
    assert!(
        matches!(app.dialog, Some(ShowDialog::LogicalActionForm(ref dialog)) if dialog.error.is_none())
    );
    let messages = outputs(update(
        app,
        Message::LogicalActionForm(LogicalActionFormMessage::Submit),
    ))
    .await;
    assert_eq!(messages.len(), 1);
    assert!(matches!(
        &messages[0],
        Message::LogicalActionPrompted(LogicalAction::CreateLvmLogicalVolume {
            size_bytes: 1024,
            ..
        })
    ));
    assert!(app.dialog.is_none());
    assert!(app.logical.pending.is_none());
    let form = LogicalActionForm::CreateLvmLogicalVolume {
        volume_group: storage_types::LogicalEntityId::new("lvm-vg:fixture").unwrap(),
        name: "data".into(),
        size_bytes: "1024".into(),
    };
    let task = update(app, Message::LogicalActionFormRequested(form));
    settle(app, task).await;
    assert!(
        outputs(update(
            app,
            Message::LogicalActionForm(LogicalActionFormMessage::Cancel)
        ))
        .await
        .is_empty()
    );
    assert!(app.dialog.is_none());
}

async fn prompt(app: &mut AppModel) -> storage_contracts::ConfirmedLogicalAction {
    let task = update(
        app,
        Message::LogicalViewRequested {
            device_path: Some("/dev/ui-disk0p1".into()),
        },
    );
    settle(app, task).await;
    let anchor = app
        .logical
        .selected_candidate
        .as_ref()
        .expect("captured real candidate");
    let action = LogicalAction::CreateLvmVolumeGroup {
        name: "scenario-vg".into(),
        devices: vec![storage_types::BlockDeviceRef::new(
            anchor.block_id.clone(),
            anchor.fingerprint.clone().unwrap(),
            anchor.observed_epoch,
        )],
    };
    let task = update(app, Message::LogicalActionPrompted(action));
    settle(app, task).await;
    let Some(ShowDialog::LogicalActionConfirmation(dialog)) = &app.dialog else {
        panic!("actual confirmation dialog expected")
    };
    assert!(!dialog.running);
    dialog.confirmed.clone()
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn confirmation_executes_once_and_schedules_both_real_refreshes(mut logical_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut logical_app;
    let confirmed = prompt(app).await;
    let task = update(app, Message::LogicalActionConfirmed(confirmed.clone()));
    assert!(app.logical.pending.is_some());
    assert!(
        outputs(update(app, Message::LogicalActionConfirmed(confirmed)))
            .await
            .is_empty()
    );
    let mut finished = outputs(task).await;
    assert_eq!(finished.len(), 1);
    let duplicate = finished[0].clone();
    let refreshes = outputs(update(app, finished.remove(0))).await;
    assert_eq!(refreshes.len(), 2);
    assert_eq!(
        refreshes
            .iter()
            .filter(|message| matches!(message, Message::LoadLogicalEntities))
            .count(),
        1
    );
    assert_eq!(
        refreshes
            .iter()
            .filter(|message| matches!(message, Message::LoadDrivesIncremental))
            .count(),
        1
    );
    assert!(app.dialog.is_none());
    assert!(app.logical.pending.is_none());
    assert!(outputs(update(app, duplicate)).await.is_empty());
    for message in refreshes {
        let task = update(app, message);
        settle(app, task).await;
    }
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
    assert!(!app.logical.loading);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_confirmation_and_changed_identity_cannot_execute(mut logical_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut logical_app;
    let confirmed = prompt(app).await;
    let mut stale = confirmed.clone();
    stale.preflight_key.udisks_epoch += 1;
    assert!(
        outputs(update(app, Message::LogicalActionConfirmed(stale)))
            .await
            .is_empty()
    );
    assert!(app.logical.pending.is_none());
    assert!(
        outputs(update(app, Message::LogicalActionCancelled))
            .await
            .is_empty()
    );
    assert!(app.dialog.is_none());
    assert!(
        outputs(update(
            app,
            Message::LogicalActionConfirmed(confirmed.clone())
        ))
        .await
        .is_empty()
    );
    assert!(
        outputs(update(app, Message::LogicalActionExecute(confirmed)))
            .await
            .is_empty()
    );
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
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_preflight_cannot_reopen_confirmation(mut logical_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut logical_app;
    let confirmed = prompt(app).await;
    assert!(
        outputs(update(app, Message::LogicalActionCancelled))
            .await
            .is_empty()
    );
    let task = update(app, Message::LogicalActionPrompted(confirmed.action));
    let completions = outputs(task).await;
    assert!(
        outputs(update(app, Message::LogicalActionCancelled))
            .await
            .is_empty()
    );
    for message in completions {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert!(app.dialog.is_none());
    assert!(app.logical.confirmation.is_none());
}

#[rstest]
#[case::replaced(false)]
#[case::left_view(true)]
#[tokio::test(flavor = "current_thread")]
async fn stale_candidate_capture_cannot_change_current_selection(
    mut logical_app: AppModel,
    #[case] leave: bool,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut logical_app;
    let completions = outputs(update(
        app,
        Message::LogicalViewRequested {
            device_path: Some("/dev/ui-disk0p1".into()),
        },
    ))
    .await;
    if leave {
        assert!(
            outputs(update(
                app,
                Message::SidebarSelectDrive {
                    device_path: "/dev/nonexistent".into()
                }
            ))
            .await
            .is_empty()
        );
    } else {
        let task = update(
            app,
            Message::LogicalViewRequested {
                device_path: Some("/dev/other-candidate".into()),
            },
        );
        settle(app, task).await;
    }
    for message in completions {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert_eq!(
        app.logical.selected_device.as_deref(),
        if leave {
            None
        } else {
            Some("/dev/other-candidate")
        }
    );
    assert!(app.logical.selected_candidate.is_none());
}
