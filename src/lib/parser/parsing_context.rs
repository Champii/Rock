use std::{
    collections::{BTreeMap, HashMap},
    path::{Component, PathBuf},
};

use colored::*;

use crate::{
    ast::{Identifier, NodeId},
    diagnostics::{Diagnostic, DiagnosticType, Diagnostics},
    parser::span::Span,
    Config,
};

use super::SourceFile;

#[derive(Default, Debug)]
pub struct ParsingCtx {
    pub files: HashMap<PathBuf, SourceFile>,
    pub config: Config,
    pub current_file: Option<PathBuf>,
    pub diagnostics: Diagnostics,
    pub operators_list: HashMap<String, u8>,
    pub identities: BTreeMap<NodeId, Span>,
}

impl ParsingCtx {
    pub fn new(config: &Config) -> Self {
        ParsingCtx {
            config: config.clone(),
            ..Default::default()
        }
    }

    pub fn add_file(&mut self, file: &SourceFile) {
        self.current_file = Some(file.file_path.clone());

        self.files.insert(file.file_path.clone(), file.clone());
    }

    pub fn get_current_file(&self) -> SourceFile {
        self.files
            .get(&self.current_file.clone().unwrap())
            .unwrap()
            .clone()
    }

    pub fn print_diagnostics(&self) {
        if self.config.quiet {
            return;
        }

        self.diagnostics.print(&self.files);
    }

    pub fn print_success_diagnostics(&self) {
        if self.config.quiet {
            return;
        }

        self.print_diagnostics();

        if !self.diagnostics.list.is_empty() {
            let diag_type_str = format!(
                "{}{}{}",
                "[".bright_black(),
                "Success".green(),
                "]".bright_black(),
            );

            println!(
                "{} {} {} {} {} {}",
                diag_type_str,
                "Compilation".bright_black(),
                "successful".bright_green(),
                "with".bright_black(),
                self.diagnostics.list.len().to_string().yellow(),
                "warnings".bright_yellow(),
            );
        } else if self.config.verbose {
            let diag_type_str = format!(
                "{}{}{}",
                "[".bright_black(),
                "Success".green(),
                "]".bright_black(),
            );

            println!(
                "{} {}",
                diag_type_str,
                "Compilation successful".bright_black(),
            );
        }
    }

    pub fn return_if_error(&self) -> Result<(), Diagnostic> {
        if self.diagnostics.must_stop {
            self.print_diagnostics();

            let (errors, warnings): (Vec<_>, Vec<_>) = self
                .diagnostics
                .list
                .iter()
                .enumerate()
                .partition(|(i, _diag)| {
                    matches!(
                        *self.diagnostics.list_types.get(*i).unwrap(),
                        DiagnosticType::Error
                    )
                });

            let diag_type_str = format!(
                "{}{}{}",
                "[".bright_black(),
                "Error".red(),
                "]".bright_black(),
            );

            if !self.config.quiet {
                println!(
                    "{} {} {} {} {} {} {} {} {}",
                    diag_type_str,
                    "Compilation".bright_black(),
                    "stopped".bright_red(),
                    "with".bright_black(),
                    errors.len().to_string().red(),
                    "errors".bright_red(),
                    "and".bright_black(),
                    warnings.len().to_string().yellow(),
                    "warnings".bright_yellow(),
                );
            }

            return Err(Diagnostic::new_empty());
        }

        Ok(())
    }

    pub fn new_span(&self, start: usize, end: usize) -> Span {
        Span {
            file_path: self.get_current_file().file_path,
            start,
            end,
        }
    }

    pub fn resolve_and_add_file(&mut self, name: String) -> Result<SourceFile, Diagnostic> {
        let current_file = self.get_current_file();

        let new_file = current_file.resolve_new(name).map_err(|m| {
            // Placeholder span, to be overriden by calling mod (TopLevel::parse())
            Diagnostic::new_module_not_found(
                Span {
                    file_path: current_file.file_path.clone(),
                    start: 0,
                    end: 0,
                }
                .into(),
                m,
            )
        })?;

        if self.config.verbose {
            println!(
                " -> Compiling {}",
                new_file
                    .mod_path
                    .components()
                    .map(|m| {
                        match m {
                            Component::RootDir => "main",
                            Component::Normal(m) => m.to_str().unwrap(),
                            _ => "",
                        }
                        .green()
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(" -> "),
            );
        }

        self.add_file(&new_file);

        Ok(new_file)
    }

    pub fn operator_exists(&self, name: &Identifier) -> bool {
        self.operators_list.contains_key(&name.name)
    }
}
