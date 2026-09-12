use std::path::Path;

use super::{CrateContext, CrateManifest};

impl CrateContext {
    pub fn load_manifest(manifest_path: &Path) -> Result<CrateManifest, String> {
        rock_shared::manifest::load_manifest(manifest_path)
    }

    pub fn parse_rock_toml(content: &str) -> Result<CrateManifest, String> {
        rock_shared::manifest::parse_rock_toml(content)
    }
}
