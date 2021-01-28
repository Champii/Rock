#![feature(const_fn, associated_type_bounds)]

extern crate llvm_sys as llvm;

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

pub use crate::infer::*;
mod ast_lowering;
mod codegen;
pub mod config;
mod error;
mod hir;
pub mod logger;
mod parser;
mod scope;
mod tests;

use crate::ast::ast_print::*;
use crate::ast::visit::*;
use crate::codegen::*;
pub use crate::config::Config;
use crate::error::Error;
use crate::parser::*;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<Builder, Error> {
    info!("   -> Parsing {}", in_name);

    let file = fs::read_to_string(in_name).expect("Woot");

    parse_str(file, out_name, config)
}

pub fn parse_str(input: String, output_name: String, config: Config) -> Result<Builder, Error> {
    let (ast, tokens) = parser::parse(input.clone())?;

    let _infer_builder = &mut InferBuilder::new(InferState::new());

    let hir = ast_lowering::lower_crate(&ast);

    // ast.annotate(infer_builder);

    // ast.constrain(infer_builder);

    // infer_builder.solve();

    // println!("INFER {:#?}", infer_builder);

    if config.show_ast {
        // println!("AST {:#?}", ast);
        AstPrintContext::new(tokens.clone(), input.clone()).visit_root(&ast);

        println!("{:#?}", hir);
    }

    // info!("   -> Codegen {}", output_name);
    let builder = Builder::new(&output_name, ast, config);

    // builder.build();

    // Ok(builder)
    Ok(builder)
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

    let _builder: Builder = parse_str(input, entry.clone(), config)?;

    Ok(0)
    // Ok(builder.run(&entry))
}
