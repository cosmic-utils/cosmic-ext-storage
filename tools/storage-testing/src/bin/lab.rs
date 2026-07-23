use clap::{Parser, Subcommand};
use storage_testing::{artifacts::create_run_artifact_dir, errors::Result};

#[derive(Debug, Parser)]
#[command(name = "lab", about = "Disposable fixture-lab support")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create and print a fresh marker-bearing run-artifact directory.
    CreateArtifact {
        #[arg(long, default_value = "lab")]
        label: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::CreateArtifact { label } => {
            println!("{}", create_run_artifact_dir(&label)?.display());
            Ok(())
        }
    }
}
