extern crate clap;
extern crate rock;

#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub mod logger;

use rock::diagnostics::DiagnosticKind;
pub(crate) use rock::*;

fn build(config: &Config) -> bool {
    debug!(" -> Building");

    let entry_file = "./src/main.rk";

    fs::create_dir_all(config.build_folder.clone()).unwrap();

    if let Err(diagnostic) = rock::compile_file(entry_file.to_string(), config) {
        if let DiagnosticKind::NoError = diagnostic.get_kind() {
        } else {
            println!("Error: {}", diagnostic.get_kind());
        }

        return false;
    }

    let clang_cmd = Command::new("clang")
        .args(&[
            config.build_folder.join("out.bc").to_str().unwrap(),
            "-o",
            config.build_folder.join("a.out").to_str().unwrap(),
        ])
        .output()
        .expect("failed to compile to ir");

    match clang_cmd.status.code() {
        Some(code) => {
            if code != 0 {
                println!(
                    "BUG: Cannot compile: \n{}",
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

    print!("{}", unsafe { String::from_utf8_unchecked(cmd.stdout) });

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
        .about("A compiler for the Rock programming language")
        .arg(
            Arg::with_name("verbose")
                .takes_value(false)
                .short("v")
                .long("verbose")
                .help("Verbose level"),
        )
        .arg(
            Arg::with_name("ast")
                .short("a")
                .long("ast")
                .takes_value(false)
                .help("Show ast"),
        )
        .arg(
            Arg::with_name("hir")
                .short("h")
                .long("hir")
                .takes_value(false)
                .help("Show hir"),
        )
        .arg(
            Arg::with_name("thir")
                .short("t")
                .long("thir")
                .takes_value(false)
                .help("Show typed hir after monomorphization"),
        )
        .arg(
            Arg::with_name("no-optimize")
                .short("N")
                .long("no-optimize")
                .takes_value(false)
                .help("Disable LLVM optimization passes"),
        )
        .arg(
            Arg::with_name("ir")
                .short("i")
                .long("ir")
                .takes_value(false)
                .help("Show the generated IR"),
        )
        .arg(
            Arg::with_name("nostd")
                .long("nostd")
                .takes_value(false)
                .help("Does not include stdlib"),
        )
        .arg(
            Arg::with_name("output-folder")
                .short("o")
                .long("output-folder")
                .takes_value(true)
                .default_value("./build")
                .help("Choose a different output folder"),
        )
        .subcommand(SubCommand::with_name("build").about("Build the current project directory"))
        .subcommand(SubCommand::with_name("run").about("Run the current project directory"))
        .subcommand(
            SubCommand::with_name("new")
                .about("Create a new empty project folder")
                .arg(
                    Arg::with_name("name")
                        .required(true)
                        .help("The name of the new project"),
                ),
        )
        .get_matches();

    let config = rock::Config {
        verbose: matches.is_present("verbose"),
        show_ast: matches.is_present("ast"),
        show_hir: matches.is_present("hir"),
        show_thir: matches.is_present("thir"),
        show_ir: matches.is_present("ir"),
        no_optimize: matches.is_present("no-optimize"),
        build_folder: PathBuf::from(matches.value_of("output-folder").unwrap()),
        std: !matches.is_present("nostd"),
        ..Default::default()
    };

    logger::init_logger();

    if let Some(_matches) = matches.subcommand_matches("build") {
        build(&config);
    } else if let Some(_matches) = matches.subcommand_matches("run") {
        run(config);
    } else if let Some(matches) = matches.subcommand_matches("new") {
        create_project_folder(matches.value_of("name").unwrap());
    /* } else if config.repl {
    run(config) */
    } else {
        println!("{}", matches.usage());
    }
}

fn create_project_folder(name: &str) {
    let path = Path::new(name);

    if path.exists() {
        println!("Error: {} already exists", name);
        return;
    }

    fs::create_dir(path).expect("Failed to create project folder");
    fs::create_dir(path.join("src")).expect("Failed to create project src folder");

    let mut file = File::create(path.join("src/main.rk")).expect("Failed to create main.rk");

    file.write(b"main: -> \"Hello World !\".print!").unwrap();
}
