#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    pub name: String,
    pub base_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub project_config: ProjectConfig,
    pub show_ast: bool,
    pub show_ir: bool,
    pub files: Vec<String>,
    pub verbose: u8,
}
