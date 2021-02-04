use std::collections::HashMap;
use std::io::Write;
use std::{fs, io};

use super::Config;

pub struct Builder {
    pub project_path: String,
    pub config: Config,
    pub to_compile: Vec<String>,
    pub out_files: Vec<String>,
    pub files: HashMap<String, String>, //<In, Out>
}

impl Builder {
    pub fn new(config: Config, project_path: String) -> Builder {
        Builder {
            config,
            project_path,
            out_files: vec![],
            to_compile: vec![],
            files: HashMap::new(),
        }
    }

    // fn create_dirs(&self) {
    //     fs::create_dir_all(self.project_path.clone() + "/.build/");
    // }

    // pub fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&DirEntry)) -> io::Result<()> {
    //     if dir.is_dir() {
    //         for entry in fs::read_dir(dir)? {
    //             let entry = entry?;
    //             let path = entry.path();
    //             if path.is_dir() {
    //                 Self::visit_dirs(&path, cb)?;
    //             } else {
    //                 cb(&entry);
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    // fn is_more_recent(item: &Path, to_test: &Path) -> std::io::Result<bool> {
    //     Ok(fs::metadata(item)?.modified()? > fs::metadata(to_test)?.modified()?)
    // }

    pub fn populate(&mut self) {
        // check for Build.toml
        // set project path accordingly
        let mut to_compile = vec![];
        let mut files = HashMap::new();
        // let mut out_files = vec![];

        // if !Path::new("./Rock.toml").exists() {
        //     panic!("No 'Rock.toml' file found.");
        // }

        to_compile.push(format!("{}/{}", &self.project_path, "src/main.rk"));
        files.insert(
            format!("{}/{}", &self.project_path, "src/main.rk"),
            "".to_string(),
        );

        // Self::visit_dirs(Path::new(&self.project_path), &mut |item| {
        //     let path = item.path();
        //     let path_str = path.to_str().unwrap();

        //     let out_path = Path::new("./.build/").join(path.with_extension("o"));

        //     if let Some(ext) = path.extension() {
        //         if ext == "rk" {
        //             files.insert(path_str.to_string(), out_path.to_str().unwrap().to_string());

        //             match Self::is_more_recent(&path.clone(), &path.with_extension("o")) {
        //                 Ok(true) | Err(_) => to_compile.push(path_str.to_string()),
        //                 _ => (),
        //             }

        //             out_files.push(out_path.to_str().unwrap().to_string());
        //         }
        //     }
        // })
        // .unwrap();

        self.files = files;
        // self.out_files.extend(out_files);
        self.to_compile.extend(to_compile);
    }

    fn progress_bar(done: i32, total: i32) -> String {
        let mut res = String::new();

        res += "[";
        let size = 50;

        let ratio = done * size / total;

        let mut i = 0;
        while i < size {
            if i < ratio {
                res += "=";
            } else if i == ratio {
                res += ">";
            } else {
                res += " ";
            }
            i += 1;
        }

        res += "]";

        res
    }

    fn clear_line() {
        print!("\r                                                                                                              \r");
    }

    pub fn build(&mut self) -> bool {
        let files_len = self.files.len();
        let mut i = files_len - self.to_compile.len();

        fs::create_dir_all(self.project_path.clone() + "/.build/").unwrap();

        for file in &self.to_compile {
            // let out_file = self.files.get(file).unwrap();
            // let mut splitted: Vec<String> = file.split(".").map(|x| x.to_string()).collect();
            // let len = splitted.len();

            // splitted[len - 1] = "o".to_string();

            // let mut out_file = splitted.join(".");

            // fs::create_dir_all(Path::new(out_file).parent().unwrap()).unwrap();

            // let out_file = out_file.to_owned() + &"\0".to_string();

            print!(
                "    Building: {} {}/{}: {} ",
                Self::progress_bar(i as i32, files_len as i32),
                i,
                files_len,
                file.to_string()
            );

            io::stdout().flush().unwrap();

            if let Err(e) =
                fock::file_to_file(file.to_string(), "".to_string(), self.config.clone())
            {
                Self::clear_line();

                println!("\n   Error: {}", e);

                return false;
            }

            Self::clear_line();

            println!("\r    Compiled: {}..", file.to_string());
            i += 1;
        }

        true
    }
}
