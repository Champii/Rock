#![feature(const_fn, associated_type_bounds)]

// extern crate llvm_sys as llvm;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;
#[macro_use]
extern crate concat_idents;

use std::{fs, path::PathBuf};

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
type Error = Diagnostic;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<(), Error> {
    let file = fs::read_to_string(in_name.clone()).expect("Woot");

    parse_str(
        SourceFile {
            file_path: PathBuf::from(in_name.clone()),
            mod_path: PathBuf::from(in_name),
            content: file,
        },
        out_name,
        config,
    )
}

pub fn parse_str(input: SourceFile, _output_name: String, config: Config) -> Result<(), Error> {
    let mut parsing_ctx = ParsingCtx::default();
    parsing_ctx.add_file(input.clone());

    info!("    -> Parsing");

    // Text to Ast
    let (mut ast, tokens) = parser::parse(&mut parsing_ctx)?;

    if parsing_ctx.diagnostics.must_stop {
        parsing_ctx.print_diagnostics();

        std::process::exit(-1);
    }

    // Debug trees
    if config.show_ast {
        AstPrintContext::new(tokens, input).visit_root(&ast);
    }

    info!("    -> Resolving");

    ast::resolve(&mut ast, &mut parsing_ctx);

    if parsing_ctx.diagnostics.must_stop {
        parsing_ctx.print_diagnostics();

        std::process::exit(-1);
    }

    info!("    -> Lowering to HIR");

    // Ast to Hir
    let mut hir = ast_lowering::lower_crate(&ast);

    if config.show_hir {
        println!("{:#?}", hir);
    }

    info!("    -> Infer HIR");

    // Infer Hir
    infer::infer(&mut hir);

    info!("    -> Lower to LLVM IR");

    // Generate code
    codegen::generate(&config, &hir);

    Ok(())
}

pub fn file_to_file(in_name: String, out_name: String, config: Config) -> Result<(), Error> {
    let _builder = parse_file(in_name, out_name, config)?;

    Ok(())
}

pub fn run(in_name: String, entry: String, config: Config) -> Result<u64, Error> {
    parse_file(in_name, entry, config)?;

    Ok(0)
}

pub fn run_str(input: String, entry: String, config: Config) -> Result<u64, Error> {
    info!("Parsing StdIn");

    let source = SourceFile {
        file_path: PathBuf::from(""),
        mod_path: PathBuf::from(""),
        content: input,
    };

    parse_str(source, entry, config)?;

    Ok(0)
}
