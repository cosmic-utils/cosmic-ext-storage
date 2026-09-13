//! Selected application-composition cases, baked into the same private lab.
use cosmic_ext_storage::{
    operations::{OperationError, PartitionsClient},
    runtime::AppRuntime,
};
use std::{path::Path, sync::Arc, time::Duration};
use storage_lab_tests::{LabFixture, Result};
use storage_udisks::UdisksBackend;

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn production_registry_partition_workflow_and_error_mapping() -> Result<()> {
    let mut fixture = LabFixture::create("application-registry")?;
    let disk = fixture
        .attach_sparse_loop("disk.img", 128 * 1024 * 1024)?
        .path()
        .to_string_lossy()
        .into_owned();
    let backend = Arc::new(UdisksBackend::from_connection(Arc::new(
        zbus::Connection::system().await?,
    )));
    let runtime = AppRuntime::with_udisks_backend(backend)
        .await
        .map_err(|e| e.to_string())?;
    assert!(!runtime.is_scenario());
    assert!(runtime.scenario_control().is_none());
    let operations = runtime.operations();
    for _ in 0..100 {
        if operations
            .registry
            .block
            .list_disks()
            .await?
            .iter()
            .any(|info| info.device == disk)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let disks = operations.registry.block.list_disks().await?;
    assert!(disks.iter().any(|info| info.device == disk));
    let client = PartitionsClient::with_operations(Arc::clone(&operations));
    let invalid = client
        .create_partition_table(&disk, "unsupported-table")
        .await
        .unwrap_err();
    assert!(matches!(invalid, OperationError::InvalidInput(_)));
    fixture.owned_loop(Path::new(&disk))?;
    client
        .create_partition_table(&disk, "GPT")
        .await
        .map_err(|e| e.to_string())?;
    assert!(
        client
            .list_partitions(&disk)
            .await
            .map_err(|e| e.to_string())?
            .is_empty()
    );
    let output = fixture
        .root()?
        .path()
        .join("backup.img")
        .to_string_lossy()
        .into_owned();
    fixture.owned_loop(Path::new(&disk))?;
    let copy = operations
        .image_manager
        .backup(
            Arc::clone(&operations.registry.block),
            disk.clone(),
            output.clone(),
        )
        .await
        .map_err(|e| e.to_string())?;
    operations
        .image_manager
        .wait(&copy)
        .await
        .map_err(|e| e.to_string())?;
    let status = operations
        .image_manager
        .status(&copy)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(status.total_bytes, 128 * 1024 * 1024);
    assert_eq!(status.bytes_completed, status.total_bytes);
    assert_eq!(std::fs::metadata(&output)?.len(), status.total_bytes);
    operations
        .image_manager
        .forget(&copy)
        .await
        .map_err(|e| e.to_string())?;
    assert!(matches!(
        operations.image_manager.status(&copy).await,
        Err(OperationError::MissingOperation(_))
    ));
    fixture.owned_loop(Path::new(&disk))?;
    let restore = operations
        .image_manager
        .restore(Arc::clone(&operations.registry.block), disk.clone(), output)
        .await
        .map_err(|e| e.to_string())?;
    operations
        .image_manager
        .wait(&restore)
        .await
        .map_err(|e| e.to_string())?;
    operations
        .image_manager
        .forget(&restore)
        .await
        .map_err(|e| e.to_string())?;
    fixture.owned_loop(Path::new(&disk))?;
    let cancelled_path = fixture.root()?.path().join("cancelled.img");
    let cancelled = operations
        .image_manager
        .backup(
            Arc::clone(&operations.registry.block),
            disk.clone(),
            cancelled_path.to_string_lossy().into_owned(),
        )
        .await
        .map_err(|e| e.to_string())?;
    operations
        .image_manager
        .cancel(&cancelled)
        .await
        .map_err(|e| e.to_string())?;
    assert!(
        operations
            .image_manager
            .wait(&cancelled)
            .await
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    assert!(
        !cancelled_path.exists(),
        "pre-start cancellation must not create a partial image"
    );
    operations
        .image_manager
        .forget(&cancelled)
        .await
        .map_err(|e| e.to_string())?;
    fixture.cleanup()?;
    Ok(())
}
