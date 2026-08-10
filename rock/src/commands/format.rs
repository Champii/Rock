use std::process::{Command as ProcessCommand, ExitStatus};

use crate::{package::Package, rockc::resolve_rockc_path};

pub(crate) fn format_project() -> Result<(), String> {
    let package = Package::load(
        std::env::current_dir().map_err(|e| format!("Failed to read current directory: {}", e))?,
    )?;
    let rockc = resolve_rockc_path()?;
    let mut command = ProcessCommand::new(&rockc);
    command
        .arg("--entry-file")
        .arg(package.entry_file())
        .arg("--format");
    let status = run_format_command(&mut command)?;

    if !status.success() {
        return Err(format!(
            "rockc failed for formatting with status {}",
            status
        ));
    }

    Ok(())
}

fn run_format_command(command: &mut ProcessCommand) -> Result<ExitStatus, String> {
    #[cfg(test)]
    {
        return command
            .output()
            .map(|output| output.status)
            .map_err(|e| format!("Failed to spawn rockc for formatting: {}", e));
    }

    #[cfg(not(test))]
    {
        command
            .status()
            .map_err(|e| format!("Failed to spawn rockc for formatting: {}", e))
    }
}
