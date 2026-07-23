use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::path::Path;

pub use storage_types::{KillResult, ProcessInfo};

/// Find all processes that have open file descriptors pointing to the given mount point.
///
/// This function uses the procfs crate to enumerate all running processes and check
/// their open file descriptors. Any process with an fd pointing to a path under the
/// mount point is included in the results.
///
/// # Arguments
/// * `mount_point` - The mount point path to search for (e.g., "/mnt/mydisk")
///
/// # Returns
/// A vector of `ProcessInfo` structs, one for each process holding the mount open.
/// Returns an empty vector if:
/// - No processes are found using the mount point
/// - The mount point is invalid (empty or not absolute path)
/// - /proc cannot be accessed or enumerated
///
/// # Errors
/// Returns an error only if the tokio blocking task fails to spawn.
pub async fn find_processes_using_mount(mount_point: &str) -> Result<Vec<ProcessInfo>> {
    let mount_point = mount_point.to_string();

    // Spawn blocking task since procfs operations are synchronous
    tokio::task::spawn_blocking(move || find_processes_using_mount_sync(&mount_point))
        .await
        .context("Failed to spawn blocking task")?
}

/// Synchronous implementation of process discovery
fn find_processes_using_mount_sync(mount_point: &str) -> Result<Vec<ProcessInfo>> {
    // Input validation
    let trimmed = mount_point.trim();
    if trimmed.is_empty() {
        tracing::warn!("Empty mount point provided, returning no processes");
        return Ok(Vec::new());
    }

    if !trimmed.starts_with('/') {
        tracing::warn!(
            mount_point = %mount_point,
            "Mount point is not an absolute path, returning no processes"
        );
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    let mount_path = Path::new(trimmed);

    tracing::debug!("Searching for processes using mount point: {}", trimmed);

    // Build UID map once for efficient username lookups
    let uid_map = build_uid_map();

    // Enumerate all processes
    let all_procs = match procfs::process::all_processes() {
        Ok(procs) => procs,
        Err(e) => {
            tracing::warn!("Failed to enumerate processes: {}", e);
            return Ok(Vec::new()); // Return empty vec instead of error
        }
    };

    for proc_result in all_procs {
        let process = match proc_result {
            Ok(p) => p,
            Err(_) => continue, // Process vanished, skip silently
        };

        let pid = process.pid();

        // Check if any file descriptor points to the mount point
        let has_open_fd = match check_process_fds(&process, mount_path) {
            Ok(has_fd) => has_fd,
            Err(_) => continue, // Permission denied or process vanished
        };

        if !has_open_fd {
            continue;
        }

        // Extract process information
        let command = extract_command(&process);
        let (uid, username) = extract_user_info(&process, &uid_map);

        tracing::debug!(
            "Found process using mount: PID={}, command={}, user={}",
            pid,
            command,
            username
        );

        result.push(ProcessInfo {
            pid,
            command,
            uid,
            username,
        });
    }

    tracing::info!(
        "Found {} process(es) using mount point: {}",
        result.len(),
        mount_point
    );

    Ok(result)
}

/// Kill multiple processes by sending SIGKILL.
///
/// This function attempts to terminate the specified processes immediately using SIGKILL.
/// It performs safety checks to prevent killing system-critical processes.
///
/// # Arguments
/// * `pids` - Slice of process IDs to terminate
///
/// # Returns
/// A vector of `KillResult` structs indicating success/failure for each PID.
///
/// # Safety
/// - Refuses to kill PID 0 or 1 (special processes: scheduler, init)
/// - Refuses to kill negative PIDs (process groups - requires separate handling)
/// - Returns success for ESRCH (process not found) - process already gone
/// - Returns error for EPERM (permission denied) - user doesn't own the process
///
/// # Notes
/// This function is synchronous and does not spawn a blocking task. The kill syscall
/// completes quickly, so async wrapping is unnecessary.
pub fn kill_processes(pids: &[i32]) -> Vec<KillResult> {
    let mut results = Vec::new();

    for &pid in pids {
        // Safety check: reject negative PIDs (process groups)
        if pid < 0 {
            tracing::warn!("Refusing to kill negative PID {} (process group)", pid);
            results.push(KillResult {
                pid,
                success: false,
                error: Some("Process groups not supported (negative PID)".to_string()),
            });
            continue;
        }

        // Safety check: never kill init or kernel processes
        if pid <= 1 {
            tracing::warn!("Refusing to kill system process with PID {}", pid);
            results.push(KillResult {
                pid,
                success: false,
                error: Some("Refusing to kill system process".to_string()),
            });
            continue;
        }

        tracing::debug!("Attempting to kill process: PID={}", pid);

        // Send SIGKILL to the process
        match kill(Pid::from_raw(pid), Signal::SIGKILL) {
            Ok(()) => {
                tracing::info!("Successfully killed process: PID={}", pid);
                results.push(KillResult {
                    pid,
                    success: true,
                    error: None,
                });
            }
            Err(nix::Error::ESRCH) => {
                // Process doesn't exist - treat as success (already gone)
                tracing::debug!("Process {} not found (already terminated)", pid);
                results.push(KillResult {
                    pid,
                    success: true,
                    error: None,
                });
            }
            Err(nix::Error::EPERM) => {
                // Permission denied - user doesn't own the process
                tracing::warn!("Permission denied when killing process {}", pid);
                results.push(KillResult {
                    pid,
                    success: false,
                    error: Some("Permission denied".to_string()),
                });
            }
            Err(e) => {
                // Other error
                tracing::error!("Failed to kill process {}: {}", pid, e);
                results.push(KillResult {
                    pid,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    results
}

/// Check if a process has any file descriptors pointing to paths under the mount point
fn check_process_fds(process: &procfs::process::Process, mount_path: &Path) -> Result<bool> {
    let fds = process.fd().context("Failed to read file descriptors")?;

    for fd_result in fds {
        let fd = match fd_result {
            Ok(f) => f,
            Err(_) => continue, // FD vanished or permission denied
        };

        // Get the target path of this fd - note: target is a field, not a method
        let target = match &fd.target {
            procfs::process::FDTarget::Path(path) => path,
            _ => continue, // Not a regular file path (socket, pipe, etc.)
        };

        // Check if this path is under the mount point
        if target.starts_with(mount_path) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Extract command name from process
fn extract_command(process: &procfs::process::Process) -> String {
    // Try cmdline first (full command with args)
    if let Ok(cmdline) = process.cmdline()
        && !cmdline.is_empty()
    {
        // Use first element (executable path)
        let cmd = &cmdline[0];
        // Strip path, just return the basename
        return Path::new(cmd)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(cmd)
            .to_string();
    }

    // Fallback to stat.comm (kernel thread name or process name)
    if let Ok(stat) = process.stat() {
        return stat.comm;
    }

    // Final fallback
    format!("<PID {}>", process.pid())
}

/// Extract UID and username from process
fn extract_user_info(
    process: &procfs::process::Process,
    uid_map: &HashMap<u32, String>,
) -> (u32, String) {
    // Get real UID from status
    if let Ok(status) = process.status() {
        let uid = status.ruid;

        // Look up username in the prebuilt map
        let username = uid_map
            .get(&uid)
            .cloned()
            .unwrap_or_else(|| uid.to_string());

        return (uid, username);
    }

    // Fallback
    (0, "root".to_string())
}

/// Build a UID to username map from /etc/passwd
/// This is more efficient than calling resolve_username repeatedly
fn build_uid_map() -> HashMap<u32, String> {
    let mut map = HashMap::new();

    match std::fs::read_to_string("/etc/passwd") {
        Ok(passwd_content) => {
            for line in passwd_content.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3
                    && let Ok(uid) = parts[2].parse::<u32>()
                {
                    map.insert(uid, parts[0].to_string());
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to read /etc/passwd for UID map: {}", e);
        }
    }

    map
}

#[cfg(test)]
mod tests {
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
}
