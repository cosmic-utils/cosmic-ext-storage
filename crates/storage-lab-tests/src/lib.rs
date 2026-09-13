//! Safety-first fixtures for the private storage integration lab.
//!
//! The fixture only creates file-backed loop devices below its own temporary
//! root. It never accepts an arbitrary block device as a mutation target.

use std::{
    error::Error,
    fmt,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LAB_ROOT_PARENT: &str = "/tmp/storage-lab";
const LOOP_NODE_LIMIT: u32 = 16;
const DETACH_ATTEMPTS: u32 = 100;
const DETACH_POLL: Duration = Duration::from_millis(100);

pub type Result<T> = std::result::Result<T, LabError>;

#[derive(Debug)]
pub struct LabError(String);

impl LabError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for LabError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for LabError {}

impl From<std::io::Error> for LabError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for LabError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

/// A container-local directory that owns every fixture resource below it.
#[derive(Debug)]
pub struct LabRoot {
    path: PathBuf,
}

impl LabRoot {
    pub fn create(label: &str) -> Result<Self> {
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(LabError::new(
                "lab root label must be ASCII alphanumeric or hyphen",
            ));
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| LabError::new(error.to_string()))?
            .as_nanos();
        let path = Path::new(LAB_ROOT_PARENT).join(format!("{label}-{nonce}"));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sparse_file(&self, name: &str, bytes: u64) -> Result<LabBackingFile> {
        if name.contains('/') || name.is_empty() {
            return Err(LabError::new(
                "backing file name must be a single path component",
            ));
        }
        let path = self.path.join(name);
        File::create(&path)?.set_len(bytes)?;
        Ok(LabBackingFile { path })
    }

    pub fn remove(self) -> Result<()> {
        fs::remove_dir_all(self.path)?;
        Ok(())
    }
}

/// A sparse file that can be attached only through this fixture.
#[derive(Debug)]
pub struct LabBackingFile {
    path: PathBuf,
}

impl LabBackingFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A loop device allocated from a [`LabBackingFile`].
#[derive(Debug)]
pub struct LabLoopDevice {
    path: PathBuf,
    created_nodes: Vec<PathBuf>,
}

impl LabLoopDevice {
    pub fn attach(backing: &LabBackingFile) -> Result<Self> {
        let created_nodes = create_missing_loop_nodes()?;
        let attach_result = run(Command::new("losetup").args([
            "--find",
            "--show",
            backing.path().to_string_lossy().as_ref(),
        ]));
        let output = match attach_result {
            Ok(output) => output,
            Err(error) => {
                remove_created_nodes(&created_nodes);
                return Err(error);
            }
        };
        let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        if !is_lab_loop_device(path.to_string_lossy().as_ref()) {
            remove_created_nodes(&created_nodes);
            return Err(LabError::new(format!(
                "losetup returned a non-loop device: {}",
                path.display()
            )));
        }
        Ok(Self {
            path,
            created_nodes,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn detach(self) -> Result<()> {
        let detach_result =
            run(Command::new("losetup").args(["--detach", self.path.to_string_lossy().as_ref()]));
        let wait_result = wait_for_detach(&self.path);
        remove_created_nodes(&self.created_nodes);
        detach_result?;
        wait_result
    }
}

/// A ledgered group of resources that can be torn down after a failed test.
#[derive(Debug)]
pub struct LabFixture {
    root: Option<LabRoot>,
    loop_device: Option<LabLoopDevice>,
}

impl LabFixture {
    pub fn create(label: &str) -> Result<Self> {
        Ok(Self {
            root: Some(LabRoot::create(label)?),
            loop_device: None,
        })
    }

    pub fn attach_sparse_loop(&mut self, name: &str, bytes: u64) -> Result<&LabLoopDevice> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| LabError::new("lab fixture has already been cleaned up"))?;
        if self.loop_device.is_some() {
            return Err(LabError::new("this fixture already owns a loop device"));
        }
        let backing = root.sparse_file(name, bytes)?;
        self.loop_device = Some(LabLoopDevice::attach(&backing)?);
        self.write_ledger()?;
        self.loop_device
            .as_ref()
            .ok_or_else(|| LabError::new("loop device was not recorded"))
    }

    pub fn root(&self) -> Result<&LabRoot> {
        self.root
            .as_ref()
            .ok_or_else(|| LabError::new("lab fixture has already been cleaned up"))
    }

    pub fn cleanup(&mut self) -> Result<()> {
        let loop_result = self
            .loop_device
            .take()
            .map_or(Ok(()), LabLoopDevice::detach);
        let root_result = self.root.take().map_or(Ok(()), LabRoot::remove);
        loop_result.and(root_result)
    }

    fn write_ledger(&self) -> Result<()> {
        let root = self.root()?;
        let loop_device = self
            .loop_device
            .as_ref()
            .ok_or_else(|| LabError::new("cannot write a ledger without a loop device"))?;
        fs::write(
            root.path().join("ledger.txt"),
            format!("loop_device={}\n", loop_device.path().display()),
        )?;
        Ok(())
    }
}

impl Drop for LabFixture {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

pub fn is_lab_loop_device(path: &str) -> bool {
    path.strip_prefix("/dev/loop").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn create_missing_loop_nodes() -> Result<Vec<PathBuf>> {
    let mut created = Vec::new();
    for index in 0..LOOP_NODE_LIMIT {
        let path = PathBuf::from(format!("/dev/loop{index}"));
        if path.exists() {
            continue;
        }
        run(Command::new("mknod").args([
            path.to_string_lossy().as_ref(),
            "b",
            "7",
            &index.to_string(),
        ]))?;
        created.push(path);
    }
    Ok(created)
}

fn wait_for_detach(loop_device: &Path) -> Result<()> {
    for _ in 0..DETACH_ATTEMPTS {
        let output = Command::new("losetup")
            .args([
                "--list",
                "--noheadings",
                "--output",
                "BACK-FILE",
                loop_device.to_string_lossy().as_ref(),
            ])
            .output()?;
        if output.status.success() && String::from_utf8_lossy(&output.stdout).trim().is_empty() {
            return Ok(());
        }
        thread::sleep(DETACH_POLL);
    }
    Err(LabError::new(format!(
        "loop device still has a backing file after detach: {}",
        loop_device.display()
    )))
}

fn remove_created_nodes(nodes: &[PathBuf]) {
    for node in nodes {
        let _ = fs::remove_file(node);
    }
}

fn run(command: &mut Command) -> Result<Output> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(output);
    }
    Err(LabError::new(format!(
        "command failed with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::is_lab_loop_device;

    #[test]
    fn rejects_non_loop_and_physical_device_patterns() {
        for device in [
            "/dev/sda",
            "/dev/sda1",
            "/dev/nvme0n1",
            "/dev/vda",
            "/dev/loop",
        ] {
            assert!(!is_lab_loop_device(device), "must reject {device}");
        }
        assert!(is_lab_loop_device("/dev/loop42"));
    }
}
