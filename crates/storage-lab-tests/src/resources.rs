//! Capabilities for resources derived from the fixture's verified loops.
//! Registration is read-only; teardown revalidates ancestry before mutation.

use crate::{LabError, LabFixture, Result, run};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
pub struct LabMount {
    pub(crate) path: PathBuf,
    source: PathBuf,
    network: bool,
}
#[derive(Debug)]
pub struct LabMapper {
    pub(crate) path: PathBuf,
}
#[derive(Debug)]
pub struct LabVolumeGroup {
    pub(crate) name: String,
}
#[derive(Debug)]
pub struct LabArray {
    pub(crate) path: PathBuf,
}

impl LabMount {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl LabMapper {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl LabVolumeGroup {
    pub fn name(&self) -> &str {
        &self.name
    }
}
impl LabArray {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug)]
pub(crate) enum Resource {
    Mount(LabMount),
    Mapper(LabMapper),
    VolumeGroup(LabVolumeGroup),
    Array(LabArray),
}

impl LabFixture {
    pub fn prepare_array(&self, name: &str, members: &[crate::LabLoopDevice]) -> Result<()> {
        if members.len() < 2
            || name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(LabError::new(
                "array reservation requires a safe name and two owned loops",
            ));
        }
        let mut paths = Vec::new();
        for member in members {
            self.owned_loop(member.path())?;
            paths.push(member.path().to_string_lossy().into_owned());
        }
        paths.sort();
        if paths.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(LabError::new("array members must be distinct"));
        }
        if fs::read_to_string(self.ledger_path())?
            .lines()
            .any(|line| line.starts_with("reserved-array\t"))
        {
            return Err(LabError::new(
                "one array reservation per fixture is supported",
            ));
        }
        self.record(
            "reserved-array",
            Path::new(&format!("{name}\t{}", paths.join(","))),
        )
    }
    pub fn prepare_volume_group(&mut self, name: &str) -> Result<()> {
        validate_group_name(name)?;
        // Cleanup first checks every reported PV against this fixture; even a
        // same-name pre-existing host group can never be adopted or removed.
        self.resources
            .push(Resource::VolumeGroup(LabVolumeGroup { name: name.into() }));
        self.record("reserved-volume-group", Path::new(name))
    }

    pub fn prepare_network_mount(&mut self, name: &str) -> Result<PathBuf> {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(LabError::new(
                "network mount name must be a single safe component",
            ));
        }
        let path = self.root()?.path().join("mnt").join(name);
        fs::create_dir_all(&path)?;
        self.resources.push(Resource::Mount(LabMount {
            path: path.clone(),
            source: PathBuf::from(format!("{name}:")),
            network: true,
        }));
        self.record("reserved-network-mount", &path)?;
        Ok(path)
    }

    /// Reserve cleanup before asking the production adapter to mount. A failed
    /// mount call may still leave a live mount, so registering afterwards alone
    /// is not sufficient for failure safety.
    pub fn prepare_mount(&mut self, device: &crate::LabLoopDevice, name: &str) -> Result<PathBuf> {
        self.owned_loop(device.path())?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(LabError::new("mount name must be a single safe component"));
        }
        let path = self.root()?.path().join(name);
        fs::create_dir(&path)?;
        self.resources.push(Resource::Mount(LabMount {
            path: path.clone(),
            source: device.path().to_owned(),
            network: false,
        }));
        self.record("reserved-mount", &path)?;
        Ok(path)
    }

    /// UDisks can create a cleartext mapping and then fail before returning its
    /// name. Discover only children of a still-owned loop as a rollback net.
    /// Never traverse upwards from unrelated devices or use remove-all flags.
    pub(crate) fn cleanup_dependents(&self, device: &Path, depth: usize) -> Result<()> {
        if depth > 16 {
            return Err(LabError::new("cleanup ancestry is too deep"));
        }
        self.verify_derived_device(device)?;
        let name = device
            .file_name()
            .ok_or_else(|| LabError::new("missing device name"))?;
        let sys = Path::new("/sys/class/block").join(name);
        for holder in fs::read_dir(sys.join("holders"))? {
            let holder = holder?;
            let path = Path::new("/dev").join(holder.file_name());
            self.cleanup_dependents(&path, depth + 1)?;
            let kind = holder.file_name().to_string_lossy().into_owned();
            let resource = if kind.starts_with("dm-") {
                Resource::Mapper(LabMapper { path: path.clone() })
            } else if kind.starts_with("md") {
                Resource::Array(LabArray { path: path.clone() })
            } else {
                return Err(LabError::new(
                    "unknown dependent block device; refusing teardown",
                ));
            };
            self.record("rollback-dependent", &path)?;
            self.cleanup_resource(&resource)?;
        }
        // Partition holder directories are below the parent in canonical sysfs.
        for child in fs::read_dir(&sys)? {
            let child = child?;
            if child.path().join("partition").is_file() {
                self.cleanup_dependents(&Path::new("/dev").join(child.file_name()), depth + 1)?;
            }
        }
        Ok(())
    }

    /// Register an observed mount only when both source ancestry and exact
    /// mount target belong to this fixture. This never adopts a host mount.
    pub fn track_mount(&mut self, path: &Path) -> Result<&LabMount> {
        let path = fs::canonicalize(path)?;
        if !path.starts_with(self.root()?.path()) || path == self.root()?.path() {
            return Err(LabError::new("mount point must be below this fixture root"));
        }
        let source =
            mount_source(&path)?.ok_or_else(|| LabError::new("mount point is not mounted"))?;
        self.verify_derived_device(&source)?;
        self.resources.push(Resource::Mount(LabMount {
            path: path.clone(),
            source,
            network: false,
        }));
        self.record("mount", &path)?;
        match self.resources.last().unwrap() {
            Resource::Mount(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn track_mapper(&mut self, path: &Path) -> Result<&LabMapper> {
        let path = fs::canonicalize(path)?;
        if !path.to_string_lossy().starts_with("/dev/dm-") {
            return Err(LabError::new("mapper must resolve to a device-mapper node"));
        }
        self.verify_derived_device(&path)?;
        self.resources
            .push(Resource::Mapper(LabMapper { path: path.clone() }));
        self.record("mapper", &path)?;
        match self.resources.last().unwrap() {
            Resource::Mapper(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn track_array(&mut self, path: &Path) -> Result<&LabArray> {
        let path = fs::canonicalize(path)?;
        let name = path.file_name().unwrap().to_string_lossy();
        if !name.strip_prefix("md").is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            return Err(LabError::new("array must resolve to an MD device"));
        }
        self.verify_derived_device(&path)?;
        self.resources
            .push(Resource::Array(LabArray { path: path.clone() }));
        self.record("array", &path)?;
        match self.resources.last().unwrap() {
            Resource::Array(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    pub fn track_volume_group(&mut self, name: &str) -> Result<&LabVolumeGroup> {
        validate_group_name(name)?;
        self.verify_group(name, false)?;
        self.resources
            .push(Resource::VolumeGroup(LabVolumeGroup { name: name.into() }));
        self.record("volume-group", Path::new(name))?;
        match self.resources.last().unwrap() {
            Resource::VolumeGroup(value) => Ok(value),
            _ => unreachable!(),
        }
    }

    /// Walk every slave, not merely one: a mixed host/lab array is forbidden.
    pub fn verify_derived_device(&self, path: &Path) -> Result<()> {
        self.verify_ancestry(path, 0)
    }

    fn verify_ancestry(&self, path: &Path, depth: usize) -> Result<()> {
        if depth > 16 {
            return Err(LabError::new("device ancestry is too deep"));
        }
        if self.owned_loop(path).is_ok() {
            return Ok(());
        }
        let name = path
            .strip_prefix("/dev/")
            .map_err(|_| LabError::new("not a device path"))?;
        if name.components().count() != 1 {
            return Err(LabError::new("non-canonical device path"));
        }
        let sys = Path::new("/sys/class/block").join(name);
        // Partitions are children of a loop in sysfs, without slave entries.
        if sys.join("partition").is_file() {
            let real = fs::canonicalize(&sys)?;
            let parent = real
                .parent()
                .and_then(Path::file_name)
                .ok_or_else(|| LabError::new("partition has no parent"))?;
            return self.verify_ancestry(&Path::new("/dev").join(parent), depth + 1);
        }
        let slaves = fs::read_dir(sys.join("slaves"))?.collect::<std::io::Result<Vec<_>>>()?;
        if slaves.is_empty() {
            return Err(LabError::new("device has no ledgered ancestry"));
        }
        for slave in slaves {
            self.verify_ancestry(&Path::new("/dev").join(slave.file_name()), depth + 1)?;
        }
        Ok(())
    }

    fn verify_group(&self, name: &str, allow_absent: bool) -> Result<bool> {
        validate_group_name(name)?;
        let output = run(Command::new("pvs").args([
            "--noheadings",
            "--separator",
            "|",
            "-o",
            "pv_name,vg_name",
        ]))?;
        let output = String::from_utf8_lossy(&output.stdout);
        let mut found = false;
        for line in output.lines() {
            let Some((device, group)) = line.split_once('|') else {
                continue;
            };
            if group.trim() == name {
                self.verify_derived_device(Path::new(device.trim()))?;
                found = true;
            }
        }
        if !found && !allow_absent {
            return Err(LabError::new("volume group has no owned physical volumes"));
        }
        Ok(found)
    }

    pub(crate) fn cleanup_resource(&self, resource: &Resource) -> Result<()> {
        match resource {
            Resource::Mount(mount) => {
                if let Some(source) = mount_source(&mount.path)? {
                    if source != mount.source {
                        return Err(LabError::new("mount source changed since registration"));
                    }
                    if !mount.network {
                        self.verify_derived_device(&source)?;
                    }
                    run(Command::new("umount").arg("--").arg(&mount.path))?;
                }
            }
            Resource::Mapper(mapper) => {
                if sysfs_present(&mapper.path) {
                    let sys = Path::new("/sys/class/block").join(mapper.path.file_name().unwrap());
                    let identity = fs::read_to_string(sys.join("dm/uuid"))?;
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                    loop {
                        // UDisks/udev can briefly keep a freshly formatted mapper
                        // open. Retry only EBUSY, never force or defer removal;
                        // recheck both identity and every owned slave each time.
                        if !sysfs_present(&mapper.path) {
                            break;
                        }
                        self.verify_derived_device(&mapper.path)?;
                        if fs::read_to_string(sys.join("dm/uuid"))? != identity {
                            return Err(LabError::new("mapper identity changed during cleanup"));
                        }
                        match run(Command::new("dmsetup")
                            .env("LC_ALL", "C")
                            .arg("remove")
                            .arg(&mapper.path))
                        {
                            Ok(_) => break,
                            Err(error)
                                if error.to_string().contains("Device or resource busy")
                                    && std::time::Instant::now() < deadline =>
                            {
                                self.record("retry-busy-mapper", &mapper.path)?;
                                std::thread::sleep(std::time::Duration::from_millis(25));
                            }
                            Err(error) => return Err(error),
                        }
                    }
                }
            }
            Resource::Array(array) => {
                if sysfs_present(&array.path) {
                    self.verify_derived_device(&array.path)?;
                    let missing_node = !array.path.exists();
                    if missing_node {
                        let numbers = fs::read_to_string(
                            Path::new("/sys/class/block")
                                .join(array.path.file_name().unwrap())
                                .join("dev"),
                        )?;
                        let (major, minor) = numbers
                            .trim()
                            .split_once(':')
                            .ok_or_else(|| LabError::new("invalid array device number"))?;
                        self.record("cleanup-array-node", &array.path)?;
                        run(Command::new("mknod")
                            .arg(&array.path)
                            .args(["b", major, minor]))?;
                    }
                    run(Command::new("mdadm").arg("--stop").arg(&array.path))?;
                    if missing_node {
                        fs::remove_file(&array.path)?;
                    }
                }
            }
            Resource::VolumeGroup(group) => {
                if self.verify_group(&group.name, true)? {
                    run(Command::new("vgremove").args(["--yes", &group.name]))?;
                }
            }
        }
        Ok(())
    }
}

fn sysfs_present(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| Path::new("/sys/class/block").join(name).exists())
}

fn mount_source(path: &Path) -> Result<Option<PathBuf>> {
    let output = Command::new("findmnt")
        .args([
            "--noheadings",
            "--raw",
            "--output",
            "SOURCE",
            "--mountpoint",
        ])
        .arg(path)
        .output()?;
    if output.status.code() == Some(1) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(LabError::new("could not query mount source"));
    }
    let source = String::from_utf8_lossy(&output.stdout);
    Ok(Some(if source.trim().starts_with("/dev/") {
        fs::canonicalize(source.trim())?
    } else {
        PathBuf::from(source.trim())
    }))
}

fn validate_group_name(name: &str) -> Result<()> {
    if !name.starts_with("storage-lab-")
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(LabError::new("volume group must use a storage-lab- name"));
    }
    Ok(())
}

#[path = "../tests/unit/resources_tests.rs"]
#[cfg(test)]
mod tests;
