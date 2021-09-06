use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use crate::hir;

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

// TBD
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageMetaData {
    pub hir: hir::Root,
}

impl PackageMetaData {
    pub fn load(path: &Path) -> bincode::Result<Self> {
        bincode::deserialize_from(BufReader::new(File::open(path).unwrap()))
    }

    pub fn store(&self, path: &Path) -> bincode::Result<()> {
        bincode::serialize_into(BufWriter::new(File::create(path).unwrap()), self)
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
    pub show_tokens: bool,
    pub show_ast: bool,
    pub show_hir: bool,
    pub show_ir: bool,
    pub show_state: bool,
    pub verbose: u8,
    pub build_folder: PathBuf,
}
