#[derive(Clone, Debug, Default)]
pub struct Span {
    pub file_id: usize,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(file_id: usize, start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            file_id,
        }
    }
}
