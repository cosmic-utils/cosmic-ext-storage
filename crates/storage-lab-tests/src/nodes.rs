//! Materialize only partition nodes descended from verified fixture loops.
//! Container udev can observe a partition without creating its /dev node.
use crate::{LabError, Result, loop_backing, run};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug)]
pub(crate) struct PartitionNodes {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<()>>>,
    created: Arc<Mutex<Vec<PathBuf>>>,
}

impl PartitionNodes {
    pub(crate) fn start(device: PathBuf, backing: PathBuf, ledger: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let created = Arc::new(Mutex::new(Vec::new()));
        let worker_created = created.clone();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                let sys = PathBuf::from("/sys/class/block").join(device.file_name().unwrap());
                for entry in fs::read_dir(&sys)? {
                    let entry = entry?;
                    if !entry.path().join("partition").is_file() {
                        continue;
                    }
                    let node = PathBuf::from("/dev").join(entry.file_name());
                    if node.exists() {
                        continue;
                    }
                    if loop_backing(&device)?.as_deref() != Some(backing.as_path()) {
                        return Err(LabError::new("partition parent lost fixture ownership"));
                    }
                    let numbers = match fs::read_to_string(entry.path().join("dev")) {
                        Ok(numbers) => numbers,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(error) => return Err(error.into()),
                    };
                    let Some((major, minor)) = numbers.trim().split_once(':') else {
                        return Err(LabError::new("invalid partition device number"));
                    };
                    let major: u32 = major.parse().map_err(|_| LabError::new("invalid major"))?;
                    let minor: u32 = minor.parse().map_err(|_| LabError::new("invalid minor"))?;
                    let mut evidence = OpenOptions::new().append(true).open(&ledger)?;
                    evidence.write_all(
                        format!(
                            "partition-node-intent\t{}\t{major}:{minor}\n",
                            node.display()
                        )
                        .as_bytes(),
                    )?;
                    evidence.sync_all()?;
                    run(Command::new("mknod").arg(&node).args([
                        "b",
                        &major.to_string(),
                        &minor.to_string(),
                    ]))?;
                    worker_created
                        .lock()
                        .map_err(|_| LabError::new("partition node ledger poisoned"))?
                        .push(node);
                }
                // MD creation can finish in the kernel before udev has made
                // either the node or the requested /dev/md/<name> alias.
                for holder in fs::read_dir(sys.join("holders"))? {
                    let holder = holder?;
                    let name = holder.file_name();
                    if !name.to_string_lossy().starts_with("md") {
                        continue;
                    }
                    let array_sys = PathBuf::from("/sys/class/block").join(&name);
                    let entries = match fs::read_dir(array_sys.join("slaves")) {
                        Ok(entries) => entries,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(error) => return Err(error.into()),
                    };
                    let mut members = entries
                        .map(|entry| {
                            entry.map(|entry| PathBuf::from("/dev").join(entry.file_name()))
                        })
                        .collect::<std::io::Result<Vec<_>>>()?;
                    members.sort();
                    // One worker owns materialisation for a multi-loop array.
                    if members.first() != Some(&device) {
                        continue;
                    }
                    let evidence = fs::read_to_string(&ledger)?;
                    for member in &members {
                        let member_backing = loop_backing(member)?
                            .ok_or_else(|| LabError::new("array member has no backing"))?;
                        if member_backing.parent() != backing.parent()
                            || !evidence
                                .lines()
                                .any(|line| line == format!("loop\t{}", member.display()))
                        {
                            return Err(LabError::new("array contains a non-ledgered member"));
                        }
                    }
                    let node = PathBuf::from("/dev").join(&name);
                    if !node.exists() {
                        let numbers = match fs::read_to_string(array_sys.join("dev")) {
                            Ok(numbers) => numbers,
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                            Err(error) => return Err(error.into()),
                        };
                        let (major, minor) = numbers
                            .trim()
                            .split_once(':')
                            .ok_or_else(|| LabError::new("invalid array device number"))?;
                        let mut journal = OpenOptions::new().append(true).open(&ledger)?;
                        journal.write_all(
                            format!("array-node-intent\t{}\t{major}:{minor}\n", node.display())
                                .as_bytes(),
                        )?;
                        journal.sync_all()?;
                        run(Command::new("mknod").arg(&node).args(["b", major, minor]))?;
                        worker_created
                            .lock()
                            .map_err(|_| LabError::new("node ledger poisoned"))?
                            .push(node.clone());
                    }
                    for line in evidence.lines() {
                        if let Some(reservation) = line.strip_prefix("reserved-array\t") {
                            let (alias, reserved_members) =
                                reservation.split_once('\t').ok_or_else(|| {
                                    LabError::new("array reservation lacks exact members")
                                })?;
                            let reserved_members: Vec<_> =
                                reserved_members.split(',').map(PathBuf::from).collect();
                            if members != reserved_members {
                                if members
                                    .iter()
                                    .any(|member| !reserved_members.contains(member))
                                {
                                    return Err(LabError::new(
                                        "array includes an unreserved member",
                                    ));
                                }
                                // Assembly and stop publish intermediate member
                                // sets. Do not grant an alias until complete.
                                continue;
                            }
                            if alias.is_empty()
                                || !alias
                                    .bytes()
                                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                            {
                                return Err(LabError::new("invalid array alias in ledger"));
                            }
                            fs::create_dir_all("/dev/md")?;
                            let link = PathBuf::from("/dev/md").join(alias);
                            let created = ensure_array_alias(&link, &node, || {
                                let mut journal = OpenOptions::new().append(true).open(&ledger)?;
                                journal.write_all(
                                    format!(
                                        "array-alias-intent\t{}\t{}\n",
                                        link.display(),
                                        node.display()
                                    )
                                    .as_bytes(),
                                )?;
                                journal.sync_all()?;
                                Ok(())
                            })?;
                            if created {
                                worker_created
                                    .lock()
                                    .map_err(|_| LabError::new("node ledger poisoned"))?
                                    .push(link);
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(2));
            }
            Ok(())
        });
        Self {
            stop,
            worker: Some(worker),
            created,
        }
    }

    pub(crate) fn finish(&mut self) -> Result<Vec<PathBuf>> {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| LabError::new("partition node worker panicked"))??;
        }
        Ok(std::mem::take(&mut *self.created.lock().map_err(|_| {
            LabError::new("partition node ledger poisoned")
        })?))
    }
}

fn ensure_array_alias(
    link: &Path,
    node: &Path,
    before_create: impl FnOnce() -> Result<()>,
) -> Result<bool> {
    match fs::read_link(link) {
        Ok(existing) if alias_target_matches(link, node, &existing) => return Ok(false),
        Ok(_) => return Err(LabError::new("array alias points to a different device")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    before_create()?;
    match std::os::unix::fs::symlink(node, link) {
        Ok(()) => Ok(true),
        // udev or the other owned member's worker can publish the same alias
        // between observation and creation. Never replace an existing entry.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if alias_target_matches(link, node, &fs::read_link(link)?) {
                Ok(false)
            } else {
                Err(LabError::new("array alias changed during creation"))
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn alias_target_matches(link: &Path, node: &Path, target: &Path) -> bool {
    let absolute = if target.is_absolute() {
        target.to_owned()
    } else {
        let Some(parent) = link.parent() else {
            return false;
        };
        parent.join(target)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return false;
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized == node
}

impl Drop for PartitionNodes {
    fn drop(&mut self) {
        // Fixture cleanup normally joins and records these nodes first.
        if let Err(error) = self.finish() {
            eprintln!("partition node worker: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alias_creation_handles_udev_races_without_overwriting_foreign_targets() {
        let fixture = crate::LabFixture::create("alias-race").unwrap();
        let root = fixture.root().unwrap().path();
        let target = root.join("missing-owned-node");
        let link = root.join("alias");
        assert!(ensure_array_alias(&link, &target, || Ok(())).unwrap());
        // read_link, unlike exists(), recognizes an already-created dangling alias.
        assert!(!ensure_array_alias(&link, &target, || panic!("must not recreate")).unwrap());
        fs::remove_file(&link).unwrap();
        assert!(
            !ensure_array_alias(&link, &target, || {
                std::os::unix::fs::symlink(&target, &link)?;
                Ok(())
            })
            .unwrap()
        );
        fs::remove_file(&link).unwrap();
        let foreign = root.join("foreign");
        assert!(
            ensure_array_alias(&link, &target, || {
                std::os::unix::fs::symlink(&foreign, &link)?;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(fs::read_link(&link).unwrap(), foreign);
        assert!(ensure_array_alias(&link, &target, || Ok(())).is_err());
        fs::remove_file(&link).unwrap();
        fs::write(&link, b"not an alias").unwrap();
        assert!(ensure_array_alias(&link, &target, || Ok(())).is_err());
        assert_eq!(fs::read(&link).unwrap(), b"not an alias");
    }

    #[test]
    fn relative_udev_aliases_must_resolve_to_the_exact_owned_node() {
        let fixture = crate::LabFixture::create("relative-alias").unwrap();
        let root = fixture.root().unwrap().path();
        fs::create_dir(root.join("md")).unwrap();
        let link = root.join("md/fixture");
        let node = root.join("md127");
        std::os::unix::fs::symlink("../md127", &link).unwrap();
        assert!(!ensure_array_alias(&link, &node, || panic!("must preserve udev alias")).unwrap());
        assert!(!alias_target_matches(&link, &node, Path::new("../md126")));
        assert!(!alias_target_matches(&link, &node, Path::new("/dev/sda")));
        assert!(!alias_target_matches(
            Path::new("/alias"),
            Path::new("/node"),
            Path::new("../../node")
        ));
    }
}
