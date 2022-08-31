use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub enum PackageType {
    Lib,
    Bin,
}

impl Default for PackageType {
    fn default() -> Self {
        Self::Bin
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    pub name: String,
    pub base_path: PathBuf,
    pub package_type: PackageType,
    pub externs: HashMap<String, PathBuf>, // Packages name and MetaData path
    pub entry_point: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub project_config: ProjectConfig,
    pub show_ast: bool,
    pub show_hir: bool,
    pub show_thir: bool,
    pub show_ir: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub build_folder: PathBuf,
    pub no_optimize: bool,
    pub std: bool,
}
