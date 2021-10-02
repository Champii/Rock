extern crate clap;
extern crate rock;

#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::{fs, path::PathBuf, process::Command};

pub mod logger;

use rock::diagnostics::DiagnosticKind;
pub(crate) use rock::*;

fn build(config: &Config) -> bool {
    debug!(" -> Building");

    let entry_file = "./src/main.rk";

    fs::create_dir_all(config.build_folder.clone()).unwrap();

    if let Err(diagnostic) = rock::parse_file(entry_file.to_string(), config) {
        if let DiagnosticKind::NoError = diagnostic.get_kind() {
        } else {
            println!("Error: {}", diagnostic.get_kind());
        }

        return false;
    }

    let llc_cmd = Command::new("llc")
        .args(&[
            "--relocation-model=pic",
            config.build_folder.join("out.ir").to_str().unwrap(),
        ])
        .output()
        .expect("failed to compile to ir");

    match llc_cmd.status.code() {
        Some(code) => {
            if code != 0 {
                println!(
                    "BUG: Cannot compile to ir: \n{}",
                    String::from_utf8(llc_cmd.stderr).unwrap()
                );

                return false;
            }
        }
        None => println!(
            "\nError running: \n{}",
            String::from_utf8(llc_cmd.stderr).unwrap()
        ),
    }
    let clang_cmd = Command::new("clang")
        .args(&[
            "-o",
            config.build_folder.join("a.out").to_str().unwrap(),
            config.build_folder.join("out.ir.s").to_str().unwrap(),
        ])
        .output()
        .expect("failed to compile to binary");

    match clang_cmd.status.code() {
        Some(code) => {
            if code != 0 {
                println!(
                    "BUG: Cannot compile to binary: {}",
                    String::from_utf8(clang_cmd.stderr).unwrap()
                );

                return false;
            }
        }
        None => println!(
            "\nError running: \n{}",
            String::from_utf8(clang_cmd.stderr).unwrap()
        ),
    }
    true
}

fn run(config: Config) {
    if !build(&config) {
        return;
    }

    let cmd = Command::new(config.build_folder.join("a.out").to_str().unwrap())
        .output()
        .expect("failed to execute binary");

    print!("{}", String::from_utf8(cmd.stdout).unwrap());

    match cmd.status.code() {
        Some(code) => {
            std::process::exit(code);
        }
        None => println!(
            "\nError running: \n{}",
            String::from_utf8(cmd.stderr).unwrap()
        ),
    }

    std::process::exit(-1);
}

fn main() {
    let matches = App::new("Rock")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Simple toy language")
        .arg(
            Arg::with_name("verbose")
                .takes_value(false)
                .short("v")
                .help("Verbose level"),
        )
        .arg(
            Arg::with_name("tokens")
                .short("t")
                .takes_value(false)
                .help("Show tokens"),
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
        .arg(
            Arg::with_name("state")
                .short("s")
                .takes_value(false)
                .help("Show the InferContext state before solve"),
        )
        .arg(
            Arg::with_name("output-folder")
                .short("o")
                .takes_value(true)
                .default_value("./build")
                .help("Choose a different output folder"),
        )
        .subcommand(SubCommand::with_name("build").about("Build the current project directory"))
        .subcommand(SubCommand::with_name("run").about("Run the current project directory"))
        .get_matches();

    let config = rock::Config {
        verbose: matches.is_present("verbose"),
        show_tokens: matches.is_present("tokens"),
        show_ast: matches.is_present("ast"),
        show_hir: matches.is_present("hir"),
        show_ir: matches.is_present("ir"),
        show_state: matches.is_present("state"),
        build_folder: PathBuf::from(matches.value_of("output-folder").unwrap()),
        ..Default::default()
    };

    logger::init_logger();

    if let Some(_matches) = matches.subcommand_matches("build") {
        build(&config);
    } else if let Some(_matches) = matches.subcommand_matches("run") {
        run(config);
    } else {
        println!("{}", matches.usage());
    }
}
