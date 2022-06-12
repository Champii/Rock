use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::diagnostics::Diagnostic;

use super::span::Span;

#[derive(Default, Debug, Clone)]
pub struct SourceFile {
    pub file_path: PathBuf,
    pub mod_path: PathBuf,
    pub content: String,
}

impl SourceFile {
    pub fn from_file(in_name: String) -> Result<Self, Diagnostic> {
        let content = if let Some(content) = super::STDLIB_FILES.get(&in_name) {
            content.to_string()
        } else {
            fs::read_to_string(in_name.clone()).map_err(|_| {
                Diagnostic::new_file_not_found(Span::new_placeholder(), in_name.clone())
            })?
        };

        let content = Self::filter_content(&content);

        let content = content.replace("^[ \t]*\n", "\n");

        let mut mod_path = PathBuf::from(in_name.clone());

        mod_path.set_extension("");

        Ok(SourceFile {
            file_path: PathBuf::from(in_name),
            mod_path,
            content,
        })
    }

    pub fn from_str(path: &str, content: &str) -> Result<Self, Diagnostic> {
        let mut mod_path = PathBuf::from(path.clone());

        mod_path.set_extension("");

        let content = Self::filter_content(&content);

        Ok(SourceFile {
            file_path: PathBuf::from(path.clone()),
            mod_path,
            content,
        })
    }

    pub fn from_expr(
        top_levels: String,
        mut expr: String,
        do_print: bool,
    ) -> Result<Self, Diagnostic> {
        let print_str = if do_print { "print " } else { "" };

        if expr.is_empty() {
            expr = "  0".to_string();
        }

        let top_levels = r##"mod lib
use lib::prelude::(*)
"##
        .to_owned()
            + &top_levels
            + r##"

main =
  "## + &print_str.to_string()
            + &r##"custom()
  0
custom =
"##
            .to_owned()
            + &expr;

        let content = Self::filter_content(&top_levels);

        Ok(SourceFile {
            file_path: PathBuf::from("./src/main.rk"),
            mod_path: PathBuf::from("root"),
            content,
        })
    }

    pub fn resolve_new(&self, name: String) -> Result<Self, String> {
        let mut file_path = self.file_path.parent().unwrap().join(Path::new(&name));

        file_path.set_extension("rk");

        let mod_path = self.mod_path.as_path().join(Path::new(&name));

        let content = match fs::read_to_string(file_path.to_str().unwrap().to_string()) {
            Ok(content) => content,
            Err(_) => return Err(mod_path.as_path().to_str().unwrap().to_string()),
        };

        let content = Self::filter_content(&content);

        Ok(Self {
            file_path,
            mod_path,
            content,
        })
    }

    // This function replace the lines containing only whitespaces and tabs with a newline
    // And append a \n at the end of file to avoid out of bounds error as the parser requires
    // a newline at the end of the file
    fn filter_content(content: &str) -> String {
        Regex::new(r"[ \t]+\n")
            .unwrap()
            .replace_all(content, "\n\n")
            .to_string()
            + "\n"
    }
}
