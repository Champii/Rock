#![feature(const_fn, associated_type_bounds)]

// extern crate llvm_sys as llvm;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;
#[macro_use]
extern crate concat_idents;

use std::fs;

#[macro_use]
pub mod ast;
#[macro_use]
pub mod infer;

use diagnostics::Diagnostic;
use parser::ParsingCtx;

pub use crate::infer::*;
mod ast_lowering;
mod codegen;
pub mod config;
mod diagnostics;
mod error;
mod hir;
pub mod logger;
mod parser;
mod scopes;
mod tests;
// mod visit_macro;

use crate::ast::ast_print::*;
use crate::ast::visit::*;
pub use crate::config::Config;
// use crate::error::Error;
type Error = Diagnostic;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<(), Error> {
    info!("   -> Parsing {}", in_name);

    let file = fs::read_to_string(in_name).expect("Woot");

    parse_str(file, out_name, config)
}

pub fn parse_str(input: String, _output_name: String, config: Config) -> Result<(), Error> {
    let mut parsing_ctx = ParsingCtx::default();
    parsing_ctx.add_file(input.clone());

    // Text to Ast
    let (mut ast, tokens) = parser::parse(&mut parsing_ctx)?;

    if parsing_ctx.diagnostics.must_stop {
        parsing_ctx.print_diagnostics();

        std::process::exit(-1);
    }

    // Debug trees
    if config.show_ast {
        AstPrintContext::new(tokens.clone(), input.clone()).visit_root(&ast);
    }

    ast::resolve(&mut ast, &mut parsing_ctx);

    if parsing_ctx.diagnostics.must_stop {
        parsing_ctx.print_diagnostics();

        std::process::exit(-1);
    }
    // Ast to Hir
    let mut hir = ast_lowering::lower_crate(&ast);

    // Infer Hir
    infer::infer(&mut hir);

    if config.show_hir {
        println!("{:#?}", hir);
    }

    // Generate code
    codegen::generate(&config, &hir);

    Ok(())
}

pub fn parse_mod(_config: Config) -> Result<(), Error> {
    Ok(())
}

pub fn parse_crate(_config: Config) -> Result<(), Error> {
    Ok(())
}

pub fn file_to_file(in_name: String, out_name: String, config: Config) -> Result<(), Error> {
    let _builder = parse_file(in_name, out_name.clone(), config)?;

    // builder.write(&out_name);

    Ok(())
}

pub fn run(in_name: String, entry: String, config: Config) -> Result<u64, Error> {
    let _builder = parse_file(in_name, entry.clone(), config)?;

    // Ok(builder.run(&entry))
    Ok(0)
}

pub fn run_str(input: String, entry: String, config: Config) -> Result<u64, Error> {
    info!("Parsing StdIn");

    let _builder = parse_str(input, entry.clone(), config)?;

    Ok(0)
    // Ok(builder.run(&entry))
}
