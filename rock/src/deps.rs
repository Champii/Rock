use std::path::{Path, PathBuf};

use rock_shared::manifest::Dependency;

pub(crate) fn resolve_dependency_root(
    package_root: &Path,
    dependency_name: &str,
    dependency: &Dependency,
) -> Result<PathBuf, String> {
    if let Some(path) = &dependency.path {
        let candidate = PathBuf::from(path);
        let resolved = if candidate.is_relative() {
            package_root.join(candidate)
        } else {
            candidate
        };

        return resolved.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve dependency '{}' at {}: {}",
                dependency_name,
                resolved.display(),
                e
            )
        });
    }

    if let Some(version) = &dependency.version {
        return Err(format!(
            "Registry dependency '{}@{}' is not supported yet",
            dependency_name, version
        ));
    }

    Err(format!(
        "Dependency '{}' must have either 'path' or 'version'",
        dependency_name
    ))
}
