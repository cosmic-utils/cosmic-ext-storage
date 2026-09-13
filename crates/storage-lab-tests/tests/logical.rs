mod common;
use common::{lab, owned};
use std::time::Duration;
use storage_contracts::*;
use storage_lab_tests::Result;
use storage_udisks::{
    UdisksBackend,
    storage_types::{
        BlockDeviceRef, ConfirmedDestructiveScope, LogicalEntity, LogicalEntityDetails,
        LogicalEntityId,
    },
};

async fn preflight(
    backend: &UdisksBackend,
    kind: LogicalActionKind,
    target: LogicalPreflightTarget,
) -> Result<LogicalPreflight> {
    Ok(backend
        .preflight_logical_action(LogicalPreflightRequest {
            request_key: LogicalPreflightRequestKey {
                target,
                action_kind: kind,
                logical_load_generation: 1,
                draft_revision: 1,
            },
        })
        .await?)
}

async fn execute(
    backend: &UdisksBackend,
    action: LogicalAction,
    target: LogicalPreflightTarget,
) -> Result<()> {
    let mut review = preflight(backend, action.kind(), target.clone()).await?;
    for _ in 0..100 {
        if review.availability == LogicalPreflightAvailability::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        review = preflight(backend, action.kind(), target.clone()).await?;
    }
    assert_eq!(
        review.availability,
        LogicalPreflightAvailability::Ready,
        "preflight for {:?}",
        action.kind()
    );
    backend
        .execute_logical_action(ConfirmedLogicalAction {
            action,
            preflight_key: review.key,
        })
        .await?;
    Ok(())
}

async fn candidate(
    backend: &UdisksBackend,
    disk: &str,
    kind: LogicalActionKind,
) -> Result<BlockDeviceRef> {
    for _ in 0..100 {
        let review = preflight(backend, kind, LogicalPreflightTarget::Landing).await?;
        for candidate in review.device_candidates {
            if let LogicalDeviceCandidate::Ready { device, display } = candidate
                && display.path.as_known().is_some_and(|path| path == disk)
            {
                return Ok(device);
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("no eligible owned candidate for {disk}").into())
}

async fn entity(backend: &UdisksBackend, prefix: &str, name: &str) -> Result<LogicalEntity> {
    for _ in 0..100 {
        if let Some(entity) = backend
            .list_logical_entities()
            .await?
            .into_iter()
            .find(|entity| entity.id.0.starts_with(prefix) && (entity.name == name || matches!(&entity.details,
                LogicalEntityDetails::MdRaidArray(details) if details.members.iter().any(|member| member.display_path.as_known().is_some_and(|path| path == name)))))
        {
            return Ok(entity);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let observed: Vec<_> = backend
        .list_logical_entities()
        .await?
        .into_iter()
        .map(|entity| (entity.id, entity.name))
        .collect();
    Err(format!("logical entity did not appear: {prefix} {name}; observed: {observed:?}").into())
}

async fn delete(
    backend: &UdisksBackend,
    id: LogicalEntityId,
    kind: LogicalActionKind,
) -> Result<()> {
    let review = preflight(backend, kind, LogicalPreflightTarget::Root(id.clone())).await?;
    let LogicalReviewData::DestructiveScope { scope, .. } = review.review else {
        panic!("deletion must expose its exact destructive scope");
    };
    let action = match kind {
        LogicalActionKind::DeleteLvmVolumeGroup => LogicalAction::DeleteLvmVolumeGroup {
            volume_group: id,
            confirmed_scope: scope,
            pv_label_policy: LvmWipePolicy::Preserve,
            configuration_policy: ConfigurationCleanupPolicy::Preserve,
        },
        LogicalActionKind::DeleteMdRaidArray => LogicalAction::DeleteMdRaidArray {
            array: id,
            confirmed_scope: scope,
            configuration_policy: ConfigurationCleanupPolicy::Preserve,
        },
        _ => panic!("unsupported deletion in this test"),
    };
    backend
        .execute_logical_action(ConfirmedLogicalAction {
            action,
            preflight_key: review.key,
        })
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn lvm_create_resize_delete_and_stale_review() -> Result<()> {
    let (mut fixture, backend, disk) = lab("lvm-lifecycle", 256).await?;
    backend.enable_optional_modules().await?;
    let name = format!(
        "storage-lab-{}",
        fixture
            .root()?
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
    fixture.prepare_volume_group(&name)?;
    let device = candidate(
        &backend,
        owned(&fixture, &disk)?,
        LogicalActionKind::CreateLvmVolumeGroup,
    )
    .await?;
    let action = LogicalAction::CreateLvmVolumeGroup {
        name: name.clone(),
        devices: vec![device],
    };
    let mut review = preflight(&backend, action.kind(), LogicalPreflightTarget::Landing).await?;
    review.key.udisks_epoch += 1;
    let error = backend
        .execute_logical_action(ConfirmedLogicalAction {
            action: action.clone(),
            preflight_key: review.key,
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::Conflict);
    execute(&backend, action, LogicalPreflightTarget::Landing).await?;
    fixture.track_volume_group(&name)?;
    let vg = entity(&backend, "lvm-vg:", &name).await?;
    execute(
        &backend,
        LogicalAction::CreateLvmLogicalVolume {
            volume_group: vg.id.clone(),
            name: "data".into(),
            size_bytes: 32 * 1024 * 1024,
        },
        LogicalPreflightTarget::Root(vg.id.clone()),
    )
    .await?;
    let lv = entity(&backend, "lvm-lv:", "data").await?;
    execute(
        &backend,
        LogicalAction::ResizeLvmLogicalVolume {
            logical_volume: lv.id.clone(),
            size_bytes: 64 * 1024 * 1024,
        },
        LogicalPreflightTarget::Root(lv.id.clone()),
    )
    .await?;
    let mut resized = entity(&backend, "lvm-lv:", "data").await?;
    for _ in 0..100 {
        if matches!(&resized.details, LogicalEntityDetails::LvmLogicalVolume(details) if details.size.as_known() == Some(&(64 * 1024 * 1024)))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        resized = entity(&backend, "lvm-lv:", "data").await?;
    }
    let LogicalEntityDetails::LvmLogicalVolume(details) = resized.details else {
        panic!("expected a typed logical volume");
    };
    assert_eq!(details.size.as_known(), Some(&(64 * 1024 * 1024)));
    execute(
        &backend,
        LogicalAction::DeleteLvmLogicalVolume {
            logical_volume: lv.id.clone(),
            configuration_policy: ConfigurationCleanupPolicy::Preserve,
            confirmed_scope: ConfirmedDestructiveScope::new(Vec::new(), Vec::new()).unwrap(),
        },
        LogicalPreflightTarget::Root(lv.id),
    )
    .await?;
    delete(
        &backend,
        vg.id.clone(),
        LogicalActionKind::DeleteLvmVolumeGroup,
    )
    .await?;
    for _ in 0..100 {
        if !backend
            .list_logical_entities()
            .await?
            .iter()
            .any(|entity| entity.id == vg.id)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !backend
            .list_logical_entities()
            .await?
            .iter()
            .any(|entity| entity.id == vg.id)
    );
    fixture.cleanup()?;
    Ok(())
}

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn mdraid_create_stop_start_delete_and_invalid_members() -> Result<()> {
    let (mut fixture, backend, first) = lab("mdraid-lifecycle", 128).await?;
    let second = fixture
        .attach_sparse_loop("second.img", 128 * 1024 * 1024)?
        .path()
        .to_string_lossy()
        .into_owned();
    let first_ref = candidate(
        &backend,
        owned(&fixture, &first)?,
        LogicalActionKind::CreateMdRaidArray,
    )
    .await?;
    let second_ref = candidate(
        &backend,
        owned(&fixture, &second)?,
        LogicalActionKind::CreateMdRaidArray,
    )
    .await?;
    let level = MdRaidLevel::Raid1;
    let name = "storage-lab-raid";
    fixture.prepare_array(
        name,
        &[
            fixture.owned_loop(std::path::Path::new(&first))?.clone(),
            fixture.owned_loop(std::path::Path::new(&second))?.clone(),
        ],
    )?;
    let invalid = LogicalAction::CreateMdRaidArray {
        name: MdRaidName(name.into()),
        level,
        devices: vec![first_ref.clone(), first_ref.clone()],
        profile: MdRaidCreateProfile::for_level(level),
    };
    assert!(
        invalid.validate().is_err(),
        "duplicate array members must be rejected"
    );
    execute(
        &backend,
        LogicalAction::CreateMdRaidArray {
            name: MdRaidName(name.into()),
            level,
            devices: vec![first_ref, second_ref],
            profile: MdRaidCreateProfile::for_level(level),
        },
        LogicalPreflightTarget::Landing,
    )
    .await?;
    // Metadata 0.90 does not persist a user-defined name. Resolve the new
    // array by its owned member, never by a guessed /dev/md number.
    let array = entity(&backend, "mdraid:", &first).await?;
    if let Some(path) = &array.device_path {
        fixture.track_array(std::path::Path::new(path))?;
    }
    execute(
        &backend,
        LogicalAction::StopMdRaidArray {
            array: array.id.clone(),
        },
        LogicalPreflightTarget::Root(array.id.clone()),
    )
    .await?;
    execute(
        &backend,
        LogicalAction::StartMdRaidArray {
            array: array.id.clone(),
        },
        LogicalPreflightTarget::Root(array.id.clone()),
    )
    .await?;
    delete(&backend, array.id, LogicalActionKind::DeleteMdRaidArray).await?;
    fixture.cleanup()?;
    Ok(())
}
