use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use walkdir::WalkDir;

fn visit_dirs(dir: &Path) -> io::Result<Vec<String>> {
    let mut res = vec![];

    if dir.is_dir() {
        for entry in read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                res.extend(visit_dirs(&path)?);
            } else if let Some(ext) = path.extension() {
                if ext == "rk" && path.file_name().unwrap() == "main.rk" {
                    res.push(entry.path().to_str().unwrap().to_string());
                }
            }
        }
    }

    Ok(res)
}

fn schedule_rerun_if_folder_changed(path: &Path) {
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();

        if entry.path() == path {
            continue;
        }

        if entry.path().is_dir() {
            schedule_rerun_if_folder_changed(&entry.path());
        } else {
            println!("cargo:rerun-if-changed={}", entry.path().to_str().unwrap());
        }
    }
}

// build script's entry point
fn main() {
    schedule_rerun_if_folder_changed(&PathBuf::from("src/lib/testcases/"));

    let out_dir = "src/lib";
    let destination = Path::new(&out_dir).join("tests.rs");
    let mut output_file = File::create(&destination).unwrap();

    // write test file header, put `use`, `const` etc there
    write_header(&mut output_file);

    for file in visit_dirs(Path::new(&"src/lib/testcases".to_string())).unwrap() {
        write_test(&mut output_file, &file);
    }
}

fn write_test(output_file: &mut File, path: &str) {
    let path = path.replace("src/lib/", "");

    let name = path.replace("./", "").replace("/", "_").replace(".rk", "");

    write!(
        output_file,
        include_str!("src/lib/testcases/test_template"),
        name = name,
        path = path
    )
    .unwrap();
}

fn write_header(output_file: &mut File) {
    write!(
        output_file,
        r##"use std::path::PathBuf;

#[allow(dead_code)]
        fn run(path: &str, input: &str, expected_ret: &str, expected_output: &str) {{
            let mut config = super::Config::default();

            config.project_config.entry_point = PathBuf::from(path);

            let expected_ret = expected_ret.parse::<i64>().unwrap();

            let (ret_code, stdout) = super::test::run(path, input.to_string(), config);

            assert_eq!(expected_ret, ret_code);
            assert_eq!(expected_output, stdout);
        }}
        "##
    )
    .unwrap();
}
