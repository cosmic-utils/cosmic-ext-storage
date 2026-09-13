use super::*;

#[tokio::test]
async fn find_processes_returns_empty_for_nonexistent_mount() {
    // This mount point shouldn't exist and shouldn't have any processes
    let result = find_processes_using_mount("/nonexistent/mount/point/12345")
        .await
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn find_processes_handles_proc_access() {
    // Test with /proc itself - should work even if no processes are using it
    let result = find_processes_using_mount("/proc").await;
    assert!(result.is_ok());
}

#[test]
fn kill_processes_rejects_system_pids() {
    // Test safety checks: negative PIDs and PIDs <= 1 should be rejected
    let results = kill_processes(&[0, 1, -1]);

    assert_eq!(results.len(), 3);

    // PID 0 and 1 should have "system" in error message
    for result in &results[0..2] {
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.as_ref().unwrap().contains("system"));
    }

    // PID -1 should have "process group" in error message
    assert!(!results[2].success);
    assert!(results[2].error.is_some());
    assert!(
        results[2].error.as_ref().unwrap().contains("process group")
            || results[2].error.as_ref().unwrap().contains("negative PID")
    );
}

#[test]
fn kill_processes_handles_nonexistent_pid() {
    // Very high PID unlikely to exist - should treat as success (ESRCH)
    let results = kill_processes(&[99999]);

    assert_eq!(results.len(), 1);
    // ESRCH should be treated as success (process already gone)
    assert!(
        results[0].success
            || results[0]
                .error
                .as_ref()
                .map(|e| e.contains("Permission"))
                .unwrap_or(false)
    );
}

#[test]
fn kill_processes_handles_invalid_negative_pid() {
    // Negative PIDs should be rejected
    let results = kill_processes(&[-5, -100]);

    assert_eq!(results.len(), 2);
    for result in &results {
        assert!(!result.success);
    }
}
