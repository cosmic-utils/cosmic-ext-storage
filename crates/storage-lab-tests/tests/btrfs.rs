mod common;
use common::owned;
use std::{path::Path, time::Duration};
use storage_contracts::*;
use storage_lab_tests::Result;
use storage_udisks::storage_types::{FormatOptions, MountOptions};

#[rstest::rstest]
#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn btrfs_subvolume_snapshot_default_and_conflict_round_trip(
    #[from(common::lab)]
    #[with("btrfs-lifecycle", 256)]
    #[future(awt)]
    lab: Result<common::OwnedLab>,
) -> Result<()> {
    let (mut fixture, backend, disk) = lab?;
    backend.enable_optional_modules().await?;
    backend
        .format_filesystem(
            owned(&fixture, &disk)?,
            "btrfs",
            "lab-btrfs",
            FormatOptions::default(),
        )
        .await?;
    let device = fixture.owned_loop(Path::new(&disk))?.clone();
    let mount = fixture.prepare_mount(&device, "mount")?;
    let mount_str = mount.to_string_lossy();
    backend
        .set_mount_options(
            owned(&fixture, &disk)?,
            false,
            false,
            false,
            None,
            None,
            None,
            "defaults".into(),
            mount_str.to_string(),
            disk.clone(),
            "btrfs".into(),
        )
        .await?;
    assert_eq!(
        backend
            .mount_filesystem(owned(&fixture, &disk)?, &mount_str, MountOptions::default())
            .await?,
        mount_str
    );
    let btrfs = disks_btrfs::BtrfsUtilBackend::new();
    btrfs.create_subvolume(&mount_str, "data").await?;
    assert!(
        btrfs.create_subvolume(&mount_str, "data").await.is_err(),
        "duplicate subvolume must fail"
    );
    let data = mount.join("data");
    std::fs::write(data.join("payload"), b"snapshot contents")?;
    let snapshot = mount.join("snapshot");
    btrfs
        .create_snapshot(
            &mount_str,
            &data.to_string_lossy(),
            &snapshot.to_string_lossy(),
            true,
        )
        .await?;
    assert_eq!(
        std::fs::read(snapshot.join("payload"))?,
        b"snapshot contents"
    );
    assert!(std::fs::write(snapshot.join("forbidden"), b"read only").is_err());
    let volumes = btrfs.list_subvolumes(&mount_str).await?;
    assert_eq!(volumes.subvolumes.len(), 2);
    let original_default = volumes.default_id;
    btrfs
        .set_default(&mount_str, &data.to_string_lossy())
        .await?;
    assert_ne!(btrfs.default_subvolume(&mount_str).await?, original_default);
    btrfs.set_default(&mount_str, &mount_str).await?;
    btrfs
        .delete_subvolume(&mount_str, &snapshot.to_string_lossy(), false)
        .await?;
    btrfs
        .delete_subvolume(&mount_str, &data.to_string_lossy(), false)
        .await?;
    assert!(
        btrfs
            .list_subvolumes(&mount_str)
            .await?
            .subvolumes
            .is_empty()
    );
    let second = fixture
        .attach_sparse_loop("second.img", 256 * 1024 * 1024)?
        .path()
        .to_string_lossy()
        .into_owned();
    backend.enable_optional_modules().await?;
    let mut observed = Vec::new();
    let mut filesystem = None;
    for _ in 0..100 {
        observed = backend.list_logical_entities().await?;
        filesystem = observed.iter().find(|entity| matches!(&entity.details,
            storage_udisks::storage_types::LogicalEntityDetails::BtrfsFilesystem(details)
            if details.members.iter().any(|member| member.display_path.as_known().is_some_and(|path| path == &disk)))).cloned();
        if filesystem.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let filesystem = filesystem.ok_or_else(|| {
        format!("formatted Btrfs filesystem missing from logical discovery: {observed:?}")
    })?;
    let mut added = false;
    for _ in 0..100 {
        let review = backend
            .preflight_logical_action(LogicalPreflightRequest {
                request_key: LogicalPreflightRequestKey {
                    target: LogicalPreflightTarget::Root(filesystem.id.clone()),
                    action_kind: LogicalActionKind::AddBtrfsDevice,
                    logical_load_generation: 1,
                    draft_revision: 1,
                },
            })
            .await?;
        let candidate =
            review
                .device_candidates
                .into_iter()
                .find_map(|candidate| match candidate {
                    LogicalDeviceCandidate::Ready { device, display }
                        if display.path.as_known().is_some_and(|path| path == &second) =>
                    {
                        Some(device)
                    }
                    _ => None,
                });
        if let Some(device) = candidate {
            owned(&fixture, &second)?;
            backend
                .execute_logical_action(ConfirmedLogicalAction {
                    action: LogicalAction::AddBtrfsDevice {
                        filesystem: filesystem.id.clone(),
                        device,
                    },
                    preflight_key: review.key,
                })
                .await?;
            added = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        added,
        "second owned loop must be eligible for Btrfs membership"
    );
    let mut removed = false;
    for _ in 0..100 {
        let entities = backend.list_logical_entities().await?;
        let current = entities
            .iter()
            .find(|entity| entity.id == filesystem.id)
            .unwrap();
        if let storage_udisks::storage_types::LogicalEntityDetails::BtrfsFilesystem(details) =
            &current.details
            && details.members.len() == 2
            && let Some(device) = details
                .members
                .iter()
                .find(|member| {
                    member
                        .display_path
                        .as_known()
                        .is_some_and(|path| path == &second)
                })
                .and_then(|member| member.block.clone())
        {
            let ids: Vec<_> = details
                .members
                .iter()
                .map(|member| member.block.as_ref().unwrap().id.major_minor())
                .collect();
            assert!(
                ids.windows(2).all(|pair| pair[0] < pair[1]),
                "Btrfs members must use stable device ordering"
            );
            assert!(matches!(&details.primary_member,
                storage_udisks::storage_types::BtrfsPrimaryMember::Selected { member_id }
                if member_id == &details.members[0].member_id));
            let review = backend
                .preflight_logical_action(LogicalPreflightRequest {
                    request_key: LogicalPreflightRequestKey {
                        target: LogicalPreflightTarget::Root(filesystem.id.clone()),
                        action_kind: LogicalActionKind::RemoveBtrfsDevice,
                        logical_load_generation: 1,
                        draft_revision: 2,
                    },
                })
                .await?;
            owned(&fixture, &second)?;
            backend
                .execute_logical_action(ConfirmedLogicalAction {
                    action: LogicalAction::RemoveBtrfsDevice {
                        filesystem: filesystem.id.clone(),
                        device,
                    },
                    preflight_key: review.key,
                })
                .await?;
            removed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        removed,
        "new Btrfs member must be discoverable and removable; observed: {:?}",
        backend.list_logical_entities().await?
    );
    btrfs.create_subvolume(&mount_str, "stale-source").await?;
    let entities = backend.list_logical_entities().await?;
    let current = entities
        .iter()
        .find(|entity| entity.id == filesystem.id)
        .unwrap();
    let storage_udisks::storage_types::LogicalEntityDetails::BtrfsFilesystem(details) =
        &current.details
    else {
        panic!("expected Btrfs details");
    };
    let subvolume = details
        .subvolumes
        .iter()
        .find(|volume| volume.relative_path.as_str() == "stale-source")
        .expect("created subvolume must be exposed by native discovery");
    let review = backend
        .preflight_logical_action(LogicalPreflightRequest {
            request_key: LogicalPreflightRequestKey {
                target: LogicalPreflightTarget::Root(filesystem.id.clone()),
                action_kind: LogicalActionKind::DeleteBtrfsSubvolume,
                logical_load_generation: 1,
                draft_revision: 3,
            },
        })
        .await?;
    let stale = storage_udisks::storage_types::BtrfsSubvolumeRef {
        filesystem: filesystem.id.clone(),
        id: subvolume.id,
        expected_relative_path: subvolume.relative_path.clone(),
        expected_parent_id: subvolume.parent_id,
        observed_topology_epoch: review.key.udisks_epoch,
    };
    let stale_path = mount.join("stale-source");
    btrfs
        .delete_subvolume(&mount_str, &stale_path.to_string_lossy(), false)
        .await?;
    btrfs.create_subvolume(&mount_str, "stale-source").await?;
    let error = backend
        .execute_logical_action(ConfirmedLogicalAction {
            action: LogicalAction::DeleteBtrfsSubvolume {
                filesystem: filesystem.id.clone(),
                subvolume: stale,
            },
            preflight_key: review.key,
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::Conflict);
    assert!(
        stale_path.is_dir(),
        "stale confirmation must not delete the replacement"
    );
    btrfs
        .delete_subvolume(&mount_str, &stale_path.to_string_lossy(), false)
        .await?;
    backend
        .unmount_filesystem(owned(&fixture, &disk)?, false)
        .await?;
    backend.reset_mount_options(owned(&fixture, &disk)?).await?;
    fixture.cleanup()?;
    Ok(())
}
