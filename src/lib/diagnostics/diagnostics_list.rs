use std::{collections::HashMap, path::PathBuf};

use crate::parser::SourceFile;

use super::Diagnostic;

#[derive(Debug, Default)]
pub struct Diagnostics {
    list: Vec<Diagnostic>,
    pub must_stop: bool,
}

impl Diagnostics {
    pub fn push(&mut self, diag: Diagnostic) {
        self.must_stop = true;

        self.list.push(diag);
    }

    pub fn print(&self, files: &HashMap<PathBuf, SourceFile>) {
        for diag in &self.list {
            let input = files.get(&diag.span.file_path).unwrap();

            diag.print(input);
        }
    }
}
