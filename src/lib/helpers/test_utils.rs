use crate::{parser::SourceFile, Config};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn build(input: String, config: Config) -> bool {
    let file = SourceFile {
        file_path: PathBuf::from("src/lib").join(&config.project_config.entry_point),
        mod_path: PathBuf::from("main"),
        content: input,
    };

    if let Err(_e) = crate::compile_str(&file, &config) {
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

pub fn run(path: &str, input: String, config: Config) -> (i64, String) {
    let path = Path::new("src/lib/").join(path);

    let build_path = path.parent().unwrap().join("build");

    let mut config = config;
    config.build_folder = build_path;

    fs::create_dir_all(config.build_folder.clone()).unwrap();

    if !build(input, config.clone()) {
        return (-1, String::new());
    }

    let cmd = Command::new(config.build_folder.join("a.out").to_str().unwrap())
        .output()
        .expect("failed to execute BINARY");

    let stdout = String::from_utf8(cmd.stderr).unwrap();

    fs::remove_dir_all(config.build_folder).unwrap();

    match cmd.status.code() {
        Some(code) => (code.into(), stdout),
        None => (-1, stdout),
    }
}
