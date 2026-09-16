use super::*;
use crate::message::network::NetworkMessage as N;
use storage_types::rclone::{ConfigScope, rclone_provider};

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn overlapping_loads_reject_stale_success_error_and_duplicate(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    let old = outputs(network(app, N::LoadRemotes)).await.remove(0);
    let old_id = app.network.load_request_id.unwrap();
    let current = outputs(network(app, N::LoadRemotes)).await.remove(0);
    assert!(outputs(update(app, old)).await.is_empty());
    assert!(
        outputs(network(
            app,
            N::RemotesLoaded {
                request_id: old_id,
                result: Err("stale".into())
            }
        ))
        .await
        .is_empty()
    );
    assert!(app.network.loading);
    let task = update(app, current.clone());
    settle(app, task).await;
    assert!(!app.network.loading);
    assert!(app.network.error.is_none());
    assert!(outputs(update(app, current)).await.is_empty());
    assert!(app.network.get_mount("remote", ConfigScope::User).is_some());
}

#[rstest]
#[case::success(false)]
#[case::failure(true)]
#[tokio::test(flavor = "current_thread")]
async fn delete_requires_review_preserves_other_scope_and_rejects_stale_results(
    #[future(awt)] mut network_app: AppModel,
    #[case] fail: bool,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    let confirm = N::ConfirmDeleteRemote {
        name: "remote".into(),
        scope: ConfigScope::User,
    };
    assert!(outputs(network(app, confirm.clone())).await.is_empty());
    // Same name in another scope must never be removed by this completion.
    let mut other = app
        .network
        .get_mount("remote", ConfigScope::User)
        .unwrap()
        .config
        .clone();
    other.scope = ConfigScope::System;
    app.network.mounts.insert(
        ("remote".into(), ConfigScope::System),
        crate::state::network::NetworkMountState::new(other),
    );
    app.network
        .select(Some("remote".into()), Some(ConfigScope::System));
    let old_load = outputs(network(app, N::LoadRemotes)).await.remove(0);
    let _ = network(
        app,
        N::DeleteRemote {
            name: "remote".into(),
            scope: ConfigScope::User,
        },
    );
    if fail {
        crate::operations::RcloneClient::with_operations(app.runtime.operations())
            .delete_remote("remote", "user")
            .await
            .unwrap();
    }
    let completion = outputs(network(app, confirm.clone())).await.remove(0);
    assert!(outputs(network(app, confirm)).await.is_empty());
    let task = update(app, completion.clone());
    settle(app, task).await;
    assert!(outputs(update(app, completion)).await.is_empty());
    assert!(
        app.network
            .get_mount("remote", ConfigScope::System)
            .is_some()
    );
    assert_eq!(
        app.network.selected,
        Some(("remote".into(), ConfigScope::System))
    );
    if fail {
        assert!(matches!(app.dialog, Some(ShowDialog::Info { .. })));
        assert!(
            !app.network
                .get_mount("remote", ConfigScope::User)
                .unwrap()
                .loading
        );
    } else {
        assert!(outputs(update(app, old_load)).await.is_empty());
        assert!(app.network.get_mount("remote", ConfigScope::User).is_none());
        assert!(
            crate::operations::RcloneClient::with_operations(app.runtime.operations())
                .list_remotes()
                .await
                .unwrap()
                .remotes
                .is_empty()
        );
    }
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn old_status_cannot_overwrite_mount_and_errors_are_visible(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    let stale = outputs(network(
        app,
        N::RefreshStatus {
            name: "remote".into(),
            scope: ConfigScope::User,
        },
    ))
    .await;
    let mount = network(
        app,
        N::MountRemote {
            name: "remote".into(),
            scope: ConfigScope::User,
        },
    );
    assert!(
        outputs(network(
            app,
            N::MountRemote {
                name: "remote".into(),
                scope: ConfigScope::User
            }
        ))
        .await
        .is_empty()
    );
    settle(app, mount).await;
    for message in stale {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert!(
        app.network
            .get_mount("remote", ConfigScope::User)
            .unwrap()
            .is_mounted()
    );
    // An external removal leaves a known UI entry whose status request fails.
    crate::operations::RcloneClient::with_operations(app.runtime.operations())
        .delete_remote("remote", "user")
        .await
        .unwrap();
    let task = network(
        app,
        N::RefreshStatus {
            name: "remote".into(),
            scope: ConfigScope::User,
        },
    );
    settle(app, task).await;
    let mount = app.network.get_mount("remote", ConfigScope::User).unwrap();
    assert!(matches!(
        mount.status,
        storage_types::rclone::MountStatus::Error(_)
    ));
    assert!(mount.error.is_some());
    assert!(!mount.loading);
}

#[rstest]
#[case::unsupported("not-a-provider")]
#[case::missing_required("ftp")]
#[tokio::test(flavor = "current_thread")]
async fn invalid_provider_or_missing_schema_field_never_submits(
    #[future(awt)] mut network_app: AppModel,
    #[case] provider: &str,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    let _ = network(app, N::BeginCreateRemote);
    let _ = network(app, N::WizardSetName("invalid".into()));
    let _ = network(app, N::WizardSelectType(provider.into()));
    assert!(outputs(network(app, N::WizardCreate)).await.is_empty());
    assert!(app.network.wizard.as_ref().unwrap().error.is_some());
    assert!(!app.network.wizard.as_ref().unwrap().running);
    let _ = network(app, N::WizardAdvanced);
    assert!(outputs(network(app, N::SaveRemote)).await.is_empty());
    assert!(app.network.editor.as_ref().unwrap().error.is_some());
    assert!(!app.network.editor.as_ref().unwrap().running);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn editor_conflict_retry_updates_backend_and_completion_is_once(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    wizard(app, "remote").await;
    let _ = network(app, N::WizardAdvanced);
    let task = network(app, N::SaveRemote);
    settle(app, task).await;
    assert!(app.network.editor.as_ref().unwrap().error.is_some());
    assert!(!app.network.editor.as_ref().unwrap().running);
    let _ = network(app, N::EditorNameChanged("saved".into()));
    let completions = outputs(network(app, N::SaveRemote)).await;
    assert_eq!(completions.len(), 1);
    for message in completions {
        let task = update(app, message.clone());
        settle(app, task).await;
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert!(app.network.get_mount("saved", ConfigScope::User).is_some());
    let editor = app.network.editor.as_ref().unwrap();
    assert!(!editor.is_new);
    assert_eq!(editor.original_name.as_deref(), Some("saved"));
    let _ = network(
        app,
        N::EditorFieldChanged {
            key: "endpoint".into(),
            value: "https://changed.invalid".into(),
        },
    );
    let task = network(app, N::SaveRemote);
    settle(app, task).await;
    assert_eq!(
        app.network
            .get_mount("saved", ConfigScope::User)
            .unwrap()
            .config
            .options["endpoint"],
        "https://changed.invalid"
    );
}

fn network(app: &mut AppModel, message: N) -> Task<Message> {
    update(app, Message::Network(message))
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn failed_rename_preserves_original_remote(#[future(awt)] mut network_app: AppModel) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    wizard(app, "taken").await;
    let task = network(app, N::WizardCreate);
    settle(app, task).await;
    let task = network(
        app,
        N::SelectRemote {
            name: "remote".into(),
            scope: ConfigScope::User,
        },
    );
    settle(app, task).await;
    let _ = network(app, N::EditorNameChanged("taken".into()));
    let task = network(app, N::SaveRemote);
    settle(app, task).await;
    assert!(app.network.editor.as_ref().unwrap().error.is_some());
    let remotes = crate::operations::RcloneClient::with_operations(app.runtime.operations())
        .list_remotes()
        .await
        .unwrap()
        .remotes;
    assert!(
        remotes.iter().any(|remote| remote.name == "remote"),
        "failed rename must retain original configuration"
    );
    assert!(remotes.iter().any(|remote| remote.name == "taken"));
}

#[fixture]
async fn network_app() -> AppModel {
    let runtime = crate::AppRuntime::scenario(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ui/scenarios/network/mount.toml"),
        None,
        None,
    )
    .unwrap();
    let mut app = AppModel::for_handler_test(runtime);
    let task = network(&mut app, N::LoadRemotes);
    settle(&mut app, task).await;
    assert!(app.network.get_mount("remote", ConfigScope::User).is_some());
    app
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn successful_rename_updates_selection_and_rejects_duplicate(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    let task = network(
        app,
        N::SelectRemote {
            name: "remote".into(),
            scope: ConfigScope::User,
        },
    );
    settle(app, task).await;
    let _ = network(app, N::EditorNameChanged("renamed".into()));
    let completions = outputs(network(app, N::SaveRemote)).await;
    assert_eq!(completions.len(), 1);
    let completion = completions[0].clone();
    let task = update(app, completion.clone());
    settle(app, task).await;
    assert_eq!(
        app.network.selected,
        Some(("renamed".into(), ConfigScope::User))
    );
    let editor = app.network.editor.as_ref().unwrap();
    assert_eq!(editor.original_name.as_deref(), Some("renamed"));
    assert!(editor.error.is_none());
    assert!(app.network.get_mount("remote", ConfigScope::User).is_none());
    assert!(
        app.network
            .get_mount("renamed", ConfigScope::User)
            .is_some()
    );
    let remotes = crate::operations::RcloneClient::with_operations(app.runtime.operations())
        .list_remotes()
        .await
        .unwrap()
        .remotes;
    assert!(remotes.iter().any(|remote| remote.name == "renamed"));
    assert!(!remotes.iter().any(|remote| remote.name == "remote"));
    assert!(outputs(update(app, completion)).await.is_empty());
}

async fn wizard(app: &mut AppModel, name: &str) {
    for message in [
        N::BeginCreateRemote,
        N::WizardSelectType("s3".into()),
        N::WizardSetName(name.into()),
    ] {
        assert!(outputs(network(app, message)).await.is_empty());
    }
    for option in &rclone_provider("s3").unwrap().options {
        if option.required && !option.is_hidden() {
            assert!(
                outputs(network(
                    app,
                    N::WizardFieldChanged {
                        key: option.name.clone(),
                        value: "fixture".into(),
                    }
                ))
                .await
                .is_empty()
            );
        }
    }
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn create_select_mount_status_unmount_use_real_handlers(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    wizard(app, "created").await;
    let task = network(app, N::WizardCreate);
    assert!(app.network.wizard.as_ref().unwrap().running);
    assert!(
        outputs(network(app, N::WizardCreate)).await.is_empty(),
        "duplicate create must not execute"
    );
    settle(app, task).await;
    assert!(app.network.wizard.is_none());
    assert_eq!(
        app.network
            .editor
            .as_ref()
            .expect("created remote editor")
            .name,
        "created"
    );
    for (message, mounted) in [
        (
            N::MountRemote {
                name: "created".into(),
                scope: ConfigScope::User,
            },
            true,
        ),
        (
            N::RefreshStatus {
                name: "created".into(),
                scope: ConfigScope::User,
            },
            true,
        ),
        (
            N::UnmountRemote {
                name: "created".into(),
                scope: ConfigScope::User,
            },
            false,
        ),
    ] {
        let task = network(app, message);
        settle(app, task).await;
        let mount = app.network.get_mount("created", ConfigScope::User).unwrap();
        assert_eq!(mount.is_mounted(), mounted);
        assert!(!mount.loading);
        assert!(mount.error.is_none());
    }
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn create_validation_conflict_and_retry_preserve_form(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    wizard(app, "").await;
    assert!(outputs(network(app, N::WizardCreate)).await.is_empty());
    assert!(
        app.network
            .wizard
            .as_ref()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("name")
    );
    let _ = network(app, N::WizardSetName("remote".into()));
    let task = network(app, N::WizardCreate);
    settle(app, task).await;
    assert!(!app.network.wizard.as_ref().unwrap().running);
    assert!(app.network.wizard.as_ref().unwrap().error.is_some());
    let _ = network(app, N::WizardSetName("retry".into()));
    let task = network(app, N::WizardCreate);
    settle(app, task).await;
    assert!(app.network.get_mount("retry", ConfigScope::User).is_some());
    assert!(app.network.wizard.is_none());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_create_completion_cannot_replace_new_wizard(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    wizard(app, "old").await;
    let completions = outputs(network(app, N::WizardCreate)).await;
    assert_eq!(completions.len(), 1);
    let _ = network(app, N::WizardCancel);
    wizard(app, "new").await;
    for message in completions {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    assert_eq!(app.network.wizard.as_ref().unwrap().name, "new");
    assert!(!app.network.wizard.as_ref().unwrap().running);
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn editor_save_rejects_duplicate_and_stale_completion(
    #[future(awt)] mut network_app: AppModel,
) {
    let _guard = crate::operations::forbid_global_operations_for_handler_tests();
    let app = &mut network_app;
    wizard(app, "advanced").await;
    let _ = network(app, N::WizardAdvanced);
    let task = network(app, N::SaveRemote);
    assert!(app.network.editor.as_ref().unwrap().running);
    assert!(outputs(network(app, N::SaveRemote)).await.is_empty());
    let completions = outputs(task).await;
    wizard(app, "replacement").await;
    let _ = network(app, N::WizardAdvanced);
    for message in completions {
        assert!(outputs(update(app, message)).await.is_empty());
    }
    let editor = app.network.editor.as_ref().unwrap();
    assert_eq!(editor.name, "replacement");
    assert!(editor.is_new);
    assert!(!editor.running);
}
