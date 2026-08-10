mod cli;
mod constants;
mod dev;
mod fsutil;
mod home;
mod layout;
mod selection;
mod shell;
mod target;
mod toolchain;

#[cfg(test)]
mod tests;

fn main() {
    match cli::run() {
        Ok(cli::Exit::Success) => {}
        Ok(cli::Exit::Code(code)) => std::process::exit(code),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
