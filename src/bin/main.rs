extern crate clap;
extern crate fock;

#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::process::Command;

use fock::logger;

pub(crate) use fock::*;

fn build(config: Config) -> bool {
    info!(" -> Building");

    let entry_file = "./src/main.rk";

    if let Err(_e) = fock::file_to_file(entry_file.to_string(), "".to_string(), config.clone()) {
        return false;
    }

    info!(" -> Linking");

    Command::new("llc")
        .args(&["out.ir"])
        .output()
        .expect("failed to execute process");

    Command::new("clang")
        .args(&["out.ir.s"])
        .output()
        .expect("failed to execute process");

    true
}

fn run(config: Config) {
    if !build(config) {
        return;
    }

    let cmd = Command::new("./a.out")
        .output()
        .expect("failed to execute process");

    print!("{}", String::from_utf8(cmd.stdout).unwrap());

    match cmd.status.code() {
        Some(code) => std::process::exit(code),
        None => println!(
            "\nError running: \n{}",
            String::from_utf8(cmd.stderr).unwrap()
        ),
    }

    std::process::exit(-1);
}

fn main() {
    let matches = App::new("fock")
        .version("0.0.1")
        .author("Champii <contact@champii.io>")
        .about("Simple toy language")
        .arg(
            Arg::with_name("verbose")
                .takes_value(true)
                .short("v")
                .help("Verbose level"),
        )
        .arg(
            Arg::with_name("ast")
                .short("a")
                .takes_value(false)
                .help("Show ast"),
        )
        .arg(
            Arg::with_name("hir")
                .short("h")
                .takes_value(false)
                .help("Show hir"),
        )
        .arg(
            Arg::with_name("ir")
                .short("i")
                .takes_value(false)
                .help("Show the generated IR"),
        )
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

    let mut config = fock::Config {
        verbose: 2,
        show_ast: matches.is_present("ast"),
        show_hir: matches.is_present("hir"),
        show_ir: matches.is_present("ir"),
        ..Default::default()
    };

    if let Some(value) = matches.value_of("verbose") {
        config.verbose = value.parse::<u8>().unwrap();
    }

    logger::init_logger(config.verbose);

    if let Some(_matches) = matches.subcommand_matches("build") {
        build(config);

        return;
    }

    if let Some(_matches) = matches.subcommand_matches("run") {
        run(config);

        return;
    }
}
