use std::path::PathBuf;

use test_backend::{ScenarioRuntime, ScenarioStore, parse_fixture};

fn main() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("validate") => {
            let path = arguments
                .next()
                .ok_or("usage: ui-scenario validate <fixture>")?;
            if arguments.next().is_some() {
                return Err("usage: ui-scenario validate <fixture>".into());
            }
            let store = ScenarioStore::new(PathBuf::from(path), None, None);
            let (spec, hash) =
                parse_fixture(&store.read_fixture().map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            println!("valid scenario {} ({hash})", spec.id);
            Ok(())
        }
        Some("print-schema") => {
            println!(
                "schema_version = 2; root = schema_version,id,description,clock,backend,world,behaviour"
            );
            Ok(())
        }
        Some("materialize-state") => {
            let path = arguments
                .next()
                .ok_or("usage: ui-scenario materialize-state <fixture>")?;
            let runtime =
                ScenarioRuntime::load(path, None, None).map_err(|error| error.to_string())?;
            let diagnostics = futures::executor::block_on(runtime.backend().diagnostics_snapshot());
            println!(
                "{}",
                serde_json::to_string(&diagnostics).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        _ => Err("usage: ui-scenario <validate|print-schema|materialize-state> ...".into()),
    }
}
