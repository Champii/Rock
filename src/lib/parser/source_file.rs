use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Default, Debug, Clone)]
pub struct SourceFile {
    pub file_path: PathBuf,
    pub mod_path: PathBuf,
    pub content: String,
}

impl SourceFile {
    pub fn from_file(in_name: String) -> Self {
        let file = fs::read_to_string(in_name.clone()).expect("Woot");

        SourceFile {
            file_path: PathBuf::from(in_name.clone()),
            mod_path: PathBuf::from(in_name),
            content: file,
        }
    }

    pub fn resolve_new(&self, name: String) -> Result<Self, ()> {
        let mut file_path = self.file_path.parent().unwrap().join(Path::new(&name));

        file_path.set_extension("rk");

        let mod_path = self.mod_path.as_path().join(Path::new(&name));

        let content = match fs::read_to_string(file_path.to_str().unwrap().to_string()) {
            Ok(content) => content,
            Err(_) => return Err(()),
        };

        Ok(Self {
            file_path,
            mod_path,
            content,
        })
    }
}
