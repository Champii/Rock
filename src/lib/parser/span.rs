use crate::parser::Parser;
use std::path::PathBuf;

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

impl<'a> From<Parser<'a>> for Span {
    fn from(source: Parser<'a>) -> Self {
        Self {
            start: source.location_offset(),
            end: source.to_string().len() + source.location_offset(),
            file_path: source.extra.current_file_path().clone(),
        }
    }
}
