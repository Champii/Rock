use std::path::PathBuf;
use super::span2::Span as Span2;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

impl From<Span2> for Span {
    fn from(span: Span2) -> Self {
        Self {
            start: span.offset,
            end: span.txt.len() + span.offset,
            file_path: span.file_path,
        }
    }
}
