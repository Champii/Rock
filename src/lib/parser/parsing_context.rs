use colored::*;
use std::{collections::HashMap, path::PathBuf};

use crate::{
    ast::{ast_print::AstPrintContext, Identifier, Root},
    diagnostics::{Diagnostic, Diagnostics},
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
}

impl ParsingCtx {
    pub fn new(config: &Config) -> Self {
        ParsingCtx {
            config: config.clone(),
            ..Default::default()
        }
    }

    // pub fn get_span_from_hir_id(&self, hir_id: HirId) -> Span {}

    pub fn add_file(&mut self, file: SourceFile) {
        self.current_file = Some(file.file_path.clone());

        self.files.insert(file.file_path.clone(), file);
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

    pub fn return_if_error(&self) -> Result<(), Diagnostic> {
        if self.diagnostics.must_stop {
            self.print_diagnostics();

            println!(
                "[{}] Compilation stoped with {} {} emitted",
                "Error".red(),
                self.diagnostics.list.len().to_string().red(),
                "errors".red(),
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

        let new_file = current_file.resolve_new(name).map_err(|_| {
            // Placeholder span, to be overriden by calling mod (TopLevel::parse())
            Diagnostic::new_module_not_found(Span::new(current_file.file_path.clone(), 0, 0))
        })?;

        self.add_file(new_file.clone());

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

    pub fn print_ast(&self, ast: &Root) {
        use crate::ast::visit::Visitor;

        AstPrintContext::new(ast.r#mod.tokens.clone(), self.get_current_file()).visit_root(ast);
    }
}
