mod common;
use common::owned;
use futures::StreamExt;
use std::{path::Path, time::Duration};
use storage_contracts::{DeviceEventSource, PartitionOperations};
use storage_lab_tests::Result;
use storage_udisks::storage_types::DeviceEvent;

#[rstest::rstest]
#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn partition_create_edit_delete_round_trip(
    #[from(common::lab)]
    #[with("partition-lifecycle", 256)]
    #[future(awt)]
    lab: Result<common::OwnedLab>,
) -> Result<()> {
    let (mut fixture, backend, disk) = lab?;
    backend
        .create_partition_table(owned(&fixture, &disk)?, "gpt")
        .await?;
    let mut events = backend.device_events().await?;
    let partition = backend
        .create_partition(
            owned(&fixture, &disk)?,
            1024 * 1024,
            64 * 1024 * 1024,
            "0FC63DAF-8483-4772-8E79-3D69D8477DE4",
        )
        .await?;
    let object = format!(
        "/org/freedesktop/UDisks2/block_devices/{}",
        Path::new(&partition).file_name().unwrap().to_string_lossy()
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match events.next().await {
                Some(Ok(DeviceEvent::Added(path))) if path == object => break,
                Some(Ok(_)) => {}
                other => panic!("device events ended or failed: {other:?}"),
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    // Feed the returned device path into the contract, as application clients do.
    backend
        .set_partition_name(owned(&fixture, &partition)?, "lab-data")
        .await?;
    backend
        .set_partition_flags(owned(&fixture, &partition)?, 4)
        .await?;
    backend
        .set_partition_type(
            owned(&fixture, &partition)?,
            "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F",
        )
        .await?;
    let partitions = backend.list_partitions(&disk).await?;
    assert_eq!(partitions.len(), 1);
    assert_eq!(partitions[0].device, partition);
    assert_eq!(partitions[0].name, "lab-data");
    assert_eq!(partitions[0].flags, 4);
    assert_eq!(
        partitions[0].type_id.to_ascii_uppercase(),
        "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F"
    );
    assert!(
        backend
            .create_partition(owned(&fixture, &disk)?, 1024 * 1024, 64 * 1024 * 1024, "")
            .await
            .is_err(),
        "overlapping partition must fail"
    );
    backend
        .delete_partition(owned(&fixture, &partition)?)
        .await?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match events.next().await {
                Some(Ok(DeviceEvent::Removed(path))) if path == object => break,
                Some(Ok(_)) => {}
                other => panic!("device events ended or failed: {other:?}"),
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    assert!(backend.list_partitions(&disk).await?.is_empty());
    fixture.cleanup()?;
    Ok(())
}
