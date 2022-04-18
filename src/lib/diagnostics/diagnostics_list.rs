use std::{collections::HashMap, path::PathBuf};

use crate::parser::SourceFile;

use super::Diagnostic;

#[derive(Debug, Default, Clone)]
pub enum DiagnosticType {
    Warning,
    #[default]
    Error,
}

#[derive(Debug, Default, Clone)]
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
            let input = match files.get(&diag.span.file_path) {
                Some(input) => input,
                None => {
                    warn!("Diagnostic has been silenced because the file is not found");

                    continue;
                }
            };

            diag.print(input, self.list_types.get(i).unwrap());
        }
    }

    pub fn append(&mut self, other: Self) {
        self.list.extend(other.list);
        self.list_types.extend(other.list_types);
        self.must_stop = self.must_stop || other.must_stop;
    }
}
