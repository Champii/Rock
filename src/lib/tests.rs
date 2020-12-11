use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

#[allow(unused)]
use super::Config;

// one possible implementation of walking a directory only visiting files
#[allow(unused)]
pub fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

#[test]
pub fn run() {
    let config = Config::default();
    // config.show_ast = true;
    // config.show_ir = true;
    visit_dirs(Path::new(&"./tests".to_string()), &|file| {
        let path = file.path();
        println!("{:?}", file);

        if let Some(ext) = path.extension() {
            if ext == "rk" {
                let content = fs::read_to_string(path).unwrap();

                let res = super::run_str(content, "main\0".to_string(), config.clone()).unwrap();

                if res != 42 {
                    println!("FAILED: {:?}", file);
                }

                assert_eq!(res as u8, 42);
            }
        }
    })
    .unwrap();
}
