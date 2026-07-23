use std::str::FromStr;

use clap::Parser;
use storage_testing::{
    errors::{Result, TestingError},
    harness::{HarnessRunner, NondestructiveExecutor, Profile, RunConfig},
    lab::orchestrator::FullLabExecutor,
};

#[derive(Debug, Parser)]
#[command(
    name = "harness",
    about = "Disposable-fixture storage integration harness"
)]
struct Cli {
    #[arg(long)]
    profile: String,
    #[arg(long)]
    suite: Option<String>,
    #[arg(long = "case")]
    case_ids: Vec<String>,
    #[arg(long)]
    require_executed: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let profile = Profile::from_str(&cli.profile)?;
    let runner = HarnessRunner::from_environment()?;
    let config = RunConfig {
        profile,
        suite: cli.suite,
        case_ids: cli.case_ids,
        require_executed: cli.require_executed,
    };
    let report = match profile {
        Profile::Nondestructive => {
            let mut executor = NondestructiveExecutor;
            runner.run(config, &mut executor)?
        }
        Profile::FullLab => {
            let mut executor = FullLabExecutor::new()?;
            runner.run(config, &mut executor)?
        }
    };
    if !report.all_selected_cases_passed() {
        return Err(TestingError::RequiredExecution(
            "selected cases did not all pass".into(),
        ));
    }
    println!(
        "harness report: {}",
        std::env::var("STORAGE_TESTING_ARTIFACT_DIR").unwrap_or_default()
    );
    Ok(())
}
