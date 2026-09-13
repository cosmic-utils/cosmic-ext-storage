//! Run the existing bridge tests plus the LUKS probe inside a disposable VM.
#![cfg(feature = "vm-spike")]

// Reuse the actual bridge and its four existing tests, including evidence
// collection and fixture cleanup checks, rather than maintaining another one.
#[path = "bridge.rs"]
mod regular;

#[test]
#[ignore = "requires STORAGE_LAB=1 inside the disposable VM capability spike"]
fn luks_runs_in_a_guest_vm_through_testcontainers() -> Result<(), Box<dyn std::error::Error>> {
    regular::run_inner_test("luks_unlock_rejects_bad_secret_and_locks_cleanly")
}
