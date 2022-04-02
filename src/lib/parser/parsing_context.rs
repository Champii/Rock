use std::{
    collections::HashMap,
    path::{Component, PathBuf},
};

use colored::*;

use crate::{
    ast::{ast_print::AstPrintContext, identity2::Identity, Identifier, Root, NodeId},
    diagnostics::{Diagnostic, DiagnosticType, Diagnostics},
    Config,
};

use super::{SourceFile, Span};

#[derive(Default, Debug)]
pub struct ParsingCtx {
    files: HashMap<PathBuf, SourceFile>,
    pub config: Config,
    pub current_file: Option<PathBuf>,
    pub diagnostics: Diagnostics,
    pub operators_list: HashMap<String, u8>,
    pub identities: HashMap<NodeId, Identity>,
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
        self.diagnostics.print(&self.files);
    }

    pub fn print_success_diagnostics(&self) {
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

            return Err(Diagnostic::new_empty());
        }

        Ok(())
    }

    pub fn new_span(&self, start: usize, end: usize) -> Span {
        Span::new(self.get_current_file().file_path, start, end)
    }

    pub fn resolve_and_add_file(&mut self, name: String) -> Result<SourceFile, Diagnostic> {
        let current_file = self.get_current_file();

        let new_file = current_file.resolve_new(name).map_err(|m| {
            // Placeholder span, to be overriden by calling mod (TopLevel::parse())
            Diagnostic::new_module_not_found(Span::new(current_file.file_path.clone(), 0, 0), m)
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

    pub fn add_operator(&mut self, name: &Identifier, precedence: u8) -> Result<(), Diagnostic> {
        if self.operator_exists(name) {
            return Err(Diagnostic::new_duplicated_operator(
                name.identity.span.clone(),
            ));
        }

        self.operators_list.insert(name.name.clone(), precedence);

        Ok(())
    }

    pub fn operator_exists(&self, name: &Identifier) -> bool {
        self.operators_list.contains_key(&name.name)
    }

    #[allow(dead_code)]
    pub fn print_ast(&self, ast: &Root) {
        use crate::ast::visit::Visitor;

        AstPrintContext::new().visit_root(ast);
    }
}
