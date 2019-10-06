extern crate clap;
extern crate rock;

use clap::{App, Arg, SubCommand};
use std::process::Command;

use rock::Config;

fn build(config: Config) -> bool {
    if let Err(e) = rock::file_to_file("./main.rk".to_string(), "./main.o\0".to_string(), config) {
        println!("{}", e);

        return false;
    }

    Command::new("clang")
        .arg("main.o")
        .output() 
        .expect("failed to execute process");

    true
}

fn compile(config: Config) -> bool {
    let mut out = vec![];

    for file in &config.files {
        let mut splitted: Vec<String> = file.split(".").map(|x| x.to_string()).collect();
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
        
        if let Err(e) = rock::file_to_file(file.to_string(), out_file, config.clone()) {
            println!("{}", e);

            return false;
        }
    }

    Command::new("clang")
        .args(out)
        .output()
        .expect("failed to execute process");

    true
}

fn run() {
    if !build(Config::default()) {
        return;
    }

    let cmd = Command::new("./a.out")
        .output()
        .expect("failed to execute process");

    print!("{}", String::from_utf8(cmd.stdout).unwrap());

    match cmd.status.code() {
        Some(code) => std::process::exit(code),
        None => println!("\nError running: \n{}", String::from_utf8(cmd.stderr).unwrap()),
    }

    std::process::exit(-1);
}

fn main() {
    let matches = App::new("rock")
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
        .subcommand(
            SubCommand::with_name("compile")
                .about("Compile given files")
                .version("0.0.1")
                .author("Champii <contact@champii.io>")
                .arg(Arg::with_name("files")
                    .multiple(true)
                    .help("Files to compile"))
                .arg(Arg::with_name("ast")
                    .short("a")
                    .takes_value(false)
                    .help("Show ast"))
                .arg(Arg::with_name("ir")
                    .short("i")
                    .takes_value(false)
                    .help("Show the generated IR")),
        )
        .get_matches();

    let mut config = rock::Config::default();

    if let Some(_matches) = matches.subcommand_matches("build") {
        build(config);

        return;
    }

    if let Some(_matches) = matches.subcommand_matches("run") {
        run();

        return;
    }

    if let Some(matches) = matches.subcommand_matches("compile") {
        config.show_ast = matches.is_present("ast");
        config.show_ir = matches.is_present("ir");
        config.files = matches.values_of("files").unwrap().map(|x| x.to_string()).collect();

        compile(config);
    }
}
