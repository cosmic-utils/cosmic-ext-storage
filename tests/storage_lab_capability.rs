use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use testcontainers::{
    GenericImage, ImageExt,
    core::{ExecCommand, WaitFor},
    runners::SyncRunner,
};

const IMAGE_NAME: &str = "cosmic-storage-lab";
const IMAGE_TAG: &str = "spike";
const ARTIFACT_ROOT: &str = "target/storage-lab-prototype-artifacts";

#[test]
fn rejects_physical_device_patterns() {
    for device in ["/dev/sda", "/dev/sda1", "/dev/nvme0n1", "/dev/vda"] {
        assert!(
            !is_lab_loop_device(device),
            "physical device must never be accepted: {device}"
        );
    }
    assert!(is_lab_loop_device("/dev/loop42"));
}

/// This probe is opt-in because it requires the locally built privileged lab
/// image and a Docker daemon. CI runs it explicitly after building that image.
#[test]
#[ignore = "requires the locally built cosmic-storage-lab:spike Docker image"]
fn github_hosted_runner_supports_private_storage_lab() -> Result<(), Box<dyn Error>> {
    let artifact_dir = unique_artifact_dir()?;
    let image = GenericImage::new(IMAGE_NAME, IMAGE_TAG)
        .with_wait_for(WaitFor::message_on_stdout("STORAGE_LAB_READY"))
        .with_network("none")
        .with_privileged(true)
        .with_label("com.cosmic.storage.lab", "prototype");
    let container = image.start()?;

    let container_id = container.id().to_owned();
    let image_id = local_image_id()?;
    let mut probe = container.exec(ExecCommand::new(["sh", "-ec", PROBE_SCRIPT]))?;
    let stdout = String::from_utf8(probe.stdout_to_vec()?)?;
    let stderr = String::from_utf8(probe.stderr_to_vec()?)?;
    let exit_code = probe.exit_code()?;

    write_artifact(&artifact_dir, "container-id.txt", &container_id)?;
    write_artifact(&artifact_dir, "probe.stdout.log", &stdout)?;
    write_artifact(&artifact_dir, "probe.stderr.log", &stderr)?;

    let mut service_logs = container.exec(ExecCommand::new([
        "sh",
        "-ec",
        "cat /tmp/storage-lab/udisksd.log /tmp/storage-lab/udisks-ready.log",
    ]))?;
    let service_stdout = String::from_utf8(service_logs.stdout_to_vec()?)?;
    let service_stderr = String::from_utf8(service_logs.stderr_to_vec()?)?;
    write_artifact(&artifact_dir, "udisks.log", &service_stdout)?;
    write_artifact(&artifact_dir, "udisks.stderr.log", &service_stderr)?;

    let mut post_cleanup = container.exec(ExecCommand::new(["losetup", "-a"]))?;
    let post_cleanup_stdout = String::from_utf8(post_cleanup.stdout_to_vec()?)?;
    let post_cleanup_stderr = String::from_utf8(post_cleanup.stderr_to_vec()?)?;
    write_artifact(
        &artifact_dir,
        "post-cleanup-losetup.log",
        &post_cleanup_stdout,
    )?;
    write_artifact(
        &artifact_dir,
        "post-cleanup-losetup.stderr.log",
        &post_cleanup_stderr,
    )?;

    write_evidence(
        &artifact_dir,
        &container_id,
        &image_id,
        exit_code,
        &stdout,
        post_cleanup_stdout.as_str(),
    )?;
    assert_eq!(
        exit_code,
        Some(0),
        "storage-lab probe failed; inspect {}",
        artifact_dir.display()
    );
    assert!(stdout.contains("STORAGE_LAB_PROBE_OK"));
    assert!(stdout.contains("LOOP_DEVICE="));
    assert!(
        !post_cleanup_stdout.contains("/tmp/storage-lab/probe."),
        "a lab loop device survived cleanup; inspect {}",
        artifact_dir.display()
    );
    Ok(())
}

fn is_lab_loop_device(device: &str) -> bool {
    device.strip_prefix("/dev/loop").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn unique_artifact_dir() -> Result<PathBuf, Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory = Path::new(ARTIFACT_ROOT).join(format!("run-{nonce}"));
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

fn write_artifact(directory: &Path, name: &str, contents: &str) -> Result<(), Box<dyn Error>> {
    fs::write(directory.join(name), contents)?;
    Ok(())
}

fn write_evidence(
    directory: &Path,
    container_id: &str,
    image_id: &str,
    exit_code: Option<i64>,
    probe_stdout: &str,
    post_cleanup_losetup: &str,
) -> Result<(), Box<dyn Error>> {
    let loop_device = probe_stdout
        .lines()
        .find_map(|line| line.strip_prefix("LOOP_DEVICE="))
        .unwrap_or("<probe failed before a loop device was recorded>");
    let exit_code = exit_code.map_or_else(|| "null".to_owned(), |code| code.to_string());
    let evidence = format!(
        concat!(
            "{{\n",
            "  \"image\": \"{}:{}\",\n",
            "  \"image_id\": \"{}\",\n",
            "  \"container_id\": \"{}\",\n",
            "  \"network\": \"none\",\n",
            "  \"loop_device\": \"{}\",\n",
            "  \"probe_exit_code\": {},\n",
            "  \"post_cleanup_losetup_empty\": {}\n",
            "}}\n"
        ),
        IMAGE_NAME,
        IMAGE_TAG,
        image_id,
        container_id,
        loop_device,
        exit_code,
        post_cleanup_losetup.trim().is_empty()
    );
    write_artifact(directory, "evidence.json", &evidence)
}

fn local_image_id() -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "image",
            "inspect",
            "--format",
            "{{.Id}}",
            &format!("{IMAGE_NAME}:{IMAGE_TAG}"),
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "could not inspect {IMAGE_NAME}:{IMAGE_TAG}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

const PROBE_SCRIPT: &str = r#"
set -eu
lab_root=$(mktemp -d /tmp/storage-lab/probe.XXXXXX)
image="$lab_root/disk.img"
truncate -s 64M "$image"
created_devices=""
for index in $(seq 0 15); do
    device="/dev/loop$index"
    if [ ! -e "$device" ]; then
        mknod "$device" b 7 "$index"
        created_devices="$created_devices $device"
    fi
done
loop_device=$(losetup --find --show "$image")
printf '%s\n' "$loop_device" >"$lab_root/loop-ledger.txt"
cleanup() {
    losetup --detach "$loop_device" || true
    rm -rf "$lab_root"
    rm -f $created_devices
}
trap cleanup EXIT
test -b "$loop_device"
dbus-send --system --dest=org.freedesktop.UDisks2 --print-reply \
    /org/freedesktop/UDisks2 org.freedesktop.DBus.Peer.Ping >/dev/null
losetup --detach "$loop_device"
test ! -e "/sys/class/block/${loop_device##*/}"
rm -rf "$lab_root"
trap - EXIT
printf 'LOOP_DEVICE=%s\n' "$loop_device"
printf '%s\n' 'STORAGE_LAB_PROBE_OK'
"#;
