use std::{path::Path, sync::Arc, time::Duration};
use storage_contracts::DiskDiscovery;
use storage_lab_tests::{LabFixture, Result};
use storage_udisks::UdisksBackend;

pub async fn lab(label: &str, megabytes: u64) -> Result<(LabFixture, UdisksBackend, String)> {
    let mut fixture = LabFixture::create(label)?;
    let device = fixture
        .attach_sparse_loop("disk.img", megabytes * 1024 * 1024)?
        .path()
        .to_string_lossy()
        .into_owned();
    let backend = UdisksBackend::from_connection(Arc::new(zbus::Connection::system().await?));
    for _ in 0..100 {
        if backend
            .list_disks()
            .await?
            .iter()
            .any(|disk| disk.device == device)
        {
            return Ok((fixture, backend, device));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("owned loop did not appear in private UDisks"
        .to_owned()
        .into())
}

/// Revalidate kernel ancestry immediately before an adapter mutation.
pub fn owned<'a>(fixture: &LabFixture, device: &'a str) -> Result<&'a str> {
    fixture.verify_derived_device(Path::new(device))?;
    Ok(device)
}
