use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    artifact::{collect_dependency_artifacts, ArtifactBuildState},
    bundled_sysroot::{
        ensure_sysroot_stdlib_available, package_requires_sysroot_stdlib, STDLIB_CRATE_NAME,
    },
    compile::output_executable_path,
    package::Package,
    rockc::{build_root_invocation, resolve_rockc_path, run_invocation, ExternArtifact},
};

pub(crate) fn build_project(project_root: &Path) -> Result<PathBuf, String> {
    let package = Package::load(project_root.to_path_buf())?;
    let mut state = ArtifactBuildState::default();
    let extern_artifacts = collect_dependency_artifacts(&package, &mut state)?;
    let output_dir = package.build_dir();
    fs::create_dir_all(&output_dir).map_err(|e| {
        format!(
            "Failed to create build directory {}: {}",
            output_dir.display(),
            e
        )
    })?;

    let mut artifacts = extern_artifacts
        .iter()
        .map(|(name, path)| ExternArtifact {
            name: name.clone(),
            path: path.clone(),
        })
        .collect::<Vec<_>>();
    if package_requires_sysroot_stdlib(&package) {
        let layout = ensure_sysroot_stdlib_available()?;
        artifacts.push(ExternArtifact {
            name: STDLIB_CRATE_NAME.to_string(),
            path: layout.stdlib_artifact,
        });
    }
    if state.dependency_artifacts_use_stdlib(&extern_artifacts) {
        let stdlib_artifact = ensure_sysroot_stdlib_available()?.stdlib_artifact;
        if !artifacts
            .iter()
            .any(|artifact| artifact.path == stdlib_artifact)
        {
            artifacts.push(ExternArtifact {
                name: STDLIB_CRATE_NAME.to_string(),
                path: stdlib_artifact,
            });
        }
    }

    let invocation = build_root_invocation(resolve_rockc_path()?, &package, &artifacts);
    run_invocation(
        invocation,
        &format!(
            "root executable for crate '{}'",
            package.manifest.crate_.name
        ),
    )?;

    Ok(output_executable_path(&output_dir, &package.entry_file()))
}

pub(crate) fn run_project(project_root: &Path, args: &[String]) -> Result<i32, String> {
    let executable = build_project(project_root)?;
    let status = std::process::Command::new(&executable)
        .args(args)
        .status()
        .map_err(|e| format!("Failed to run {}: {}", executable.display(), e))?;

    Ok(status.code().unwrap_or(1))
}
