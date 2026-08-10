use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitStatus,
};

pub(crate) fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| {
        format!(
            "Failed to create destination directory {}: {}",
            destination.display(),
            e
        )
    })?;

    for entry in fs::read_dir(source).map_err(|e| {
        format!(
            "Failed to read source directory {}: {}",
            source.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| format!("Failed to read source entry: {}", e))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect source entry {}: {}",
                source_path.display(),
                e
            )
        })?;

        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            copy_file_with_permissions(source_path, destination_path)?;
        }
    }

    Ok(())
}

pub(crate) fn copy_file_with_permissions(
    source: PathBuf,
    destination: PathBuf,
) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create destination directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    fs::copy(&source, &destination).map_err(|e| {
        format!(
            "Failed to copy file {} to {}: {}",
            source.display(),
            destination.display(),
            e
        )
    })?;

    let permissions = fs::metadata(&source)
        .map_err(|e| format!("Failed to read file metadata {}: {}", source.display(), e))?
        .permissions();
    fs::set_permissions(&destination, permissions).map_err(|e| {
        format!(
            "Failed to set file permissions on {}: {}",
            destination.display(),
            e
        )
    })
}

#[cfg(unix)]
pub(crate) fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|e| format!("Failed to inspect {}: {}", path.display(), e))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|e| format!("Failed to update {} permissions: {}", path.display(), e))
}

#[cfg(not(unix))]
pub(crate) fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

pub(crate) fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}
