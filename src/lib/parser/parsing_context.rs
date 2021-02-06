use crate::diagnostics::Diagnostics;

use super::Span;

#[derive(Default, Debug)]
pub struct ParsingCtx {
    files: Vec<String>,
    pub diagnostics: Diagnostics,
}

impl ParsingCtx {
    pub fn add_file(&mut self, file: String) {
        self.files.push(file);
    }

    pub fn get_current_file(&self) -> String {
        self.files.last().unwrap().clone()
    }

    pub fn get_current_file_id(&self) -> usize {
        self.files.len() - 1
    }

    pub fn print_diagnostics(&self) {
        self.diagnostics.print(&self.files);
    }

    pub fn new_span(&self, start: usize, end: usize) -> Span {
        Span::new(self.get_current_file_id(), start, end)
    }
}
