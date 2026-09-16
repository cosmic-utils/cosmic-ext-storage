//! Safety-first fixtures for the private storage integration lab.
//!
//! The fixture only creates file-backed loop devices below its own temporary
//! root. It never accepts an arbitrary block device as a mutation target.

use std::{
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LAB_ROOT_PARENT: &str = "/tmp/storage-lab";
const LOOP_NODE_LIMIT: u32 = 16;
const DETACH_ATTEMPTS: u32 = 100;
const DETACH_POLL: Duration = Duration::from_millis(100);
const EVIDENCE_ROOT: &str = "/tmp/storage-lab-evidence";

mod nodes;
#[cfg(feature = "outer-bridge")]
pub mod outer_bridge;
mod resources;
pub use resources::{LabArray, LabMapper, LabMount, LabVolumeGroup};

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

impl From<storage_contracts::StorageError> for LabError {
    fn from(error: storage_contracts::StorageError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<zbus::Error> for LabError {
    fn from(error: zbus::Error) -> Self {
        Self::new(error.to_string())
    }
}

/// A container-local directory that owns every fixture resource below it.
#[derive(Debug)]
pub struct LabRoot {
    path: PathBuf,
}

impl LabRoot {
    pub fn create(label: &str) -> Result<Self> {
        if label.is_empty()
            || !label
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
        fs::create_dir_all(LAB_ROOT_PARENT)?;
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sparse_file(&self, name: &str, bytes: u64) -> Result<LabBackingFile> {
        if name.is_empty()
            || name == "."
            || name == ".."
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(LabError::new(
                "backing file name must be a single safe ASCII path component",
            ));
        }
        let path = self.path.join(name);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .set_len(bytes)?;
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
#[derive(Debug, Clone)]
pub struct LabLoopDevice {
    path: PathBuf,
    backing: PathBuf,
    created_nodes: Vec<PathBuf>,
}

impl LabLoopDevice {
    fn attach(backing: &LabBackingFile) -> Result<Self> {
        require_private_lab()?;
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
            backing: backing.path.clone(),
            created_nodes,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn detach(&self) -> Result<()> {
        // A loop number can be reused. Never detach it unless its current
        // backing file still belongs to this exact capability.
        if loop_backing(&self.path)?.is_none() {
            remove_created_nodes(&self.created_nodes);
            return Ok(());
        }
        self.verify()?;
        let detach_result =
            run(Command::new("losetup").args(["--detach", self.path.to_string_lossy().as_ref()]));
        let wait_result = wait_for_detach(&self.path);
        detach_result?;
        wait_result?;
        remove_created_nodes(&self.created_nodes);
        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        require_private_lab()?;
        if loop_backing(&self.path)?.as_deref() != Some(self.backing.as_path()) {
            return Err(LabError::new(
                "loop backing no longer matches its owned capability",
            ));
        }
        Ok(())
    }
}

/// A ledgered group of resources that can be torn down after a failed test.
#[derive(Debug)]
pub struct LabFixture {
    root: Option<LabRoot>,
    loops: Vec<LabLoopDevice>,
    pending_images: Vec<PathBuf>,
    ledger: PathBuf,
    resources: Vec<resources::Resource>,
    partition_nodes: Vec<nodes::PartitionNodes>,
    created_partition_nodes: Vec<PathBuf>,
}

impl LabFixture {
    pub fn create(label: &str) -> Result<Self> {
        let root = LabRoot::create(label)?;
        fs::create_dir_all(EVIDENCE_ROOT)?;
        let ledger = Path::new(EVIDENCE_ROOT)
            .join(root.path().file_name().unwrap())
            .with_extension("ledger");
        let fixture = Self {
            root: Some(root),
            loops: Vec::new(),
            pending_images: Vec::new(),
            ledger,
            resources: Vec::new(),
            partition_nodes: Vec::new(),
            created_partition_nodes: Vec::new(),
        };
        fixture.record("root", fixture.root()?.path())?;
        Ok(fixture)
    }

    pub fn attach_sparse_loop(&mut self, name: &str, bytes: u64) -> Result<&LabLoopDevice> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| LabError::new("lab fixture has already been cleaned up"))?;
        let backing = root.sparse_file(name, bytes)?;
        self.record("backing", backing.path())?;
        self.loops.push(LabLoopDevice::attach(&backing)?);
        let device = self.loops.last().unwrap();
        for node in &device.created_nodes {
            self.record("node", node)?;
        }
        self.record("loop", device.path())?;
        self.partition_nodes.push(nodes::PartitionNodes::start(
            device.path.clone(),
            device.backing.clone(),
            self.ledger.clone(),
        ));
        self.loops
            .last()
            .ok_or_else(|| LabError::new("loop device was not recorded"))
    }

    pub fn root(&self) -> Result<&LabRoot> {
        self.root
            .as_ref()
            .ok_or_else(|| LabError::new("lab fixture has already been cleaned up"))
    }

    /// Exercise the real image-attach adapter with rollback registered before
    /// the async call. Even a failed or cancelled call can leave a kernel loop.
    pub async fn attach_image_loop(
        &mut self,
        backend: &impl storage_contracts::ImageDeviceOperations,
        name: &str,
        bytes: u64,
    ) -> Result<String> {
        require_private_lab()?;
        let backing = self.root()?.sparse_file(name, bytes)?;
        self.record("pending-image", backing.path())?;
        self.pending_images.push(backing.path.clone());
        self.created_partition_nodes
            .extend(create_missing_loop_nodes()?);
        let result = backend
            .loop_setup(
                backing
                    .path()
                    .to_str()
                    .ok_or_else(|| LabError::new("image path is not UTF-8"))?,
            )
            .await;
        // Reconcile regardless of adapter success; pending entries survive
        // until cleanup if the call failed before returning an object path.
        self.reconcile_images()?;
        let device = result?;
        self.owned_loop(Path::new(&device))?;
        Ok(device)
    }

    fn reconcile_images(&mut self) -> Result<()> {
        for backing in &self.pending_images {
            let output = run(Command::new("losetup")
                .args([
                    "--list",
                    "--noheadings",
                    "--raw",
                    "--output",
                    "NAME",
                    "--associated",
                ])
                .arg(backing))?;
            for name in String::from_utf8_lossy(&output.stdout).lines() {
                if !is_lab_loop_device(name) {
                    return Err(LabError::new(
                        "image reconciliation returned a non-loop device",
                    ));
                }
                let path = PathBuf::from(name);
                if self.loops.iter().any(|device| device.path == path) {
                    continue;
                }
                let device = LabLoopDevice {
                    path,
                    backing: backing.clone(),
                    created_nodes: Vec::new(),
                };
                device.verify()?;
                self.record("loop", device.path())?;
                self.partition_nodes.push(nodes::PartitionNodes::start(
                    device.path.clone(),
                    backing.clone(),
                    self.ledger.clone(),
                ));
                self.loops.push(device);
            }
        }
        Ok(())
    }

    pub fn cleanup(&mut self) -> Result<()> {
        // Keep failed entries and their backing files for a retry/diagnosis.
        // Unlinking a busy backing file hides a leaked kernel resource.
        if !self.pending_images.is_empty() {
            self.reconcile_images()?;
        }
        let mut worker_error = None;
        for worker in &mut self.partition_nodes {
            match worker.finish() {
                Ok(nodes) => self.created_partition_nodes.extend(nodes),
                Err(error) => {
                    worker_error.get_or_insert(error);
                    // Joining consumed the worker; a second finish recovers
                    // nodes created before its failure. Still tear down other
                    // independently verified resources, then report failure.
                    self.created_partition_nodes.extend(worker.finish()?);
                }
            }
        }
        self.partition_nodes.clear();
        while let Some(resource) = self.resources.last() {
            self.cleanup_resource(resource)?;
            self.record("removed-resource", Path::new(&format!("{resource:?}")))?;
            self.resources.pop();
        }
        while let Some(device) = self.loops.last() {
            if loop_backing(device.path())?.is_some() {
                self.cleanup_dependents(device.path(), 0)?;
            }
            device.detach()?;
            self.record("detached", device.path())?;
            self.loops.pop();
        }
        remove_created_nodes(&self.created_partition_nodes);
        self.created_partition_nodes.clear();
        self.pending_images.clear();
        if let Some(root) = &self.root {
            fs::remove_dir_all(root.path())?;
            self.record("removed-root", root.path())?;
            self.root = None;
        }
        match worker_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Read-only evidence survives successful cleanup of the fixture root.
    pub fn ledger_path(&self) -> &Path {
        &self.ledger
    }

    /// Resolve only a resource already held by this fixture, never a path
    /// merely resembling a loop device. Revalidate before each mutation.
    pub fn owned_loop(&self, path: &Path) -> Result<&LabLoopDevice> {
        let device = self
            .loops
            .iter()
            .find(|device| device.path == path)
            .ok_or_else(|| LabError::new("device is not in this fixture's ledger"))?;
        device.verify()?;
        Ok(device)
    }

    fn record(&self, action: &str, path: &Path) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger)?;
        file.write_all(format!("{action}\t{}\n", path.display()).as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}

impl Drop for LabFixture {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!(
                "storage lab cleanup failed: {error}; ledger: {}",
                self.ledger.display()
            );
            let _ = self.record("cleanup-failed", Path::new(&error.to_string()));
        }
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
        if let Err(error) = run(Command::new("mknod").args([
            path.to_string_lossy().as_ref(),
            "b",
            "7",
            &index.to_string(),
        ])) {
            remove_created_nodes(&created);
            return Err(error);
        }
        created.push(path);
    }
    Ok(created)
}

fn require_private_lab() -> Result<()> {
    if std::env::var("STORAGE_LAB_PRIVATE").as_deref() != Ok("1")
        || !Path::new("/run/storage-lab-private").is_file()
    {
        return Err(LabError::new(
            "device operations require the private storage lab",
        ));
    }
    Ok(())
}

fn loop_backing(device: &Path) -> Result<Option<PathBuf>> {
    let output = run(Command::new("losetup")
        .args(["--list", "--noheadings", "--raw", "--output", "BACK-FILE"])
        .arg(device))?;
    let value = String::from_utf8_lossy(&output.stdout);
    let value = value.trim();
    Ok((!value.is_empty()).then(|| PathBuf::from(value)))
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

#[path = "../tests/unit/lib_tests.rs"]
#[cfg(test)]
mod tests;
