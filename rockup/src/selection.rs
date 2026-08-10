use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    constants::{ROCKUP_TOOLCHAIN_ENV, ROCK_TOOLCHAIN_FILE},
    home::RockupHome,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveToolchainSource {
    Env,
    Project(PathBuf),
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveToolchain {
    pub(crate) name: String,
    pub(crate) source: ActiveToolchainSource,
}

pub(crate) fn active_toolchain_name(
    home: &RockupHome,
    env_override: Option<String>,
    current_dir: Option<&Path>,
) -> Result<String, String> {
    resolve_active_toolchain(home, env_override, current_dir).map(|toolchain| toolchain.name)
}

pub(crate) fn resolve_active_toolchain(
    home: &RockupHome,
    env_override: Option<String>,
    current_dir: Option<&Path>,
) -> Result<ActiveToolchain, String> {
    if let Some(name) = env_override.filter(|name| !name.trim().is_empty()) {
        return Ok(ActiveToolchain {
            name,
            source: ActiveToolchainSource::Env,
        });
    }

    if let Some(toolchain) = project_toolchain_name(current_dir)? {
        return Ok(toolchain);
    }

    let contents = fs::read_to_string(home.default_toolchain_file()).map_err(|e| {
        format!(
            "No active toolchain selected. Set one with 'rockup default <name>' or {}: {}",
            ROCKUP_TOOLCHAIN_ENV, e
        )
    })?;
    let name = contents.trim();
    if name.is_empty() {
        return Err(format!(
            "Default toolchain file {} is empty",
            home.default_toolchain_file().display()
        ));
    }

    Ok(ActiveToolchain {
        name: name.to_string(),
        source: ActiveToolchainSource::Default,
    })
}

pub(crate) fn default_toolchain_name(home: &RockupHome) -> Result<String, String> {
    let contents = fs::read_to_string(home.default_toolchain_file()).map_err(|e| {
        format!(
            "Failed to read default toolchain file {}: {}",
            home.default_toolchain_file().display(),
            e
        )
    })?;
    let name = contents.trim();
    if name.is_empty() {
        return Err(format!(
            "Default toolchain file {} is empty",
            home.default_toolchain_file().display()
        ));
    }

    Ok(name.to_string())
}

fn project_toolchain_name(current_dir: Option<&Path>) -> Result<Option<ActiveToolchain>, String> {
    let Some(current_dir) = current_dir else {
        return Ok(None);
    };

    for directory in current_dir.ancestors() {
        let toolchain_file = directory.join(ROCK_TOOLCHAIN_FILE);
        if !toolchain_file.exists() {
            continue;
        }

        let name = parse_project_toolchain_file(&toolchain_file)?;
        return Ok(Some(ActiveToolchain {
            name,
            source: ActiveToolchainSource::Project(toolchain_file),
        }));
    }

    Ok(None)
}

fn parse_project_toolchain_file(path: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read toolchain file {}: {}", path.display(), e))?;
    let value: toml::Value = toml::from_str(&contents)
        .map_err(|e| format!("Failed to parse toolchain file {}: {}", path.display(), e))?;
    let channel = value
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("channel"))
        .and_then(|channel| channel.as_str())
        .map(str::trim)
        .filter(|channel| !channel.is_empty())
        .ok_or_else(|| {
            format!(
                "Toolchain file {} must contain [toolchain] channel = \"...\"",
                path.display()
            )
        })?;

    Ok(channel.to_string())
}

pub(crate) fn current_env_toolchain() -> Option<String> {
    env::var(ROCKUP_TOOLCHAIN_ENV).ok()
}
