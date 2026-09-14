//! The real entrypoint's zero-match guard, exercised only inside the lab.
use std::process::Command;

#[test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
fn stale_inner_filter_fails_instead_of_running_zero_tests() {
    assert_eq!(std::env::var("STORAGE_LAB_PRIVATE").as_deref(), Ok("1"));
    let output = Command::new("/usr/local/bin/storage-lab-run-tests")
        .env("STORAGE_LAB_TEST_TARGET", "selection")
        .env(
            "STORAGE_LAB_TEST_FILTER",
            "missing_rstest_case_must_not_pass",
        )
        .output()
        .expect("real lab selection guard");
    assert_eq!(output.status.code(), Some(64));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Required storage-lab test not found: missing_rstest_case_must_not_pass")
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("test result: ok"));
}
