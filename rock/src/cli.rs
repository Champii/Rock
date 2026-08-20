use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    artifact::{ensure_artifact, ArtifactBuildState},
    build::{build_project, run_project},
    commands::{expand_project, format_project},
};

pub fn run() -> Result<CliOutcome, String> {
    let config = Config::parse();

    match config.command {
        Command::Format => format_project().map(|_| CliOutcome::Success),
        Command::Build => {
            let executable = build_project(&current_project_root()?)?;
            println!("{}", executable.display());
            Ok(CliOutcome::Success)
        }
        Command::Run { args } => run_project(&current_project_root()?, &args).map(CliOutcome::Exit),
        Command::Expand => expand_project().map(|_| CliOutcome::Success),
        Command::Artifact => {
            let mut state = ArtifactBuildState::default();
            let artifact = ensure_artifact(&current_project_root()?, &mut state)?;
            println!("{}", artifact.display());
            Ok(CliOutcome::Success)
        }
    }
}

pub enum CliOutcome {
    Success,
    Exit(i32),
}

fn current_project_root() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("Failed to read current directory: {}", e))
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Config {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    Format,
    Build,
    Run {
        #[arg(last = true)]
        args: Vec<String>,
    },
    Expand,
    Artifact,
}
