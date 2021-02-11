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

    pub fn populate(&mut self) {
        // check for Build.toml
        // set project path accordingly
        let mut to_compile = vec![];
        let mut files = HashMap::new();

        to_compile.push(format!("{}/{}", &self.project_path, "src/main.rk"));
        files.insert(
            format!("{}/{}", &self.project_path, "src/main.rk"),
            "".to_string(),
        );

        self.files = files;
        self.to_compile.extend(to_compile);
    }

    fn clear_line() {
        print!("\r                                                                                                              \r");
    }

    pub fn build(&mut self) -> bool {
        let _files_len = self.files.len();

        fs::create_dir_all(self.project_path.clone() + "/.build/").unwrap();

        for file in &self.to_compile {
            io::stdout().flush().unwrap();

            if let Err(_e) =
                fock::file_to_file(file.to_string(), "".to_string(), self.config.clone())
            {
                Self::clear_line();

                return false;
            }

            Self::clear_line();
        }

        true
    }
}
