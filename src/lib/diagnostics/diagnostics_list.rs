use std::{collections::HashMap, path::PathBuf};

use crate::parser::SourceFile;

use super::Diagnostic;

#[derive(Debug, Default)]
pub enum DiagnosticType {
    Warning,
    #[default]
    Error,
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    pub list: Vec<Diagnostic>,
    pub list_types: Vec<DiagnosticType>,
    pub must_stop: bool,
}

impl Diagnostics {
    pub fn push_error(&mut self, diag: Diagnostic) {
        self.must_stop = true;

        trace!("Push error diagnostic: {:#?}", diag);

        self.list.push(diag);
        self.list_types.push(DiagnosticType::Error);
    }

    pub fn push_warning(&mut self, diag: Diagnostic) {
        trace!("Push warning: {:#?}", diag);

        self.list.push(diag);
        self.list_types.push(DiagnosticType::Warning);
    }

    pub fn print(&self, files: &HashMap<PathBuf, SourceFile>) {
        for (i, diag) in self.list.iter().enumerate() {
            let input = files.get(&diag.span.file_path).unwrap();

            diag.print(input, self.list_types.get(i).unwrap());
        }
    }
}
