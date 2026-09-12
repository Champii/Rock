use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub fn file_modified(path: &Path) -> Result<SystemTime, String> {
    fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?
        .modified()
        .map_err(|e| format!("Failed to read modified time for {}: {}", path.display(), e))
}

pub fn collect_package_source_inputs(
    crate_root: &Path,
    build_dir_name: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut inputs = vec![crate_root.join("rock.toml")];
    collect_rock_sources(crate_root, crate_root, build_dir_name, &mut inputs)?;
    inputs.sort();
    inputs.dedup();
    Ok(inputs)
}

fn collect_rock_sources(
    root: &Path,
    current: &Path,
    build_dir_name: &str,
    inputs: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| {
        format!(
            "Failed to read package directory {} while checking artifact cache: {}",
            current.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read package directory entry in {} while checking artifact cache: {}",
                current.display(),
                e
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to read file type for {} while checking artifact cache: {}",
                path.display(),
                e
            )
        })?;

        if file_type.is_dir() {
            if path == root.join(build_dir_name) {
                continue;
            }
            collect_rock_sources(root, &path, build_dir_name, inputs)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rk") {
            inputs.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::collect_package_source_inputs;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_collect_package_source_inputs_includes_manifest_and_rock_sources() {
        let root = std::env::temp_dir().join(format!("rock-shared-fs-test-{}", std::process::id()));
        let src = root.join("src");
        let build = root.join("build");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&build).unwrap();
        fs::write(root.join("rock.toml"), b"[crate]").unwrap();
        fs::write(src.join("lib.rk"), b"").unwrap();
        fs::write(build.join("ignored.rk"), b"").unwrap();

        let inputs = collect_package_source_inputs(&root, "build").unwrap();

        assert_eq!(
            inputs,
            vec![root.join("rock.toml"), PathBuf::from(src.join("lib.rk"))]
        );

        let _ = fs::remove_dir_all(root);
    }
}
