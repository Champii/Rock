use std::path::{Path, PathBuf};

pub(crate) fn output_executable_path(output_dir: &Path, entry_file: &Path) -> PathBuf {
    let module_name = entry_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("main");

    if cfg!(target_os = "windows") {
        output_dir.join(format!("{}.exe", module_name))
    } else {
        output_dir.join(module_name)
    }
}
