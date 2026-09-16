mod common;
use common::owned;
use storage_contracts::EncryptionOperations;
use storage_lab_tests::Result;

#[rstest::rstest]
#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn luks_failure_after_format_cleans_auto_opened_mapper(
    #[from(common::lab)]
    #[with("luks-unwind", 128)]
    #[future(awt)]
    lab: Result<common::OwnedLab>,
) -> Result<()> {
    let (fixture, backend, disk) = lab?;
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
        let holders = std::fs::read_dir(
            std::path::Path::new("/sys/class/block")
                .join(std::path::Path::new(&disk).file_name().unwrap())
                .join("holders"),
        )
        .unwrap();
        let mapper = holders
            .map(|entry| std::path::Path::new("/dev").join(entry.unwrap().file_name()))
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("dm-")
            })
            .expect("format must leave an auto-opened mapper");
        fixture.verify_derived_device(&mapper).unwrap();
        // Deterministically reproduce the short-lived open that udev/UDisks
        // probing caused on the hosted runner. Cleanup must wait, not force it.
        let probe = std::fs::File::open(mapper).unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            drop(probe);
        });
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
    assert!(evidence.contains("retry-busy-mapper\t/dev/dm-"));
    assert!(!evidence.contains("storage-lab-fake-secret"));
    Ok(())
}
