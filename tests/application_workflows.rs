use std::collections::BTreeMap;

use cosmic_ext_storage::testing::{
    FixtureSecrets, ImageUsageIntent, ImageUsagePhase, LogicalIntent, LogicalPhase, NetworkIntent,
    NetworkPhase, PhysicalIntent, PhysicalPhase, ReloadIntent, ReloadPhase, SecretInput,
    WorkflowHarness, verify_workflow_facade_contract,
};
use storage_types::{CreatePartitionInfo, ImageAssetRef};

fn partition_info() -> CreatePartitionInfo {
    CreatePartitionInfo {
        name: "Scenario data".into(),
        size: 268_435_456,
        max_size: 268_435_456,
        offset: 1_048_576,
        selected_type: "8300".into(),
        filesystem_type: "ext4".into(),
        table_type: "gpt".into(),
        ..Default::default()
    }
}

fn luks_secrets() -> FixtureSecrets {
    let mut secrets = FixtureSecrets::none();
    secrets
        .insert(
            "luks0".into(),
            SecretInput::new("fixture-passphrase".into()),
        )
        .expect("fixture secret is accepted");
    secrets
}

#[tokio::test(flavor = "current_thread")]
async fn workflow_harness_uses_only_selected_scenario_runtime() {
    let first = WorkflowHarness::from_fixture("empty.toml", FixtureSecrets::none())
        .await
        .expect("first scenario harness");
    let second = WorkflowHarness::from_fixture("empty.toml", FixtureSecrets::none())
        .await
        .expect("second scenario harness");

    assert_eq!(first.selected_block_backend_id(), "ui-scenario");
    assert_eq!(second.selected_block_backend_id(), "ui-scenario");
    assert!(first.global_operations_lookup_is_rejected().await);
    assert!(second.global_operations_lookup_is_rejected().await);
}

#[tokio::test(flavor = "current_thread")]
async fn logical_open_preflight_confirm_executes_once_and_refreshes_once() {
    let mut harness =
        WorkflowHarness::from_fixture("logical/preflight.toml", FixtureSecrets::none())
            .await
            .expect("logical harness");

    harness
        .dispatch_logical(LogicalIntent::Open {
            device_path: "/dev/ui-disk0p1".into(),
        })
        .expect("open intent");
    harness.drive_until_idle().await.expect("preflight flow");
    assert_eq!(
        harness.logical_snapshot().phase,
        LogicalPhase::AwaitingConfirmation
    );

    harness
        .dispatch_logical(LogicalIntent::Confirm)
        .expect("confirm intent");
    harness.drive_until_idle().await.expect("execute flow");

    let snapshot = harness.logical_snapshot();
    assert_eq!(snapshot.phase, LogicalPhase::Completed);
    assert_eq!(snapshot.refresh_generation, 1);
    assert_eq!(
        harness
            .effect_records()
            .iter()
            .map(|record| record.operation)
            .collect::<Vec<_>>(),
        vec![
            "logical.capture_candidate",
            "logical.preflight",
            "logical.execute",
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn logical_stale_preflight_completion_is_rejected() {
    let mut harness =
        WorkflowHarness::from_fixture("logical/preflight.toml", FixtureSecrets::none())
            .await
            .expect("logical harness");

    harness
        .dispatch_logical(LogicalIntent::Open {
            device_path: "/dev/ui-disk0p1".into(),
        })
        .expect("first open");
    assert!(
        harness
            .execute_scheduled()
            .await
            .expect("execute first capture")
    );

    harness
        .dispatch_logical(LogicalIntent::Open {
            device_path: "/dev/ui-disk0p1".into(),
        })
        .expect("replacement open");
    assert!(harness.deliver_completion().expect("deliver stale capture"));
    harness
        .drive_until_idle()
        .await
        .expect("replacement preflight");

    assert_eq!(
        harness.logical_snapshot().phase,
        LogicalPhase::AwaitingConfirmation
    );
    assert_eq!(
        harness
            .effect_records()
            .iter()
            .filter(|record| record.operation == "logical.preflight")
            .count(),
        1
    );
}

#[tokio::test(flavor = "current_thread")]
async fn partition_format_validation_and_completion_preserve_effect_order() {
    let mut harness =
        WorkflowHarness::from_fixture("physical/partition-format.toml", FixtureSecrets::none())
            .await
            .expect("physical harness");
    let mut invalid = partition_info();
    invalid.size = 0;
    harness
        .dispatch_physical(PhysicalIntent::FormatPartition {
            disk: "/dev/ui-disk0".into(),
            info: invalid,
        })
        .expect("invalid intent");
    assert_eq!(harness.physical_snapshot().phase, PhysicalPhase::Failed);
    assert!(harness.effect_records().is_empty());

    harness
        .dispatch_physical(PhysicalIntent::FormatPartition {
            disk: "/dev/ui-disk0".into(),
            info: partition_info(),
        })
        .expect("valid intent");
    harness.drive_until_idle().await.expect("format flow");
    assert_eq!(harness.physical_snapshot().phase, PhysicalPhase::Completed);
    assert_eq!(harness.physical_snapshot().refresh_generation, 1);
    assert_eq!(
        harness
            .effect_records()
            .last()
            .map(|record| record.operation),
        Some("partition.create_partition_with_filesystem")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn busy_unmount_keeps_actionable_error_and_does_not_refresh() {
    let mut harness =
        WorkflowHarness::from_fixture("physical/busy-unmount.toml", FixtureSecrets::none())
            .await
            .expect("busy harness");
    harness
        .dispatch_physical(PhysicalIntent::Unmount {
            device: "/dev/ui-disk0p1".into(),
        })
        .expect("unmount intent");
    harness.drive_until_idle().await.expect("busy response");

    let snapshot = harness.physical_snapshot();
    assert_eq!(snapshot.phase, PhysicalPhase::Failed);
    assert_eq!(snapshot.refresh_generation, 0);
    assert_eq!(
        snapshot.error.as_ref().map(|error| error.kind),
        Some("busy")
    );
    assert_eq!(
        snapshot.error.as_ref().map(|error| error.reason.as_str()),
        Some("Fixture user is active")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn luks_unlock_uses_secret_input_and_redacts_every_projection() {
    let mut success = WorkflowHarness::from_fixture("physical/luks.toml", luks_secrets())
        .await
        .expect("luks harness");
    success
        .dispatch_physical(PhysicalIntent::Unlock {
            device: "/dev/ui-disk0p1".into(),
            secret: SecretInput::new("fixture-passphrase".into()),
        })
        .expect("unlock intent");
    success.drive_until_idle().await.expect("unlock success");
    assert_eq!(success.physical_snapshot().phase, PhysicalPhase::Completed);
    assert!(
        success
            .effect_records()
            .iter()
            .any(|record| record.has_secret)
    );
    assert_eq!(
        success
            .trace()
            .expect("trace projection")
            .iter()
            .map(|entry| entry.operation.as_str())
            .collect::<Vec<_>>(),
        vec!["encryption.unlock_luks"]
    );
    success
        .assert_trace_redacted_for(SecretInput::new("fixture-passphrase".into()))
        .expect("trace has no secret");

    let mut failure = WorkflowHarness::from_fixture("physical/luks.toml", luks_secrets())
        .await
        .expect("luks failure harness");
    failure
        .dispatch_physical(PhysicalIntent::Unlock {
            device: "/dev/ui-disk0p1".into(),
            secret: SecretInput::new("wrong-passphrase".into()),
        })
        .expect("unlock failure intent");
    failure
        .drive_until_idle()
        .await
        .expect("unlock failure completion");
    assert_eq!(failure.physical_snapshot().phase, PhysicalPhase::Failed);
    assert_eq!(
        failure
            .physical_snapshot()
            .error
            .as_ref()
            .map(|error| error.kind),
        Some("permission_denied")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn network_create_mount_and_status_are_reduced_from_one_flow() {
    let mut harness = WorkflowHarness::from_fixture("network/mount.toml", FixtureSecrets::none())
        .await
        .expect("network harness");
    harness
        .dispatch_network(NetworkIntent {
            config_id: "workflow-remote".into(),
            provider_id: "s3".into(),
            options: BTreeMap::from([
                ("endpoint".into(), "https://fixture.invalid".into()),
                ("bucket".into(), "storage".into()),
            ]),
        })
        .expect("network create");
    harness.drive_until_idle().await.expect("network workflow");
    assert_eq!(harness.network_snapshot().phase, NetworkPhase::Completed);
    assert_eq!(harness.network_snapshot().mounted, Some(true));
    assert_eq!(
        harness
            .effect_records()
            .iter()
            .map(|record| record.operation)
            .collect::<Vec<_>>(),
        vec![
            "network.create_config",
            "network.test_config",
            "network.mount",
            "network.mount_status",
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn image_progress_cancel_and_terminal_state_are_virtual_clock_driven() {
    let mut harness =
        WorkflowHarness::from_fixture("workflows/image-usage.toml", FixtureSecrets::none())
            .await
            .expect("image harness");
    harness
        .dispatch_image_usage(ImageUsageIntent::StartImage {
            asset: ImageAssetRef::new("asset:image").expect("asset"),
            device: "/dev/ui-disk0".into(),
        })
        .expect("start image");
    harness.drive_until_idle().await.expect("image start");
    harness
        .dispatch_reload(ReloadIntent::AdvanceTo { tick: 250 })
        .expect("advance clock");
    harness.drive_until_idle().await.expect("advance receipt");
    assert_eq!(harness.reload_snapshot().virtual_tick, 250);
    harness
        .dispatch_image_usage(ImageUsageIntent::PollImage)
        .expect("poll image");
    harness.drive_until_idle().await.expect("image progress");
    assert_eq!(
        harness.image_usage_snapshot().image_progress,
        Some((250, 1000))
    );

    harness
        .dispatch_image_usage(ImageUsageIntent::CancelImage)
        .expect("cancel image");
    harness.drive_until_idle().await.expect("image cancel");
    assert_eq!(
        harness.image_usage_snapshot().phase,
        ImageUsagePhase::ImageCancelled
    );
    harness
        .dispatch_reload(ReloadIntent::AdvanceTo { tick: 1000 })
        .expect("advance after cancellation");
    harness
        .drive_until_idle()
        .await
        .expect("advance after cancellation");
    harness
        .dispatch_image_usage(ImageUsageIntent::PollImage)
        .expect("poll cancellation");
    harness.drive_until_idle().await.expect("cancelled status");
    assert_eq!(
        harness.image_usage_snapshot().phase,
        ImageUsagePhase::ImageCancelled
    );
}

#[tokio::test(flavor = "current_thread")]
async fn image_usage_stale_completion_cannot_replace_newer_workflow_state() {
    let mut harness =
        WorkflowHarness::from_fixture("workflows/image-usage.toml", FixtureSecrets::none())
            .await
            .expect("image harness");
    harness
        .dispatch_image_usage(ImageUsageIntent::StartImage {
            asset: ImageAssetRef::new("asset:image").expect("asset"),
            device: "/dev/ui-disk0".into(),
        })
        .expect("start image");
    assert!(
        harness
            .execute_scheduled()
            .await
            .expect("execute image start")
    );

    harness
        .dispatch_image_usage(ImageUsageIntent::StartUsage {
            scan_id: "workflow-scan".into(),
            mounts: vec!["/mnt/ui-data".into()],
        })
        .expect("replace workflow");
    assert!(
        harness
            .deliver_completion()
            .expect("deliver stale image start")
    );
    harness
        .drive_until_idle()
        .await
        .expect("current workflow completes");

    let snapshot = harness.image_usage_snapshot();
    assert_eq!(snapshot.phase, ImageUsagePhase::UsageRunning);
    assert_eq!(snapshot.image_operation_id, None);
    assert_eq!(snapshot.usage_scan_id.as_deref(), Some("workflow-scan"));
}

#[tokio::test(flavor = "current_thread")]
async fn usage_scan_and_delete_map_results_without_host_file_access() {
    let mut harness =
        WorkflowHarness::from_fixture("workflows/image-usage.toml", FixtureSecrets::none())
            .await
            .expect("usage harness");
    harness
        .dispatch_image_usage(ImageUsageIntent::StartUsage {
            scan_id: "workflow-scan".into(),
            mounts: vec!["/mnt/ui-data".into()],
        })
        .expect("start usage");
    harness.drive_until_idle().await.expect("usage start");
    harness
        .dispatch_reload(ReloadIntent::AdvanceTo { tick: 1000 })
        .expect("usage advance");
    harness.drive_until_idle().await.expect("usage receipt");
    harness
        .dispatch_image_usage(ImageUsageIntent::PollUsage)
        .expect("usage poll");
    harness.drive_until_idle().await.expect("usage completion");
    assert_eq!(
        harness.image_usage_snapshot().phase,
        ImageUsagePhase::UsageCompleted
    );

    harness
        .dispatch_image_usage(ImageUsageIntent::DeleteUsage {
            paths: vec!["/mnt/ui-data/cache-file".into()],
        })
        .expect("usage delete");
    harness.drive_until_idle().await.expect("usage deletion");
    let snapshot = harness.image_usage_snapshot();
    assert_eq!(snapshot.phase, ImageUsagePhase::DeleteCompleted);
    assert_eq!(snapshot.deleted_paths, vec!["/mnt/ui-data/cache-file"]);
}

#[tokio::test(flavor = "current_thread")]
async fn scenario_reload_is_atomic_and_generation_checked_by_the_application() {
    let mut harness = WorkflowHarness::from_fixture("reload/live.toml", FixtureSecrets::none())
        .await
        .expect("reload harness");
    harness
        .stage_overlay_from_fixture(
            "scenario:reload/after.toml",
            "9c98b5cf59578f0acfad9dbbf8f0857dfd53eb25a0cbd55a446abed0e271be08",
        )
        .expect("stage valid overlay");
    harness
        .dispatch_reload(ReloadIntent::ReloadOverlay)
        .expect("valid reload");
    harness
        .drive_until_idle()
        .await
        .expect("valid reload completion");
    assert_eq!(harness.reload_snapshot().phase, ReloadPhase::Reloaded);
    assert_eq!(harness.reload_snapshot().generation, 2);

    harness
        .stage_overlay_from_fixture(
            "overlay:reload-invalid.toml",
            "5402f6a47e5e6696685f6bf245e12524434f44f70d97da9b1bfe0ff089f5918b",
        )
        .expect("stage invalid overlay");
    harness
        .dispatch_reload(ReloadIntent::ReloadOverlay)
        .expect("invalid reload");
    harness
        .drive_until_idle()
        .await
        .expect("invalid reload completion");
    assert_eq!(harness.reload_snapshot().phase, ReloadPhase::ReloadRejected);
    assert_eq!(harness.reload_snapshot().generation, 2);
}

#[tokio::test(flavor = "current_thread")]
async fn reload_stale_completion_cannot_move_virtual_time_backwards() {
    let mut harness = WorkflowHarness::from_fixture("reload/live.toml", FixtureSecrets::none())
        .await
        .expect("reload harness");
    harness
        .dispatch_reload(ReloadIntent::AdvanceTo { tick: 250 })
        .expect("first advance");
    assert!(
        harness
            .execute_scheduled()
            .await
            .expect("execute first advance")
    );
    harness
        .dispatch_reload(ReloadIntent::AdvanceTo { tick: 500 })
        .expect("replacement advance");
    assert!(
        harness
            .deliver_completion()
            .expect("deliver stale first advance")
    );
    harness.drive_until_idle().await.expect("current advance");
    assert_eq!(harness.reload_snapshot().virtual_tick, 500);
}

#[tokio::test(flavor = "current_thread")]
async fn workflow_effects_do_not_call_global_operations_context() {
    let harness = WorkflowHarness::from_fixture("empty.toml", FixtureSecrets::none())
        .await
        .expect("guarded harness");
    assert!(harness.global_operations_lookup_is_rejected().await);
}

#[tokio::test(flavor = "current_thread")]
async fn workflow_facade_covers_every_migrated_path() {
    verify_workflow_facade_contract().expect("workflow facade contract");
}
