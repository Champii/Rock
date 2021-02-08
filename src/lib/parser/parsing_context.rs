use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::diagnostics::{Diagnostic, Diagnostics};

use super::Span;

#[derive(Default, Debug, Clone)]
pub struct SourceFile {
    pub file_path: PathBuf,
    pub mod_path: PathBuf,
    pub content: String,
    // pub tokens: Vec<Token>,
}

impl SourceFile {
    // pub fn
    pub fn resolve_new(&self, name: String) -> Self {
        let mut file_path = self.file_path.parent().unwrap().join(Path::new(&name));
        file_path.set_extension("rk");
        let mut mod_path = self.mod_path.as_path().join(Path::new(&name));
        let content = fs::read_to_string(dbg!(file_path.to_str().unwrap().to_string())).unwrap();

        Self {
            file_path,
            mod_path,
            content,
        }
    }
}

#[derive(Default, Debug)]
pub struct ParsingCtx {
    files: HashMap<PathBuf, SourceFile>,
    pub current_file: Option<PathBuf>,
    pub diagnostics: Diagnostics,
}

impl ParsingCtx {
    pub fn add_file(&mut self, file: SourceFile) {
        self.current_file = Some(file.file_path.clone());

        self.files.insert(file.file_path.clone(), file.clone());
    }

    pub fn get_current_file(&self) -> SourceFile {
        self.files
            .get(&self.current_file.clone().unwrap())
            .unwrap()
            .clone()
    }

    pub fn print_diagnostics(&self) {
        self.diagnostics.print(&self.files);
    }

    pub fn new_span(&self, start: usize, end: usize) -> Span {
        Span::new(self.get_current_file().file_path.clone(), start, end)
    }

    pub fn resolve_and_add_file(&mut self, name: String) -> Result<SourceFile, Diagnostic> {
        let current_file = self.get_current_file();

        let new_file = current_file.resolve_new(name);

        self.add_file(new_file.clone());

        Ok(new_file)
    }
}
