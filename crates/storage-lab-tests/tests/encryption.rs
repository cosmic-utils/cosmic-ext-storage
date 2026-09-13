mod common;
use common::{lab, owned};
use storage_contracts::EncryptionOperations;
use storage_lab_tests::Result;

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn luks_failure_after_format_cleans_auto_opened_mapper() -> Result<()> {
    let (fixture, backend, disk) = lab("luks-unwind", 128).await?;
    let root = fixture.root()?.path().to_owned();
    let ledger = fixture.ledger_path().to_owned();
    let outcome = tokio::spawn(async move {
        // UDisks creates a mapping internally during format; no successful
        // unlock return value is available to register during this unwind.
        backend
            .format_luks(
                owned(&fixture, &disk).unwrap(),
                "storage-lab-fake-secret",
                "luks2",
            )
            .await
            .unwrap();
        panic!("deliberate failure after successful LUKS format");
    })
    .await
    .expect_err("injected panic must reach the task boundary");
    assert!(outcome.is_panic());
    assert!(!root.exists(), "failed LUKS case retained its backing root");
    let evidence = std::fs::read_to_string(ledger)?;
    assert!(
        evidence.contains("rollback-dependent\t/dev/dm-"),
        "cleanup did not discover the auto-opened mapper"
    );
    assert!(!evidence.contains("cleanup-failed"));
    assert!(!evidence.contains("storage-lab-fake-secret"));
    Ok(())
}
