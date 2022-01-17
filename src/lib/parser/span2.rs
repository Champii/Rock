use std::path::PathBuf;



use crate::parser2::Parser;

// TODO: merge spans

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Span {
    file_path: PathBuf,
    offset: usize,
    line: usize,
    column: usize,
    txt: String,
}

impl<'a> From<Parser<'a>> for Span {
    fn from(source: Parser<'a>) -> Self {
        Self {
            file_path: source.extra.current_file_path().clone(),
            offset: source.location_offset(),
            line: source.location_line() as usize,
            column: source.get_column(),
            txt: source.to_string(),
        }
    }
}
