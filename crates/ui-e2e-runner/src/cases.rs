//! Executed accessibility cases. Selection and action success never substitute
//! for semantic assertions; incomplete programs cannot produce passing evidence.

use super::*;
use atspi::proxy::{action::ActionProxy, editable_text::EditableTextProxy};
use atspi::{AccessibilityConnection, ObjectRefOwned, events::ObjectEvents};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::os::unix::process::CommandExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Parser)]
pub(super) struct ExecuteArguments {
    #[arg(long)]
    app: PathBuf,
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    sway_config: PathBuf,
    #[arg(long)]
    artifacts: PathBuf,
    #[arg(long)]
    environment_lock: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Selector {
    role: String,
    name: Option<String>,
    automation_id: Option<String>,
    #[serde(default)]
    states: Vec<String>,
    ancestor: Option<Box<Selector>>,
}

impl Selector {
    fn validate(&self) -> Result<()> {
        if self.role.is_empty() || (self.name.is_none() && self.automation_id.is_none()) {
            bail!("selectors require a role and an exact name or automation_id");
        }
        if let Some(ancestor) = &self.ancestor {
            ancestor.validate()?;
        }
        Ok(())
    }

    fn matches(&self, node: &Node, ancestors: &[Node]) -> bool {
        self.role == node.role
            && self.name.as_ref().is_none_or(|name| *name == node.name)
            && self
                .automation_id
                .as_ref()
                .is_none_or(|id| *id == node.automation_id)
            && self.states.iter().all(|state| node.states.contains(state))
            && self.ancestor.as_ref().is_none_or(|selector| {
                ancestors
                    .iter()
                    .enumerate()
                    .any(|(index, ancestor)| selector.matches(ancestor, &ancestors[..index]))
            })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Assertion {
    Unique {
        selector: Selector,
    },
    Absent {
        selector: Selector,
    },
    Count {
        selector: Selector,
        count: usize,
    },
    PropertyEquals {
        selector: Selector,
        property: String,
        value: String,
    },
    Focused {
        selector: Selector,
    },
    StateEquals {
        pointer: String,
        value: Value,
    },
}

impl Assertion {
    fn selector(&self) -> Option<&Selector> {
        match self {
            Self::Unique { selector }
            | Self::Absent { selector }
            | Self::Count { selector, .. }
            | Self::PropertyEquals { selector, .. }
            | Self::Focused { selector } => Some(selector),
            Self::StateEquals { .. } => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Readiness {
    timeout_ms: u64,
    assert: Vec<Assertion>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    schema_version: u8,
    id: String,
    scenario: PathBuf,
    scenario_sha256: String,
    ready: Readiness,
    step: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Step {
    id: String,
    #[serde(flatten)]
    operation: Operation,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Invoke {
        target: Selector,
        action: Option<String>,
    },
    SetText {
        target: Selector,
        value: String,
        #[serde(default)]
        secret: bool,
    },
    Key {
        key: String,
    },
    AdvanceClock {
        milliseconds: u64,
    },
    ReplaceOverlay {
        fixture: PathBuf,
        #[serde(default)]
        expect_error: bool,
    },
    Reload,
    Assert {
        assert: Assertion,
    },
}

#[derive(Clone, Debug, Serialize)]
struct Node {
    #[serde(skip)]
    reference: ObjectRefOwned,
    depth: usize,
    role: String,
    name: String,
    automation_id: String,
    description: String,
    states: Vec<String>,
}

fn checked_fixture(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
    {
        bail!("case fixture must be an in-repository relative path");
    }
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root.canonicalize()?) || !path.is_file() {
        bail!("case fixture escapes the repository or is not a file");
    }
    Ok(path)
}

fn parse_case(source: &str) -> Result<Case> {
    let case: Case = toml::from_str(source)?;
    if case.schema_version != 2
        || !is_case_identifier(&case.id)
        || case.scenario_sha256.len() != 64
        || !case
            .scenario_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || !(1..=15000).contains(&case.ready.timeout_ms)
        || case.ready.assert.is_empty()
        || case.step.is_empty()
    {
        bail!("invalid or empty version-2 case");
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut assertions = 0;
    let mut actions = 0;
    for assertion in &case.ready.assert {
        if let Some(selector) = assertion.selector() {
            selector.validate()?;
        }
    }
    for step in &case.step {
        if !is_case_identifier(&step.id) || !ids.insert(&step.id) {
            bail!("invalid/duplicate step ID");
        }
        match &step.operation {
            Operation::Invoke { target, .. } | Operation::SetText { target, .. } => {
                target.validate()?;
                actions += 1;
            }
            Operation::Key { key } => {
                keyboard_arguments(key)?;
                actions += 1;
            }
            Operation::Assert { assert } => {
                assertions += 1;
                if let Some(selector) = assert.selector() {
                    selector.validate()?;
                }
            }
            _ => {}
        }
    }
    if assertions == 0
        || actions == 0
        || !matches!(
            case.step.last().map(|s| &s.operation),
            Some(Operation::Assert { .. })
        )
    {
        bail!("an executed case needs a real UI action and a final explicit assertion");
    }
    Ok(case)
}

fn keyboard_arguments(key: &str) -> Result<Vec<&str>> {
    match key {
        "Tab" | "Return" | "Escape" | "BackSpace" | "space" | "Left" | "Right" | "Up" | "Down" => {
            Ok(vec!["-k", key])
        }
        "Shift+Tab" => Ok(vec!["-M", "shift", "-k", "Tab", "-m", "shift"]),
        text if text.chars().count() == 1 && !text.chars().next().unwrap().is_control() => {
            Ok(vec!["--", text])
        }
        _ => bail!("unsupported virtual key"),
    }
}

async fn tree(connection: &AccessibilityConnection) -> Result<Vec<Node>> {
    let roots = connection
        .root_accessible_on_registry()
        .await?
        .get_children()
        .await?;
    let mut application = Vec::new();
    for root in roots {
        if root
            .as_accessible_proxy(connection.connection())
            .await?
            .name()
            .await?
            == "cosmic-ext-storage"
        {
            application.push(root);
        }
    }
    if application.len() != 1 {
        bail!("expected exactly one cosmic-ext-storage AT-SPI application");
    }
    let mut pending = vec![(application.pop().unwrap(), 0)];
    let mut nodes = Vec::new();
    while let Some((reference, depth)) = pending.pop() {
        if depth > 32 || nodes.len() >= 4096 {
            bail!("AT-SPI traversal bound exceeded");
        }
        let proxy = reference
            .as_accessible_proxy(connection.connection())
            .await?;
        let children = proxy.get_children().await?;
        let mut states = proxy
            .get_state()
            .await?
            .iter()
            .map(|state| {
                serde_json::to_value(state).map(|v| v.as_str().unwrap().to_ascii_lowercase())
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        states.sort();
        let node = Node {
            reference: reference.clone(),
            depth,
            role: proxy.get_role().await?.name().into(),
            name: proxy.name().await?,
            automation_id: proxy.accessible_id().await?,
            description: proxy.description().await?,
            states,
        };
        nodes.push(node);
        pending.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
    }
    Ok(nodes)
}

fn selected<'a>(nodes: &'a [Node], selector: &Selector) -> Vec<&'a Node> {
    let mut ancestors = Vec::new();
    let mut selected = Vec::new();
    for node in nodes {
        ancestors.truncate(node.depth);
        if selector.matches(node, &ancestors) {
            selected.push(node);
        }
        ancestors.push(node.clone());
    }
    selected
}

fn unique<'a>(nodes: &'a [Node], selector: &Selector) -> Result<&'a Node> {
    let matches = selected(nodes, selector);
    if matches.len() != 1 {
        bail!(
            "selector {selector:?} matched {} nodes, expected exactly one",
            matches.len()
        );
    }
    Ok(matches[0])
}

fn assert_tree(nodes: &[Node], assertion: &Assertion) -> Result<()> {
    match assertion {
        Assertion::Unique { selector } => {
            unique(nodes, selector)?;
        }
        Assertion::Absent { selector } => {
            if !selected(nodes, selector).is_empty() {
                bail!("node is still present: {selector:?}");
            }
        }
        Assertion::Count { selector, count } => {
            if selected(nodes, selector).len() != *count {
                bail!("wrong node count for {selector:?}");
            }
        }
        Assertion::PropertyEquals {
            selector,
            property,
            value,
        } => {
            let node = unique(nodes, selector)?;
            let actual = match property.as_str() {
                "name" => &node.name,
                "description" => &node.description,
                "role" => &node.role,
                "automation_id" => &node.automation_id,
                _ => bail!("unsupported accessible property"),
            };
            if actual != value {
                bail!("accessible property differs: {actual:?} != {value:?}");
            }
        }
        Assertion::Focused { selector } => {
            let node = unique(nodes, selector)?;
            if !node.states.iter().any(|s| s == "focused")
                || nodes
                    .iter()
                    .filter(|n| n.states.iter().any(|s| s == "focused"))
                    .count()
                    != 1
            {
                bail!("expected exactly one focused node, matching {selector:?}");
            }
        }
        Assertion::StateEquals { .. } => bail!("state assertion needs scenario control"),
    }
    Ok(())
}

struct Control {
    socket: PathBuf,
    token: String,
    sequence: u64,
    evidence: Vec<Value>,
}

impl Control {
    async fn request(&mut self, command: Value, expect_error: bool) -> Result<Value> {
        self.sequence += 1;
        let request = serde_json::to_vec(
            &json!({"protocol":1,"sequence":self.sequence,"token":self.token,"command":command}),
        )?;
        let result = tokio::time::timeout(READY_TIMEOUT, async {
            let mut socket = tokio::net::UnixStream::connect(&self.socket).await?;
            socket.write_u32(u32::try_from(request.len())?).await?;
            socket.write_all(&request).await?;
            let size = socket.read_u32().await?;
            if size == 0 || size > 1024 * 1024 {
                bail!("invalid control response frame size");
            }
            let mut bytes = vec![0; size as usize];
            socket.read_exact(&mut bytes).await?;
            let response: Value = serde_json::from_slice(&bytes)?;
            if response["protocol"] != 1
                || response["sequence"] != self.sequence
                || response.get("ok").is_some() == response.get("error").is_some()
            {
                bail!("invalid/mismatched scenario control response");
            }
            Ok::<_, anyhow::Error>(response)
        })
        .await
        .context("scenario control deadline")??;
        self.evidence
            .push(json!({"sequence":self.sequence,"command":command["kind"],"response":result}));
        if result.get("error").is_some() != expect_error {
            bail!("unexpected control result: {result}");
        }
        Ok(result)
    }
}

pub(super) async fn execute(arguments: ExecuteArguments) -> Result<()> {
    // Hash the large debug ELF once, before launching the app. Avoid a long
    // duplicate hash between functional completion and the close request.
    let executable_sha256 = sha256_file(&arguments.app)?;
    let case = parse_case(&fs::read_to_string(&arguments.case)?)?;
    let scenario = checked_fixture(&arguments.root, &case.scenario)?;
    if sha256_file(&scenario)? != case.scenario_sha256 {
        bail!("scenario hash differs from executable case");
    }
    let lock = EnvironmentLock::load(&arguments.environment_lock)?;
    let evidence = EnvironmentEvidence {
        lock_sha256: sha256_file(&arguments.environment_lock)?,
        base_image: lock.base_image.clone(),
        apt_snapshot: lock.apt_snapshot.clone(),
    };
    // Never delete a user-supplied artifact directory. Each execution gets a new
    // child; failed runs and any previous evidence remain intact.
    fs::create_dir_all(&arguments.artifacts)?;
    let artifacts = arguments.artifacts.join(format!(
        "{}-{}-{}",
        case.id,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut session = CapabilitySession::new(&artifacts, &lock, evidence)?;
    let token = {
        use std::io::Read;
        let mut bytes = [0u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let token_file = session.artifacts.join("control.token");
    fs::write(&token_file, &token)?;
    fs::set_permissions(&token_file, fs::Permissions::from_mode(0o600))?;
    let mut control = Control {
        socket: session.runtime.join("control.sock"),
        token,
        sequence: 0,
        evidence: Vec::new(),
    };
    let mut completed = Vec::new();
    let mut result = tokio::time::timeout(
        Duration::from_secs(180),
        run_case(
            &arguments,
            &case,
            &scenario,
            &token_file,
            &mut session,
            &mut control,
            &mut completed,
        ),
    )
    .await
    .context("UI case watchdog expired")
    .and_then(|result| result);
    // Evidence is a functional gate and is persisted while the application is
    // still alive, before any close request or teardown race can occur.
    let capture = tokio::time::timeout(READY_TIMEOUT, session.capture_atspi_tree()).await;
    let screenshot = session.capture("HEADLESS-1", "final.png");
    if result.is_ok() {
        result = capture
            .context("pre-close tree capture deadline")
            .and_then(|r| r.map(|_| ()))
            .and(screenshot.map(|_| ()))
            .and_then(|_| {
                CapabilitySession::ensure_running("application debugger", session.app.as_mut())
            });
    }
    write_json(&artifacts.join("control.json"), &control.evidence)?;
    let functional_passed = result.is_ok();
    write_json(
        &artifacts.join("functional.json"),
        &json!({
            "schema_version":1,"case":case.id,"status":if functional_passed {"passed"} else {"failed"},
            "completed_steps":completed,"error":result.as_ref().err().map(|e|format!("{e:#}")),
            "scenario_sha256":case.scenario_sha256,
            "case_sha256":sha256_file(&arguments.case)?,
        "executable_sha256":executable_sha256,
            "tree_sha256":sha256_file(&artifacts.join("atspi-tree.json")).ok(),
            "screenshot_sha256":sha256_file(&artifacts.join("final.png")).ok()
        }),
    )?;
    let mut outcome = shutdown::Outcome::NotAttempted;
    let mut shutdown_error = None;
    if functional_passed {
        match close_application(&mut session, &arguments, &case).await {
            Ok(value) => outcome = value,
            Err(error) => {
                outcome = shutdown::Outcome::Failed;
                shutdown_error = Some(format!("{error:#}"));
            }
        }
    }
    write_json(
        &artifacts.join("shutdown.json"),
        &json!({
            "schema_version":1,"status":outcome,"error":shutdown_error,
            "quarantine":shutdown::policy_evidence()?,
            "diagnostic_sha256":sha256_file(&artifacts.join("debugger.json")).ok()
        }),
    )?;
    session.shutdown();
    let status = shutdown::status(functional_passed, &outcome);
    write_json(
        &artifacts.join("execution.json"),
        &json!({
            "schema_version":2,"case":case.id,"status":status,
            "functional_status":if functional_passed {"passed"} else {"failed"},
            "shutdown_status":outcome,"shutdown_error":shutdown_error,
            "completed_steps":completed,"scenario_sha256":case.scenario_sha256,
            "executable_sha256":executable_sha256, "case_sha256":sha256_file(&arguments.case)?,
            "error":result.as_ref().err().map(|e|format!("{e:#}")),
            "visual_baseline_status":"not_approved",
            "coverage_status":"not_verified; functional success is not profile evidence",
            "environment_lock_sha256":sha256_file(&arguments.environment_lock)?,
            "cargo_lock_sha256":sha256_file(&arguments.root.join("Cargo.lock"))?,
            "debugger_script_sha256":sha256_file(&arguments.root.join("tools/ui-testing/debug-app.py"))?
        }),
    )?;
    println!("UI case {}: {} ({})", case.id, status, artifacts.display());
    result?;
    if status == "failed" {
        bail!(
            "shutdown failed (unrecognized, missing diagnostics, or timed out): see shutdown.json"
        );
    }
    if outcome == shutdown::Outcome::KnownFailure {
        eprintln!(
            "WARNING: functional tests passed; known iced Wayland shutdown failure quarantined. Coverage is not verified."
        );
    }
    Ok(())
}

async fn wait_child(child: &mut Child, timeout: Duration) -> Result<ExitStatus> {
    tokio::time::timeout(timeout, async {
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("process exit deadline")?
}

async fn close_application(
    session: &mut CapabilitySession,
    arguments: &ExecuteArguments,
    case: &Case,
) -> Result<shutdown::Outcome> {
    CapabilitySession::ensure_running("application debugger", session.app.as_mut())?;
    if session.artifacts.join("debugger.json").exists() {
        bail!("application exited or stopped before the close request");
    }
    let (socket, _) = session.wait_for_sway()?;
    fs::write(
        session.artifacts.join("close-requested"),
        "functional evidence saved\n",
    )?;
    let mut close = session
        .command("swaymsg")
        .args([
            "-s",
            socket.to_str().context("Sway socket path")?,
            "[app_id=\"com.cosmic.ext.Storage\"] kill",
        ])
        .stdout(File::create(session.artifacts.join("close.stdout.log"))?)
        .stderr(File::create(session.artifacts.join("close.stderr.log"))?)
        .spawn()?;
    let request_result = wait_child(&mut close, READY_TIMEOUT).await;
    if request_result.is_err() {
        let _ = close.kill();
        let _ = close.wait();
    }
    if !request_result?.success() {
        bail!("window close request failed");
    }
    let exit = wait_child(
        session.app.as_mut().context("application debugger")?,
        READY_TIMEOUT,
    )
    .await?;
    let diagnostic: shutdown::Diagnostic =
        serde_json::from_slice(&fs::read(session.artifacts.join("debugger.json"))?)?;
    shutdown::classify(
        &diagnostic,
        exit.code(),
        &case.id,
        &sha256_file(&arguments.case)?,
        &sha256_file(&arguments.environment_lock)?,
        &sha256_file(&arguments.root.join("Cargo.lock"))?,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )
}

async fn run_case(
    arguments: &ExecuteArguments,
    case: &Case,
    scenario: &Path,
    token_file: &Path,
    session: &mut CapabilitySession,
    control: &mut Control,
    completed: &mut Vec<String>,
) -> Result<()> {
    session.start_sway(&arguments.sway_config)?;
    let (sway_socket, display) = session.wait_for_sway()?;
    session
        .environment
        .insert("WAYLAND_DISPLAY".into(), display.into());
    let viewport = session.assert_viewport(&sway_socket)?;
    session.app_process_group = true;
    session.app = Some(
        session
            .command("gdb")
            .process_group(0)
            .args([
                "--batch",
                "--nx",
                "--return-child-result",
                "-iex",
                "set auto-load off",
                "-iex",
                "set debuginfod enabled off",
                "-ex",
                "set confirm off",
                "-ex",
                "set disable-randomization off",
                "-ex",
                "set print frame-arguments none",
                "-ex",
                "handle SIGPIPE nostop noprint pass",
                "-x",
            ])
            .arg(arguments.root.join("tools/ui-testing/debug-app.py"))
            .args(["-ex", "run", "--args"])
            .arg(&arguments.app)
            .env("UI_DEBUG_REPORT", session.artifacts.join("debugger.json"))
            .env("UI_CLOSE_MARKER", session.artifacts.join("close-requested"))
            .args(["--backend", "scenario", "--scenario"])
            .arg(scenario)
            .arg("--scenario-state")
            .arg(session.artifacts.join("overlay.toml"))
            .arg("--scenario-trace")
            .arg(session.artifacts.join("scenario-trace.jsonl"))
            .arg("--scenario-control-socket")
            .arg(&control.socket)
            .arg("--scenario-control-token-file")
            .arg(token_file)
            .stdout(File::create(
                session.artifacts.join("application.stdout.log"),
            )?)
            .stderr(File::create(
                session.artifacts.join("application.stderr.log"),
            )?)
            .spawn()?,
    );
    session.wait_for_tree_text(&sway_socket, "com.cosmic.ext.Storage", READY_TIMEOUT)?;
    session.enable_atspi().await?;
    session.assert_atspi_application().await?;
    let connection = AccessibilityConnection::new().await?;
    connection.register_event::<ObjectEvents>().await?;
    let mut events = EventWatch::new(&connection).await?;
    for assertion in &case.ready.assert {
        wait_assert(
            &connection,
            &mut events,
            control,
            assertion,
            Duration::from_millis(case.ready.timeout_ms),
        )
        .await?;
    }
    for step in &case.step {
        let nodes = observed_tree(
            &connection,
            &mut events,
            tokio::time::Instant::now() + READY_TIMEOUT,
        )
        .await?;
        write_json(
            &session
                .artifacts
                .join(format!("{}-before.a11y.json", step.id)),
            &nodes,
        )?;
        match &step.operation {
            Operation::Invoke { target, action } => {
                let node = unique(&nodes, target)?;
                if !node.states.iter().any(|s| s == "enabled") {
                    bail!("target is disabled");
                }
                let proxy = ActionProxy::builder(connection.connection())
                    .destination(
                        node.reference
                            .name_as_str()
                            .context("AT-SPI node has no bus owner")?,
                    )?
                    .path(node.reference.path_as_str())?
                    .build()
                    .await?;
                let actions = proxy.get_actions().await?;
                let matches = actions
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| {
                        action
                            .as_ref()
                            .map_or(matches!(a.name.as_str(), "click" | "activate"), |name| {
                                a.name == *name
                            })
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    bail!("ambiguous/missing accessible action: {actions:?}");
                }
                if !proxy.do_action(matches[0].0 as i32).await? {
                    bail!("AT-SPI action was rejected");
                }
            }
            Operation::SetText {
                target,
                value,
                secret,
            } => {
                let node = unique(&nodes, target)?;
                let proxy = EditableTextProxy::builder(connection.connection())
                    .destination(
                        node.reference
                            .name_as_str()
                            .context("AT-SPI node has no bus owner")?,
                    )?
                    .path(node.reference.path_as_str())?
                    .build()
                    .await?;
                // Never serialize text contents or the manifest in evidence.
                let _redact = secret;
                if !proxy.set_text_contents(value).await? {
                    bail!("AT-SPI text update was rejected");
                }
            }
            Operation::Key { key } => {
                session.run_checked("wtype", keyboard_arguments(key)?)?;
            }
            Operation::AdvanceClock { milliseconds } => {
                control
                    .request(
                        json!({"kind":"advance_clock","milliseconds":milliseconds}),
                        false,
                    )
                    .await?;
            }
            Operation::ReplaceOverlay {
                fixture,
                expect_error,
            } => {
                let bytes = fs::read(checked_fixture(&arguments.root, fixture)?)?;
                control.request(json!({"kind":"replace_overlay","sha256":format!("{:x}",Sha256::digest(&bytes)),"bytes_base64":STANDARD.encode(bytes)}), *expect_error).await?;
            }
            Operation::Reload => {
                control.request(json!({"kind":"reload"}), false).await?;
            }
            Operation::Assert { assert } => {
                wait_assert(&connection, &mut events, control, assert, READY_TIMEOUT).await?;
            }
        }
        completed.push(step.id.clone());
        session.capture(&viewport.output, &format!("{}.png", step.id))?;
    }
    Ok(())
}

async fn wait_assert(
    connection: &AccessibilityConnection,
    events: &mut EventWatch,
    control: &mut Control,
    assertion: &Assertion,
    timeout: Duration,
) -> Result<()> {
    if let Assertion::StateEquals { pointer, value } = assertion {
        let snapshot = control.request(json!({"kind":"snapshot"}), false).await?;
        if snapshot.pointer(pointer) != Some(value) {
            bail!("scenario snapshot mismatch at {pointer}: {snapshot}");
        }
        return Ok(());
    }
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        events
            .receiver
            .borrow_and_update()
            .clone()
            .map_err(|error| anyhow!(error))?;
        let nodes = observed_tree(connection, events, deadline).await?;
        match assert_tree(&nodes, assertion) {
            Ok(()) => return Ok(()),
            Err(error) => {
                match tokio::time::timeout_at(deadline, events.receiver.changed()).await {
                    Ok(Ok(())) => {}
                    _ => return Err(error.context("semantic assertion deadline")),
                }
            }
        }
    }
}

async fn observed_tree(
    connection: &AccessibilityConnection,
    events: &mut EventWatch,
    deadline: tokio::time::Instant,
) -> Result<Vec<Node>> {
    loop {
        events
            .receiver
            .borrow_and_update()
            .clone()
            .map_err(|error| anyhow!(error))?;
        match tokio::time::timeout_at(deadline, tree(connection))
            .await
            .context("fresh AT-SPI tree deadline")?
        {
            Ok(nodes) => return Ok(nodes),
            Err(error) => {
                // Re-rendering can retire a node between GetChildren and its
                // properties. Re-query observations, never repeat UI actions.
                let retired = matches!(error.downcast_ref::<zbus::Error>(), Some(zbus::Error::MethodError(name, _, _)) if name.as_str() == "org.freedesktop.DBus.Error.UnknownObject");
                if !retired {
                    return Err(error);
                }
                tokio::time::timeout_at(deadline, events.receiver.changed())
                    .await
                    .context("retired AT-SPI node replacement deadline")??;
            }
        }
    }
}

struct EventWatch {
    receiver: tokio::sync::watch::Receiver<std::result::Result<u64, String>>,
    task: tokio::task::JoinHandle<()>,
}

impl EventWatch {
    async fn new(connection: &AccessibilityConnection) -> Result<Self> {
        // An undrained all-message stream blocks zbus's bounded dispatch queue
        // during tree RPCs. Subscribe only to object signals and drain them on
        // an independent task; assertions always inspect a fresh scoped tree.
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.a11y.atspi.Event.Object")?
            .build();
        let mut stream =
            zbus::MessageStream::for_match_rule(rule, connection.connection(), Some(256)).await?;
        let (sender, receiver) = tokio::sync::watch::channel(Ok(0u64));
        let task = tokio::spawn(async move {
            let mut sequence = 0u64;
            while let Some(message) = stream.next().await {
                match message {
                    Ok(_) => {
                        sequence += 1;
                        if sender.send(Ok(sequence)).is_err() {
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error.to_string()));
                        return;
                    }
                }
            }
            let _ = sender.send(Err("AT-SPI event stream closed".into()));
        });
        Ok(Self { receiver, task })
    }
}

impl Drop for EventWatch {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
#[path = "../tests/unit/cases_tests.rs"]
mod tests;
