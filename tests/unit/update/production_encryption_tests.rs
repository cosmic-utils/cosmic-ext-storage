use super::*;
use crate::message::dialogs::UnlockMessage;
use crate::state::dialogs::UnlockEncryptedDialog;

#[fixture]
async fn encrypted_app() -> AppModel {
    let runtime = crate::AppRuntime::scenario_with_fixture_secrets(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/scenarios/physical/luks.toml"),
        None,
        None,
        std::collections::BTreeMap::from([("luks0".into(), "fixture-passphrase".into())]),
    )
    .unwrap();
    let drives = load_all_drives_with_operations(runtime.operations())
        .await
        .unwrap();
    let mut app = AppModel::for_handler_test(runtime);
    assert!(
        outputs(update(&mut app, Message::UpdateNav(drives, None)))
            .await
            .is_empty()
    );
    app.dialog = Some(ShowDialog::UnlockEncrypted(UnlockEncryptedDialog {
        operation_id: None,
        partition_path: "/dev/ui-disk0p1".into(),
        partition_name: "Encrypted data".into(),
        passphrase: String::new(),
        error: None,
        running: false,
    }));
    app
}

fn unlock(app: &mut AppModel, message: UnlockMessage) -> Task<Message> {
    update(
        app,
        Message::VolumesMessage(VolumesControlMessage::UnlockMessage(message)),
    )
}

#[rstest]
#[case::empty("", "")]
#[case::mismatch("next-secret", "different-secret")]
#[case::valid("next-secret", "next-secret")]
#[tokio::test(flavor = "current_thread")]
async fn passphrase_change_validates_redacts_and_rejects_late_completion(
    #[future(awt)] mut encrypted_app: AppModel,
    #[case] next: &str,
    #[case] confirmation: &str,
) {
    use crate::message::dialogs::ChangePassphraseMessage as C;
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut encrypted_app;
    app.dialog = None;
    let control = app.nav.active_data_mut::<VolumesControl>().unwrap();
    control.selected_segment = control
        .segments
        .iter()
        .position(|s| {
            s.volume
                .as_ref()
                .is_some_and(|v| v.device_path.as_deref() == Some("/dev/ui-disk0p1"))
        })
        .unwrap();
    let task = update(
        app,
        Message::VolumesMessage(VolumesControlMessage::OpenChangePassphrase),
    );
    settle(app, task).await;
    for msg in [
        C::CurrentUpdate("fixture-passphrase".into()),
        C::NewUpdate(next.into()),
        C::ConfirmUpdate(confirmation.into()),
    ] {
        assert!(!format!("{msg:?}").contains("secret"));
        assert!(!format!("{msg:?}").contains("fixture-passphrase"));
        assert!(
            outputs(update(
                app,
                Message::VolumesMessage(VolumesControlMessage::ChangePassphraseMessage(msg))
            ))
            .await
            .is_empty()
        );
    }
    let Some(ShowDialog::ChangePassphrase(state)) = &app.dialog else {
        panic!("actual encryption form")
    };
    assert_eq!(state.new_passphrase, next);
    assert!(!format!("{state:?}").contains("fixture-passphrase"));
    let messages = outputs(update(
        app,
        Message::VolumesMessage(VolumesControlMessage::ChangePassphraseMessage(C::Confirm)),
    ))
    .await;
    if next.is_empty() || next != confirmation {
        assert!(messages.is_empty());
        assert!(
            matches!(&app.dialog, Some(ShowDialog::ChangePassphrase(state)) if state.error.is_some() && !state.running)
        );
    } else {
        assert_eq!(messages.len(), 1);
        assert!(
            outputs(update(
                app,
                Message::VolumesMessage(VolumesControlMessage::ChangePassphraseMessage(C::Confirm))
            ))
            .await
            .is_empty()
        );
        let completion = messages.into_iter().next().unwrap();
        let task = update(app, completion.clone());
        settle(app, task).await;
        assert!(
            matches!(&app.dialog, Some(ShowDialog::Info { body, .. }) if body.contains("unsupported"))
        );
        app.dialog = None;
        let task = update(
            app,
            Message::VolumesMessage(VolumesControlMessage::OpenChangePassphrase),
        );
        settle(app, task).await;
        assert!(outputs(update(app, completion)).await.is_empty());
        assert!(matches!(&app.dialog, Some(ShowDialog::ChangePassphrase(state)) if !state.running));
    }
    let task = update(
        app,
        Message::VolumesMessage(VolumesControlMessage::ChangePassphraseMessage(C::Cancel)),
    );
    settle(app, task).await;
    assert!(app.dialog.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn wrong_secret_retries_successfully_then_locks_actual_model(
    #[future(awt)] mut encrypted_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut encrypted_app;
    assert!(
        outputs(unlock(
            app,
            UnlockMessage::PassphraseUpdate("wrong-secret".into())
        ))
        .await
        .is_empty()
    );
    let task = unlock(app, UnlockMessage::Confirm);
    assert!(
        outputs(unlock(app, UnlockMessage::Confirm))
            .await
            .is_empty(),
        "duplicate submission"
    );
    settle(app, task).await;
    let Some(ShowDialog::UnlockEncrypted(state)) = &app.dialog else {
        panic!("retry dialog expected")
    };
    assert!(!state.running);
    assert!(state.error.as_ref().unwrap().contains("incorrect"));
    assert!(
        !format!("{:?}", app.dialog).contains("wrong-secret"),
        "dialog debug projection is redacted"
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
    let message = UnlockMessage::PassphraseUpdate("fixture-passphrase".into());
    assert!(!format!("{message:?}").contains("fixture-passphrase"));
    assert!(outputs(unlock(app, message)).await.is_empty());
    let task = unlock(app, UnlockMessage::Confirm);
    settle(app, task).await;
    assert!(app.dialog.is_none());
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    let container =
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1").unwrap();
    assert!(!container.volume.locked);
    assert_eq!(container.children.len(), 1);
    assert_eq!(
        container.children[0].device(),
        Some("/dev/mapper/ui-crypt-data")
    );
    let task = update(
        app,
        Message::VolumesMessage(VolumesControlMessage::LockContainer),
    );
    settle(app, task).await;
    let control = app.nav.active_data::<VolumesControl>().unwrap();
    let container =
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, "/dev/ui-disk0p1").unwrap();
    assert!(container.volume.locked);
    assert!(container.children.is_empty());
    assert_eq!(
        app.runtime
            .scenario_control()
            .unwrap()
            .diagnostics()
            .await
            .unwrap()
            .generation,
        2
    );
}

#[rstest]
#[case::cancel(true)]
#[case::missing_partition(false)]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_or_missing_target_never_unlocks(
    #[future(awt)] mut encrypted_app: AppModel,
    #[case] cancel: bool,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut encrypted_app;
    let message = if cancel {
        UnlockMessage::Cancel
    } else {
        app.nav
            .active_data_mut::<VolumesControl>()
            .unwrap()
            .partitions
            .clear();
        UnlockMessage::Confirm
    };
    let task = unlock(app, message);
    settle(app, task).await;
    if cancel {
        assert!(app.dialog.is_none());
    } else {
        assert!(matches!(app.dialog, Some(ShowDialog::Info { .. })));
    }
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
async fn cancelled_unlock_failure_cannot_reopen_a_secret_dialog(
    #[future(awt)] mut encrypted_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut encrypted_app;
    let task = unlock(app, UnlockMessage::Confirm);
    let completions = outputs(task).await;
    let task = unlock(app, UnlockMessage::Cancel);
    settle(app, task).await;
    for message in completions {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert!(app.dialog.is_none());
}
