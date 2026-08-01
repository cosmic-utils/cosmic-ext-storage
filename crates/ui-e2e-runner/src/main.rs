//! Rust-owned UI E2E capability checks.
//!
//! This binary deliberately owns the compositor and probe-client lifecycle.
//! A passing capability check is required before case execution or PNG goldens
//! can be enabled in CI.

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io::Write,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use atspi::proxy::{accessible::ObjectRefExt, bus::StatusProxy};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const READY_TIMEOUT: Duration = Duration::from_secs(15);
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Parser)]
#[command(name = "ui-e2e-runner", about = "COSMIC Storage UI E2E test runner")]
struct Arguments {
    #[command(subcommand)]
    command: RunnerCommand,
}

#[derive(Debug, Subcommand)]
enum RunnerCommand {
    /// Prove that the pinned UI-test environment can run real E2E clients.
    Capability(CapabilityArguments),
    /// List legacy case IDs for the planning inventory only.
    ///
    /// This intentionally performs no UI action and never writes E2E evidence.
    ListCases(ListCasesArguments),
}

#[derive(Debug, Parser)]
struct CapabilityArguments {
    /// Test-feature application binary built inside the image.
    #[arg(long)]
    app: PathBuf,
    /// Checked-in scenario supplied to the app.
    #[arg(long)]
    scenario: PathBuf,
    /// Checked-in deterministic Sway configuration.
    #[arg(long)]
    sway_config: PathBuf,
    /// Writable directory for evidence from this one capability test.
    #[arg(long)]
    artifacts: PathBuf,
    /// Reviewed lock for the image, compositor, helper, and viewport contract.
    #[arg(long)]
    environment_lock: PathBuf,
}

#[derive(Debug, Parser)]
struct ListCasesArguments {
    /// Directory containing legacy planning manifests.
    #[arg(long)]
    root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct SwayOutput {
    name: String,
    active: bool,
    rect: SwayRect,
    scale: f64,
}

#[derive(Debug, Deserialize)]
struct SwayRect {
    width: u64,
    height: u64,
}

#[derive(Debug, Deserialize)]
struct EnvironmentLock {
    schema_version: u64,
    platform: String,
    base_image: String,
    containerfile_sha256: String,
    apt_snapshot: String,
    packages: Vec<LockedPackage>,
    fonts: Vec<LockedFont>,
    sway_command: Vec<String>,
    dbus_command: Vec<String>,
    renderer_env: BTreeMap<String, String>,
    home: String,
    xdg_config_home: String,
    xdg_cache_home: String,
    locale: String,
    locale_env: BTreeMap<String, String>,
    fontconfig_file: String,
    cosmic_theme_fixture: String,
    theme: String,
    viewport: LockedViewport,
    atspi_coordinate_space: String,
    image_comparison: ImageComparison,
    capture_helper: LockedHelper,
    input_helper: LockedHelper,
    toolchain: LockedToolchain,
}

#[derive(Debug, Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct LockedFont {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct LockedViewport {
    width: u64,
    height: u64,
    scale: f64,
}

#[derive(Debug, Deserialize)]
struct LockedHelper {
    name: String,
    version: String,
    protocol: String,
}

#[derive(Debug, Deserialize)]
struct LockedToolchain {
    rust: String,
    app_built_in_image: bool,
}

#[derive(Debug, Deserialize)]
struct ImageComparison {
    colorspace: String,
    channel_delta: u8,
    max_changed_pixel_fraction: String,
    masks: bool,
    edge_rounding: String,
}

#[derive(Debug, Serialize)]
struct CapabilityEvidence {
    status: &'static str,
    compositor: &'static str,
    environment: EnvironmentEvidence,
    viewport: ViewportEvidence,
    atspi: AtspiEvidence,
    application: ApplicationEvidence,
    input: InputEvidence,
    capture: CaptureEvidence,
}

#[derive(Debug, Serialize)]
struct EnvironmentEvidence {
    lock_sha256: String,
    base_image: String,
    apt_snapshot: String,
}

#[derive(Debug, Serialize)]
struct ViewportEvidence {
    output: String,
    width: u64,
    height: u64,
    scale: f64,
}

#[derive(Debug, Serialize)]
struct AtspiEvidence {
    connected: bool,
    registry_child_count: usize,
    application_names: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AccessibleNodeSnapshot {
    depth: usize,
    automation_id: String,
    role: String,
    name: String,
    description: String,
    states: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ApplicationEvidence {
    executable_sha256: String,
    scenario_sha256: String,
    visible_in_sway_tree: bool,
    scenario_marker_accessible: bool,
    atspi_tree_node_count: usize,
    atspi_non_window_node_count: usize,
    atspi_interactive_node_count: usize,
}

#[derive(Debug, Serialize)]
struct InputEvidence {
    helper: &'static str,
    probe_client_visible: bool,
    before_sha256: String,
    after_sha256: String,
}

#[derive(Debug, Serialize)]
struct CaptureEvidence {
    helper: &'static str,
    path: String,
    bytes: u64,
    png_signature_valid: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Arguments::parse()).await {
        eprintln!("ui-e2e-runner: {error:#}");
        std::process::exit(1);
    }
}

async fn run(arguments: Arguments) -> Result<()> {
    match arguments.command {
        RunnerCommand::Capability(arguments) => run_capability(arguments).await,
        RunnerCommand::ListCases(arguments) => {
            for identifier in list_case_ids(&arguments.root)? {
                println!("{identifier}");
            }
            Ok(())
        }
    }
}

async fn run_capability(arguments: CapabilityArguments) -> Result<()> {
    validate_input_file(&arguments.app, "application binary")?;
    validate_input_file(&arguments.scenario, "scenario")?;
    validate_input_file(&arguments.sway_config, "Sway configuration")?;
    let environment_lock = EnvironmentLock::load(&arguments.environment_lock)?;
    let environment_evidence = EnvironmentEvidence {
        lock_sha256: sha256_file(&arguments.environment_lock)?,
        base_image: environment_lock.base_image.clone(),
        apt_snapshot: environment_lock.apt_snapshot.clone(),
    };

    let mut session = CapabilitySession::new(
        &arguments.artifacts,
        &environment_lock,
        environment_evidence,
    )?;
    let outcome = session.run(&arguments).await;
    session.shutdown();
    outcome
}

impl EnvironmentLock {
    fn load(path: &Path) -> Result<Self> {
        validate_input_file(path, "environment lock")?;
        let lock: Self = toml::from_str(
            &fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("parse {}", path.display()))?;
        lock.validate(path)?;
        Ok(lock)
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if self.schema_version != 1 || self.platform != "linux/amd64" {
            bail!("unsupported E2E environment lock schema or platform");
        }
        if !self
            .base_image
            .starts_with("docker.io/library/rust@sha256:")
        {
            bail!("E2E environment lock must pin a Rust image by digest");
        }
        if !self
            .apt_snapshot
            .starts_with("https://snapshot.debian.org/archive/debian/")
        {
            bail!("E2E environment lock must use a Debian snapshot URL");
        }
        let containerfile = path
            .parent()
            .ok_or_else(|| anyhow!("environment lock has no parent directory"))?
            .join("Containerfile");
        if self.containerfile_sha256 != sha256_file(&containerfile)? {
            bail!(
                "environment lock Containerfile SHA-256 does not match {}",
                containerfile.display()
            );
        }
        if self.sway_command
            != [
                "sway",
                "--unsupported-gpu",
                "--config",
                "/workspace/tools/ui-testing/sway.conf",
            ]
        {
            bail!("E2E environment lock declares an unsupported Sway command");
        }
        if self.dbus_command != ["dbus-run-session", "--"] {
            bail!("E2E environment lock declares an unsupported D-Bus command");
        }
        if self.home != "<artifact-root>/home"
            || self.xdg_config_home != "<artifact-root>/config"
            || self.xdg_cache_home != "<artifact-root>/cache"
        {
            bail!("E2E environment lock must use private artifact-root XDG directories");
        }
        if self.locale != "C.UTF-8"
            || self.locale_env.get("LANG") != Some(&"C.UTF-8".to_owned())
            || self.locale_env.get("LC_ALL") != Some(&"C.UTF-8".to_owned())
            || self.locale_env.get("TZ") != Some(&"UTC".to_owned())
        {
            bail!("E2E environment lock declares an unsupported locale or timezone");
        }
        if self.fontconfig_file != "/etc/fonts/fonts.conf"
            || self.cosmic_theme_fixture != "/opt/ui-test/cosmic-theme.toml"
            || self.theme != "light"
            || self.atspi_coordinate_space != "logical_output_origin_0_0"
        {
            bail!("E2E environment lock declares unsupported deterministic UI settings");
        }
        if self.viewport.width != 1280 || self.viewport.height != 800 || self.viewport.scale != 1.0
        {
            bail!("E2E environment lock must declare a 1280x800 scale-1 viewport");
        }
        if self.capture_helper.name != "grim"
            || self.capture_helper.protocol != "zwlr_screencopy_manager_v1"
            || self.input_helper.name != "wtype"
            || self.input_helper.protocol != "zwp_virtual_keyboard_manager_v1"
            || !self.toolchain.app_built_in_image
            || self.toolchain.rust != "1.95.0"
        {
            bail!("E2E environment lock declares unsupported helper or toolchain settings");
        }
        if self.image_comparison.colorspace != "sRGB RGBA"
            || self.image_comparison.channel_delta != 2
            || self.image_comparison.max_changed_pixel_fraction != "0.0005"
            || self.image_comparison.masks
            || self.image_comparison.edge_rounding != "floor_start_ceil_end"
        {
            bail!("E2E environment lock declares unsupported image comparison settings");
        }
        for required in [
            ("EGL_PLATFORM", "wayland"),
            ("LIBGL_ALWAYS_SOFTWARE", "1"),
            ("WGPU_BACKEND", "vulkan"),
            ("WLR_BACKENDS", "headless"),
            ("WLR_LIBINPUT_NO_DEVICES", "1"),
            ("WLR_RENDERER", "pixman"),
            ("LIBSEAT_BACKEND", "noop"),
        ] {
            if self.renderer_env.get(required.0) != Some(&required.1.to_owned()) {
                bail!(
                    "E2E environment lock omits renderer setting {}={}",
                    required.0,
                    required.1
                );
            }
        }
        for package in &self.packages {
            let output = Command::new("dpkg-query")
                .args(["-W", "-f", "${Version}"])
                .arg(&package.name)
                .output()
                .with_context(|| format!("query locked package {}", package.name))?;
            if !output.status.success()
                || String::from_utf8_lossy(&output.stdout).trim() != package.version
            {
                bail!(
                    "locked package {} is not installed at version {}",
                    package.name,
                    package.version
                );
            }
        }
        for font in &self.fonts {
            if !font.path.is_file() || sha256_file(&font.path)? != font.sha256 {
                bail!("locked font hash does not match {}", font.path.display());
            }
        }
        if self.capture_helper.version != package_version(&self.packages, "grim")
            || self.input_helper.version != package_version(&self.packages, "wtype")
        {
            bail!("capture/input helper versions must match their locked packages");
        }
        Ok(())
    }
}

fn package_version<'a>(packages: &'a [LockedPackage], name: &str) -> &'a str {
    packages
        .iter()
        .find(|package| package.name == name)
        .map(|package| package.version.as_str())
        .unwrap_or("")
}

fn list_case_ids(root: &Path) -> Result<Vec<String>> {
    let mut paths = fs::read_dir(root)
        .with_context(|| format!("list legacy case directory {}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() {
        bail!(
            "legacy case directory contains no TOML manifests: {}",
            root.display()
        );
    }

    let mut identifiers = Vec::with_capacity(paths.len());
    for path in paths {
        let source =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let document: toml::Value =
            toml::from_str(&source).with_context(|| format!("parse {}", path.display()))?;
        if document
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            != Some(1)
        {
            bail!(
                "legacy case {} must declare schema_version = 1",
                path.display()
            );
        }
        let identifier = document
            .get("id")
            .and_then(toml::Value::as_str)
            .filter(|identifier| is_case_identifier(identifier))
            .ok_or_else(|| anyhow!("legacy case {} has an invalid id", path.display()))?;
        identifiers.push(identifier.to_owned());
    }
    let mut unique = identifiers.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != identifiers.len() {
        bail!(
            "legacy case directory contains duplicate case IDs: {}",
            root.display()
        );
    }
    Ok(identifiers)
}

fn is_case_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.len() <= 63
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn is_interactive_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "check box"
            | "combo box"
            | "list box"
            | "menu item"
            | "page tab"
            | "radio button"
            | "scroll bar"
            | "slider"
            | "spin button"
            | "switch"
            | "text"
    )
}

fn scenario_marker(path: &Path) -> Result<String> {
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let document: toml::Value =
        toml::from_str(&source).with_context(|| format!("parse scenario {}", path.display()))?;
    let identifier = document
        .get("id")
        .and_then(toml::Value::as_str)
        .filter(|identifier| !identifier.is_empty())
        .ok_or_else(|| anyhow!("scenario {} has no non-empty id", path.display()))?;
    Ok(format!(
        "Test scenario: {identifier} sha256:{}",
        sha256_file(path)?
    ))
}

struct CapabilitySession {
    artifacts: PathBuf,
    runtime: PathBuf,
    environment_evidence: EnvironmentEvidence,
    viewport: LockedViewport,
    environment: BTreeMap<OsString, OsString>,
    sway: Option<Child>,
    app: Option<Child>,
    foot: Option<Child>,
}

impl CapabilitySession {
    fn new(
        artifacts: &Path,
        environment_lock: &EnvironmentLock,
        environment_evidence: EnvironmentEvidence,
    ) -> Result<Self> {
        if artifacts.exists() {
            fs::remove_dir_all(artifacts)
                .with_context(|| format!("remove {}", artifacts.display()))?;
        }
        fs::create_dir_all(artifacts).with_context(|| format!("create {}", artifacts.display()))?;
        ensure_private_directory(artifacts)?;

        let runtime = artifacts.join("runtime");
        let home = artifacts.join("home");
        let config = artifacts.join("config");
        let cache = artifacts.join("cache");
        for directory in [&runtime, &home, &config, &cache] {
            fs::create_dir(directory).with_context(|| format!("create {}", directory.display()))?;
            ensure_private_directory(directory)?;
        }

        let mut environment = BTreeMap::from([
            (
                OsString::from("XDG_RUNTIME_DIR"),
                runtime.clone().into_os_string(),
            ),
            (OsString::from("HOME"), home.into_os_string()),
            (OsString::from("XDG_CONFIG_HOME"), config.into_os_string()),
            (OsString::from("XDG_CACHE_HOME"), cache.into_os_string()),
        ]);
        for (key, value) in environment_lock
            .renderer_env
            .iter()
            .chain(environment_lock.locale_env.iter())
        {
            environment.insert(OsString::from(key), OsString::from(value));
        }
        environment.insert(
            OsString::from("FONTCONFIG_FILE"),
            OsString::from(&environment_lock.fontconfig_file),
        );

        Ok(Self {
            artifacts: artifacts.to_path_buf(),
            runtime,
            environment_evidence,
            viewport: LockedViewport {
                width: environment_lock.viewport.width,
                height: environment_lock.viewport.height,
                scale: environment_lock.viewport.scale,
            },
            environment,
            sway: None,
            app: None,
            foot: None,
        })
    }

    async fn run(&mut self, arguments: &CapabilityArguments) -> Result<()> {
        self.start_sway(&arguments.sway_config)?;
        let (sway_socket, wayland_display) = self.wait_for_sway()?;
        self.environment.insert(
            OsString::from("WAYLAND_DISPLAY"),
            OsString::from(wayland_display),
        );
        let viewport = self.assert_viewport(&sway_socket)?;

        self.start_app(arguments)?;
        let app_visible =
            self.wait_for_tree_text(&sway_socket, "com.cosmic.ext.Storage", READY_TIMEOUT)?;
        if !app_visible {
            bail!("application was not visible in the Sway tree before readiness timeout");
        }
        self.enable_atspi().await?;
        let atspi = self.assert_atspi_application().await?;
        let atspi_tree = self.capture_atspi_tree().await?;
        let expected_marker = scenario_marker(&arguments.scenario)?;
        let scenario_marker_accessible = atspi_tree
            .iter()
            .filter(|node| {
                node.depth >= 2
                    && node.automation_id == "test.scenario"
                    && node.role == "paragraph"
                    && node.name == expected_marker
            })
            .count()
            == 1;
        if !scenario_marker_accessible {
            bail!(
                "application must expose exactly one test.scenario AT-SPI marker for the selected scenario"
            );
        }
        let atspi_interactive_node_count = atspi_tree
            .iter()
            .filter(|node| node.depth >= 2 && is_interactive_role(&node.role))
            .count();
        if atspi_interactive_node_count == 0 {
            bail!(
                "application exposes no interactive AT-SPI descendants; refusing to treat a window-only tree as E2E capability evidence"
            );
        }
        self.capture(&viewport.output, "application.png")?;

        self.start_input_probe()?;
        let probe_visible =
            self.wait_for_tree_text(&sway_socket, "ui-e2e-input-probe", PROBE_TIMEOUT)?;
        if !probe_visible {
            bail!("input probe client was not visible in the Sway tree");
        }
        self.capture(&viewport.output, "input-before.png")?;
        let before_sha256 = sha256_file(&self.artifacts.join("input-before.png"))?;
        self.run_checked("wtype", ["ui-e2e-keyboard-probe"])?;
        self.capture(&viewport.output, "input-after.png")?;
        let after_path = self.artifacts.join("input-after.png");
        let after_sha256 = sha256_file(&after_path)?;
        if before_sha256 == after_sha256 {
            bail!("virtual keyboard probe did not change the captured output");
        }

        let application = self.artifacts.join("application.png");
        let evidence = CapabilityEvidence {
            status: "passed",
            compositor: "sway",
            environment: EnvironmentEvidence {
                lock_sha256: self.environment_evidence.lock_sha256.clone(),
                base_image: self.environment_evidence.base_image.clone(),
                apt_snapshot: self.environment_evidence.apt_snapshot.clone(),
            },
            viewport,
            atspi,
            application: ApplicationEvidence {
                executable_sha256: sha256_file(&arguments.app)?,
                scenario_sha256: sha256_file(&arguments.scenario)?,
                visible_in_sway_tree: app_visible,
                scenario_marker_accessible,
                atspi_tree_node_count: atspi_tree.len(),
                atspi_non_window_node_count: atspi_tree
                    .iter()
                    .filter(|node| node.depth >= 2)
                    .count(),
                atspi_interactive_node_count,
            },
            input: InputEvidence {
                helper: "wtype",
                probe_client_visible: probe_visible,
                before_sha256,
                after_sha256,
            },
            capture: CaptureEvidence {
                helper: "grim",
                path: application
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| anyhow!("invalid application evidence path"))?
                    .to_owned(),
                bytes: fs::metadata(&application)?.len(),
                png_signature_valid: is_png(&application)?,
            },
        };
        write_json(&self.artifacts.join("capability.json"), &evidence)
    }

    fn start_sway(&mut self, configuration: &Path) -> Result<()> {
        let stdout = File::create(self.artifacts.join("sway.stdout.log"))?;
        let stderr = File::create(self.artifacts.join("sway.stderr.log"))?;
        let child = self
            .command("sway")
            .args(["--unsupported-gpu", "--config"])
            .arg(configuration)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .context("start headless Sway")?;
        self.sway = Some(child);
        Ok(())
    }

    fn start_app(&mut self, arguments: &CapabilityArguments) -> Result<()> {
        let stdout = File::create(self.artifacts.join("application.stdout.log"))?;
        let stderr = File::create(self.artifacts.join("application.stderr.log"))?;
        let child = self
            .command(&arguments.app)
            .args(["--backend", "scenario", "--scenario"])
            .arg(&arguments.scenario)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .context("start test-feature COSMIC Storage")?;
        self.app = Some(child);
        Ok(())
    }

    fn start_input_probe(&mut self) -> Result<()> {
        let stdout = File::create(self.artifacts.join("input-probe.stdout.log"))?;
        let stderr = File::create(self.artifacts.join("input-probe.stderr.log"))?;
        let child = self
            .command("foot")
            .args([
                "--app-id=ui-e2e-input-probe",
                "--title=ui-e2e-input-probe",
                "--",
                "sh",
                "-ec",
                "printf ready; exec sleep 30",
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .context("start virtual-keyboard probe client")?;
        self.foot = Some(child);
        Ok(())
    }

    fn wait_for_sway(&mut self) -> Result<(PathBuf, String)> {
        wait_until(READY_TIMEOUT, || {
            Self::ensure_running("Sway", self.sway.as_mut())?;
            let Some(wayland_socket) = find_wayland_socket(&self.runtime)? else {
                return Ok(None);
            };
            let Some(sway_socket) = find_sway_socket(&self.runtime)? else {
                return Ok(None);
            };
            let display = wayland_socket
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("invalid Wayland socket name"))?
                .to_owned();
            Ok(Some((sway_socket, display)))
        })
        .context("wait for Sway Wayland and IPC sockets")
    }

    fn assert_viewport(&self, sway_socket: &Path) -> Result<ViewportEvidence> {
        let output = self.run_checked_with_env(
            "swaymsg",
            [
                "-s",
                sway_socket.to_string_lossy().as_ref(),
                "-t",
                "get_outputs",
                "-r",
            ],
        )?;
        fs::write(self.artifacts.join("sway-outputs.json"), &output)?;
        let mut active: Vec<SwayOutput> = serde_json::from_slice::<Vec<SwayOutput>>(&output)
            .context("parse Sway output inventory")?
            .into_iter()
            .filter(|output| output.active)
            .collect();
        if active.len() != 1 {
            bail!(
                "expected exactly one active Sway output, got {}",
                active.len()
            );
        }
        let output = active.remove(0);
        if output.rect.width != self.viewport.width
            || output.rect.height != self.viewport.height
            || output.scale != self.viewport.scale
        {
            bail!(
                "Sway output {} is {}x{} scale {}, expected {}x{} scale 1",
                output.name,
                output.rect.width,
                output.rect.height,
                output.scale,
                self.viewport.width,
                self.viewport.height,
            );
        }
        Ok(ViewportEvidence {
            output: output.name,
            width: output.rect.width,
            height: output.rect.height,
            scale: output.scale,
        })
    }

    async fn assert_atspi_application(&self) -> Result<AtspiEvidence> {
        let deadline = Instant::now() + READY_TIMEOUT;
        loop {
            let evidence = self.query_atspi_applications().await?;
            if evidence
                .application_names
                .iter()
                .any(|name| name.to_ascii_lowercase().contains("storage"))
            {
                write_json(&self.artifacts.join("atspi.json"), &evidence)?;
                return Ok(evidence);
            }
            if Instant::now() >= deadline {
                bail!(
                    "COSMIC Storage did not appear in the AT-SPI application roots: {:?}",
                    evidence.application_names
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn enable_atspi(&self) -> Result<()> {
        let session = zbus::Connection::session()
            .await
            .context("connect to the private D-Bus session for AT-SPI activation")?;
        let status = StatusProxy::new(&session)
            .await
            .context("open the AT-SPI status proxy")?;
        status
            .set_screen_reader_enabled(false)
            .await
            .context("reset AT-SPI before enabling it for the application")?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        status
            .set_screen_reader_enabled(true)
            .await
            .context("enable AT-SPI for the running application")
    }

    async fn query_atspi_applications(&self) -> Result<AtspiEvidence> {
        let connection = atspi::AccessibilityConnection::new()
            .await
            .context("connect to the private AT-SPI registry")?;
        let children = connection
            .root_accessible_on_registry()
            .await
            .context("obtain the AT-SPI registry root")?
            .get_children()
            .await
            .context("query children of the AT-SPI registry root")?;
        let mut application_names = Vec::with_capacity(children.len());
        for child in &children {
            let application = child
                .as_accessible_proxy(connection.connection())
                .await
                .context("open AT-SPI application root")?;
            application_names.push(
                application
                    .name()
                    .await
                    .context("query AT-SPI application name")?,
            );
        }
        application_names.sort();
        Ok(AtspiEvidence {
            connected: true,
            registry_child_count: children.len(),
            application_names,
        })
    }

    async fn capture_atspi_tree(&self) -> Result<Vec<AccessibleNodeSnapshot>> {
        let connection = atspi::AccessibilityConnection::new()
            .await
            .context("reconnect to the private AT-SPI registry for tree capture")?;
        let mut pending = connection
            .root_accessible_on_registry()
            .await
            .context("open AT-SPI registry root for tree capture")?
            .get_children()
            .await
            .context("list AT-SPI application roots for tree capture")?
            .into_iter()
            .map(|root| (root, 0_usize))
            .collect::<Vec<_>>();
        let mut nodes = Vec::new();
        while let Some((reference, depth)) = pending.pop() {
            if depth > 32 || nodes.len() >= 4096 {
                bail!("AT-SPI tree exceeded the locked traversal limit");
            }
            let proxy = reference
                .as_accessible_proxy(connection.connection())
                .await
                .context("open AT-SPI node for tree capture")?;
            let children = proxy
                .get_children()
                .await
                .context("list AT-SPI node children for tree capture")?;
            pending.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
            let mut states = proxy
                .get_state()
                .await
                .context("query AT-SPI node state for tree capture")?
                .iter()
                .map(serde_json::to_value)
                .collect::<serde_json::Result<Vec<_>>>()?
                .into_iter()
                .map(|state| {
                    state
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| anyhow!("AT-SPI state did not serialize as a string"))
                })
                .collect::<Result<Vec<_>>>()?;
            states.sort();
            nodes.push(AccessibleNodeSnapshot {
                depth,
                automation_id: proxy
                    .accessible_id()
                    .await
                    .context("query AT-SPI node automation ID for tree capture")?,
                role: proxy
                    .get_role()
                    .await
                    .context("query AT-SPI node role for tree capture")?
                    .name()
                    .to_owned(),
                name: proxy
                    .name()
                    .await
                    .context("query AT-SPI node name for tree capture")?,
                description: proxy
                    .description()
                    .await
                    .context("query AT-SPI node description for tree capture")?,
                states,
            });
        }
        write_json(&self.artifacts.join("atspi-tree.json"), &nodes)?;
        Ok(nodes)
    }

    fn wait_for_tree_text(
        &mut self,
        sway_socket: &Path,
        expected: &str,
        timeout: Duration,
    ) -> Result<bool> {
        wait_until(timeout, || {
            Self::ensure_running("Sway", self.sway.as_mut())?;
            Self::ensure_running("application", self.app.as_mut())?;
            let tree = self.run_checked_with_env(
                "swaymsg",
                [
                    "-s",
                    sway_socket.to_string_lossy().as_ref(),
                    "-t",
                    "get_tree",
                    "-r",
                ],
            )?;
            fs::write(self.artifacts.join("sway-tree.json"), &tree)?;
            if tree
                .windows(expected.len())
                .any(|window| window == expected.as_bytes())
            {
                Ok(Some(true))
            } else {
                Ok(None)
            }
        })
    }

    fn capture(&self, output: &str, name: &str) -> Result<()> {
        let target = self.artifacts.join(name);
        self.run_checked("grim", ["-o", output, target.to_string_lossy().as_ref()])?;
        let metadata =
            fs::metadata(&target).with_context(|| format!("inspect {}", target.display()))?;
        if metadata.len() == 0 || !is_png(&target)? {
            bail!(
                "grim did not write a non-empty RGBA PNG at {}",
                target.display()
            );
        }
        Ok(())
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        command.envs(&self.environment);
        command
    }

    fn run_checked<I, S>(
        &self,
        program: impl AsRef<std::ffi::OsStr>,
        arguments: I,
    ) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.run_checked_with_env(program, arguments)
    }

    fn run_checked_with_env<I, S>(
        &self,
        program: impl AsRef<std::ffi::OsStr>,
        arguments: I,
    ) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = self
            .command(&program)
            .args(arguments)
            .output()
            .with_context(|| format!("execute {}", Path::new(program.as_ref()).display()))?;
        if !output.status.success() {
            bail!(
                "{} failed with {}: {}",
                Path::new(program.as_ref()).display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim(),
            );
        }
        Ok(output.stdout)
    }

    fn ensure_running(name: &str, child: Option<&mut Child>) -> Result<()> {
        let child = child.ok_or_else(|| anyhow!("{name} was not started"))?;
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("inspect {name} process"))?
        {
            bail!("{name} exited before readiness with {status}");
        }
        Ok(())
    }

    fn shutdown(&mut self) {
        terminate("input probe", self.foot.take());
        terminate("application", self.app.take());
        terminate("Sway", self.sway.take());
    }
}

impl Drop for CapabilitySession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn validate_input_file(path: &Path, description: &str) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("inspect {description} {}", path.display()))?;
    if !metadata.is_file() {
        bail!("{description} must be a regular file: {}", path.display());
    }
    Ok(())
}

fn ensure_private_directory(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn is_socket(path: &Path) -> Result<bool> {
    Ok(fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false))
}

fn find_sway_socket(runtime: &Path) -> Result<Option<PathBuf>> {
    find_runtime_socket(runtime, |name| {
        name.starts_with("sway-ipc.") && name.ends_with(".sock")
    })
}

fn find_wayland_socket(runtime: &Path) -> Result<Option<PathBuf>> {
    find_runtime_socket(runtime, |name| {
        name.starts_with("wayland-") && !name.ends_with(".lock")
    })
}

fn find_runtime_socket(
    runtime: &Path,
    matches_name: impl Fn(&str) -> bool,
) -> Result<Option<PathBuf>> {
    let mut sockets = fs::read_dir(runtime)
        .with_context(|| format!("list {}", runtime.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(&matches_name)
        })
        .filter(|path| is_socket(path).unwrap_or(false))
        .collect::<Vec<_>>();
    sockets.sort();
    if sockets.len() > 1 {
        bail!("multiple matching sockets in {}", runtime.display());
    }
    Ok(sockets.pop())
}

fn wait_until<T>(timeout: Duration, mut attempt: impl FnMut() -> Result<Option<T>>) -> Result<T> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = attempt()? {
            return Ok(value);
        }
        if Instant::now() >= deadline {
            bail!("timed out after {} seconds", timeout.as_secs());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn is_png(path: &Path) -> Result<bool> {
    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(bytes.starts_with(PNG_SIGNATURE))
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn terminate(name: &str, child: Option<Child>) {
    let Some(mut child) = child else {
        return;
    };
    if let Ok(None) = child.try_wait() {
        let _ = child.kill();
    }
    if let Ok(status) = child.wait() {
        record_exit(name, status);
    }
}

fn record_exit(_name: &str, _status: ExitStatus) {
    // The final runner writes a complete process ledger.  The capability probe
    // already preserves each process log and always waits for the child here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_signature_requires_all_eight_bytes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("image.png");
        fs::write(&path, b"\x89PNG\r\n\x1a\nbytes").expect("write PNG");
        assert!(is_png(&path).expect("read PNG"));
        fs::write(&path, b"PNG").expect("write non-PNG");
        assert!(!is_png(&path).expect("read non-PNG"));
    }

    #[test]
    fn legacy_case_inventory_is_sorted_and_rejects_duplicate_ids() {
        let directory = tempfile::tempdir().expect("temporary directory");
        fs::write(
            directory.path().join("z.toml"),
            "schema_version = 1\nid = \"z_case\"\n",
        )
        .expect("write case");
        fs::write(
            directory.path().join("a.toml"),
            "schema_version = 1\nid = \"a_case\"\n",
        )
        .expect("write case");
        assert_eq!(
            list_case_ids(directory.path()).expect("list cases"),
            ["a_case", "z_case"]
        );

        fs::write(
            directory.path().join("duplicate.toml"),
            "schema_version = 1\nid = \"a_case\"\n",
        )
        .expect("write duplicate case");
        assert!(list_case_ids(directory.path()).is_err());
    }

    #[test]
    fn interactive_role_gate_rejects_window_only_trees() {
        assert!(is_interactive_role("button"));
        assert!(is_interactive_role("scroll bar"));
        assert!(!is_interactive_role("application"));
        assert!(!is_interactive_role("frame"));
        assert!(!is_interactive_role("paragraph"));
    }

    #[test]
    fn scenario_marker_binds_the_fixture_id_and_hash() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("fixture.toml");
        fs::write(&path, "schema_version = 2\nid = \"fixture-id\"\n")
            .expect("write scenario fixture");

        let marker = scenario_marker(&path).expect("build scenario marker");
        assert!(marker.starts_with("Test scenario: fixture-id sha256:"));
        assert_eq!(marker.len(), "Test scenario: fixture-id sha256:".len() + 64);
    }
}
