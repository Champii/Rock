extern crate clap;
extern crate plasma;

use clap::{App, Arg, SubCommand};

use std::process::{Command, ExitStatus};

fn build() -> bool {
    if let Err(e) = plasma::file_to_file("./main.pm".to_string(), "./main.o\0".to_string()) {
        println!("{}", e);

        return false;
    }

    Command::new("clang")
        .arg("main.o")
        .output()
        .expect("failed to execute process");

    true
}

fn run() {
    if !build() {
        return;
    }

    let cmd = Command::new("./a.out")
        .output()
        .expect("failed to execute process");

    print!("{}", String::from_utf8(cmd.stdout).unwrap());

    std::process::exit(cmd.status.code().unwrap());
}

fn main() {
    let matches = App::new("plasma")
        .version("0.0.1")
        .author("Champii <contact@champii.io>")
        .about("Simple toy language")
        .subcommand(
            SubCommand::with_name("build")
                .about("Build the current project directory")
                .version("0.0.1")
                .author("Champii <contact@champii.io>"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run the current project directory")
                .version("0.0.1")
                .author("Champii <contact@champii.io>"),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("build") {
        build();
    }

    if let Some(matches) = matches.subcommand_matches("run") {
        run();
    }
}
