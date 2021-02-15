extern crate clap;
extern crate fock;

#[macro_use]
extern crate log;

use clap::{App, Arg, SubCommand};
use std::convert::TryInto;
use std::process::Command;

use fock::logger;

pub(crate) use fock::*;

mod builder;

fn build(config: Config) -> bool {
    info!(" -> Building");

    let mut builder = builder::Builder::new(config, '.'.to_string());

    builder.populate();

    builder.build();

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

fn compile(config: Config) -> bool {
    info!(" -> Compiling");

    let mut out = vec![];

    for file in &config.files {
        let mut splitted: Vec<String> = file.split('.').map(|x| x.to_string()).collect();
        let len = splitted.len();
        let ext = splitted[len - 1].clone();

        if ext != "rk" {
            println!("Bad file extension: {}", file);

            return false;
        }

        splitted[len - 1] = "o".to_string();

        let mut out_file = splitted.join(".");

        out.push(out_file.clone());

        out_file += &"\0".to_string();

        if let Err(_e) = fock::file_to_file(file.to_string(), out_file, config.clone()) {
            // println!("{}", e);

            return false;
        }
    }

    info!(" -> Linking");

    Command::new("clang")
        .args(out)
        .output()
        .expect("failed to execute process");

    true
}

fn run_file(config: Config) {
    info!(" -> Running file");

    let res = fock::run(config.files[0].clone(), "main\0".to_owned(), config);

    match res {
        Ok(res) => std::process::exit(res.try_into().unwrap()),
        // Err(err) => println!("{:?}", err),
        Err(_err) => (),
    }
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
        .subcommand(
            SubCommand::with_name("runfile")
                .about("Run given files")
                .version("0.0.1")
                .author("Champii <contact@champii.io>")
                .arg(Arg::with_name("files").multiple(true).help("Files to run")),
        )
        .subcommand(
            SubCommand::with_name("compile")
                .about("Compile given files")
                .version("0.0.1")
                .author("Champii <contact@champii.io>")
                .arg(
                    Arg::with_name("files")
                        .multiple(true)
                        .help("Files to compile"),
                ),
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

    if let Some(matches) = matches.subcommand_matches("runfile") {
        config.files = matches
            .values_of("files")
            .unwrap()
            .map(|x| x.to_string())
            .collect();

        run_file(config);

        return;
    }

    if let Some(matches) = matches.subcommand_matches("compile") {
        config.files = matches
            .values_of("files")
            .unwrap()
            .map(|x| x.to_string())
            .collect();

        compile(config);
    }
}
