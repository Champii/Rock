#![feature(const_fn, associated_type_bounds)]

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;
#[macro_use]
extern crate concat_idents;

#[macro_use]
pub mod ast;
#[macro_use]
pub mod infer;
#[macro_use]
pub mod helpers;

use diagnostics::Diagnostic;
use parser::{ParsingCtx, SourceFile};

pub use crate::infer::*;
mod ast_lowering;
mod codegen;
mod diagnostics;
mod hir;
mod parser;
mod tests;

use crate::ast::ast_print::*;
use crate::ast::visit::*;
pub use crate::helpers::config::Config;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<(), Diagnostic> {
    parse_str(SourceFile::from_file(in_name), out_name, config)
}

pub fn parse_str(
    input: SourceFile,
    _output_name: String,
    config: Config,
) -> Result<(), Diagnostic> {
    let mut parsing_ctx = ParsingCtx::default();

    //     input.content = r#"__compiler_add a b = __compiler_add a b
    // "#
    //     .to_owned()
    //         + &input.content;

    parsing_ctx.add_file(input.clone());

    // Text to Ast
    info!("    -> Parsing");
    let (mut ast, tokens) = parser::parse_root(&mut parsing_ctx)?;

    // Debug trees
    if config.show_ast {
        AstPrintContext::new(tokens, input).visit_root(&ast);
    }

    info!("    -> Resolving");
    ast::resolve(&mut ast, &mut parsing_ctx)?;

    info!("    -> Lowering to HIR");
    // Ast to Hir
    let mut hir = ast_lowering::lower_crate(&ast);

    if config.show_hir {
        println!("{:#?}", hir);
    }

    // Infer Hir
    info!("    -> Infer HIR");
    infer::infer(&mut hir);

    // Generate code
    info!("    -> Lower to LLVM IR");
    codegen::generate(&config, &hir);

    Ok(())
}

pub mod test {
    use super::*;
    use crate::{parser::SourceFile, Config};
    use std::{path::PathBuf, process::Command};

    fn build(input: String, config: Config) -> bool {
        let file = SourceFile {
            file_path: PathBuf::from(""),
            mod_path: PathBuf::from(""),
            content: input,
        };

        if let Err(_e) = parse_str(file, "main".to_string(), config.clone()) {
            return false;
        }

        Command::new("llc")
            .args(&["out.ir"])
            .output()
            .expect("failed to execute process");

        Command::new("clang")
            .args(&["out.ir.s"])
            .output()
            .expect("failed to execute process");

        true
    }

    pub fn run(input: String, config: Config) -> i64 {
        if !build(input, config) {
            return -1;
        }

        let cmd = Command::new("./a.out")
            .output()
            .expect("failed to execute process");

        match cmd.status.code() {
            Some(code) => code.into(),
            None => -1,
        }
    }
}
