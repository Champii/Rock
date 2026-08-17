use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub file_path: PathBuf,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(file_path: PathBuf, start: usize, end: usize) -> Self {
        assert!(
            !file_path.as_os_str().is_empty(),
            "span requires a source path"
        );
        assert!(start <= end, "span start must not exceed its end");
        Self {
            file_path,
            start,
            end,
        }
    }

    #[cfg(test)]
    pub fn test() -> Self {
        Self::new(PathBuf::from("/test.rk"), 0, 0)
    }
}

impl PartialEq for Span {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Span {}
