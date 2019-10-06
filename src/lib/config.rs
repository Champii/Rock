#[derive(Debug, Clone, Default)]
pub struct Config {
    pub show_ast: bool,
    pub show_ir: bool,
    pub files: Vec<String>,
}
