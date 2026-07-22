use crate::error::{Result, SysError};
use crate::rclone::RCloneCli;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

const SYSTEMD_UNIT_PREFIX: &str = "storage-rclone-mount";

fn unit_name(remote_name: &str) -> String {
    format!("{SYSTEMD_UNIT_PREFIX}@{remote_name}.service")
}
fn template_name() -> String {
    format!("{SYSTEMD_UNIT_PREFIX}@.service")
}

fn systemctl(home: &Path) -> Command {
    let mut command = Command::new("systemctl");
    command.arg("--user").env("HOME", home);
    command
}

fn run_systemctl(home: &Path, args: &[&str]) -> Result<String> {
    let output = systemctl(home).args(args).output().map_err(|error| {
        SysError::OperationFailed(format!("Failed to run systemctl --user: {error}"))
    })?;
    if !output.status.success() {
        return Err(SysError::OperationFailed(format!(
            "systemctl --user failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn template_contents(rclone: &Path, mkdir: &Path, fusermount: &Path) -> String {
    format!(
        "[Unit]\nDescription=RClone mount for %i\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStartPre={} -p %h/mnt/%i\nExecStart={} mount %i: %h/mnt/%i --config %h/.config/rclone/rclone.conf --vfs-cache-mode writes\nExecStop={} -u %h/mnt/%i\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=default.target\n",
        mkdir.display(),
        rclone.display(),
        fusermount.display()
    )
}

fn ensure_template(home: &Path) -> Result<PathBuf> {
    let directory = home.join(".config/systemd/user");
    std::fs::create_dir_all(&directory).map_err(SysError::Io)?;
    let path = directory.join(template_name());
    let rclone = RCloneCli::find_rclone_binary()?;
    let mkdir = which("mkdir").map_err(|error| SysError::OperationFailed(error.to_string()))?;
    let fusermount = which("fusermount3")
        .or_else(|_| which("fusermount"))
        .map_err(|error| SysError::OperationFailed(error.to_string()))?;
    let contents = template_contents(&rclone, &mkdir, &fusermount);
    if std::fs::read_to_string(&path).ok().as_deref() != Some(&contents) {
        std::fs::write(&path, contents).map_err(SysError::Io)?;
        run_systemctl(home, &["daemon-reload"])?;
    }
    Ok(path)
}

pub(crate) fn set_mount_on_login(remote_name: &str, enabled: bool, home: &Path) -> Result<()> {
    ensure_template(home)?;
    let unit = unit_name(remote_name);
    if enabled {
        run_systemctl(home, &["enable", "--now", &unit])?;
    } else {
        run_systemctl(home, &["disable", "--now", &unit])?;
    }
    Ok(())
}

pub(crate) fn is_mount_on_login_enabled(remote_name: &str, home: &Path) -> Result<bool> {
    let unit = unit_name(remote_name);
    let output = systemctl(home)
        .args(["is-enabled", &unit])
        .output()
        .map_err(|error| {
            SysError::OperationFailed(format!("Failed to run systemctl --user: {error}"))
        })?;
    Ok(output.status.success()
        && matches!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "enabled" | "enabled-runtime"
        ))
}
