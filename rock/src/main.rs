mod artifact;
mod build;
mod bundled_sysroot;
mod cli;
mod commands;
mod compile;
mod deps;
mod package;
mod rockc;

#[cfg(test)]
mod tests;

fn main() {
    match cli::run() {
        Ok(cli::CliOutcome::Success) => {}
        Ok(cli::CliOutcome::Exit(code)) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
