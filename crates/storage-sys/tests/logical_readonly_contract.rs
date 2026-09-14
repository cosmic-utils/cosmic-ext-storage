use std::sync::{Arc, Mutex};

use storage_contracts::LogicalTopologySource;
use storage_sys::{LocalLogicalTopologySource, LogicalCommandRunner};

#[test]
fn local_tools_use_stable_readonly_argv_and_order() {
    let runner = Arc::new(FakeRunner::valid());
    let source = LocalLogicalTopologySource::with_runner(runner.clone());
    let entities = futures::executor::block_on(source.list_logical_entities()).unwrap();
    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0].name, "lv0");
    assert_eq!(entities[1].name, "vg0");
    let invocations = runner.invocations.lock().unwrap();
    assert_eq!(
        invocations
            .iter()
            .map(|(program, _)| program.as_str())
            .collect::<Vec<_>>(),
        ["vgs", "lvs", "pvs"]
    );
    assert!(
        invocations
            .iter()
            .all(|(program, args)| !program.contains("sh")
                && !args.iter().any(|argument| argument.contains(";")))
    );
}

#[test]
fn malformed_local_record_fails_source() {
    let runner = Arc::new(FakeRunner::malformed());
    let source = LocalLogicalTopologySource::with_runner(runner);
    let error = futures::executor::block_on(source.list_logical_entities()).unwrap_err();
    assert!(error.message.contains("vgs record 1"));
}

#[test]
fn local_tools_reject_mutating_commands() {
    let runner = storage_sys::SystemLogicalCommandRunner;
    assert!(runner.run("vgcreate", &["unsafe"]).is_err());
    assert!(runner.run("vgs", &["--all"]).is_err());
}

struct FakeRunner {
    invocations: Mutex<Vec<(String, Vec<String>)>>,
    output: &'static str,
}

impl FakeRunner {
    fn valid() -> Self {
        Self {
            invocations: Mutex::new(Vec::new()),
            output: "vg0\tvg-uuid\t100\t25\t1\t1\n",
        }
    }

    fn malformed() -> Self {
        Self {
            invocations: Mutex::new(Vec::new()),
            output: "vg0\tvg-uuid\tbad\n",
        }
    }
}

impl LogicalCommandRunner for FakeRunner {
    fn is_available(&self, _program: &str) -> bool {
        true
    }

    fn run(
        &self,
        program: &str,
        args: &[&str],
    ) -> Result<String, storage_sys::logical::LocalToolsError> {
        self.invocations.lock().unwrap().push((
            program.to_string(),
            args.iter().map(ToString::to_string).collect(),
        ));
        let output = match program {
            "vgs" => self.output,
            "lvs" => "vg0\tvg-uuid\tlv0\t75\tactive\n",
            "pvs" => "/dev/loop0\tvg0\t100\t25\n",
            _ => "",
        };
        Ok(output.to_string())
    }
}
