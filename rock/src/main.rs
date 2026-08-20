fn main() {
    match rock::cli::run() {
        Ok(rock::cli::CliOutcome::Success) => {}
        Ok(rock::cli::CliOutcome::Exit(code)) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
