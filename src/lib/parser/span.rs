use std::path::PathBuf;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Span {
    pub file_path: PathBuf,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(file_path: PathBuf, start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            file_path,
        }
    }

    pub fn new_placeholder() -> Self {
        Self {
            start: 0,
            end: 0,
            file_path: PathBuf::new(),
        }
    }
}
