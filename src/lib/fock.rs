#![feature(const_fn, associated_type_bounds)]

extern crate llvm_sys as llvm;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;
#[macro_use]
extern crate concat_idents;

use regex::Regex;
use std::fs;

#[macro_use]
pub mod ast;
#[macro_use]
pub mod infer;

pub use crate::infer::*;
mod codegen;
pub mod config;
mod error;
mod lexer;
pub mod logger;
mod parser;
mod scope;
mod tests;
mod token;
mod visit;

use self::ast::ast_print::*;
use self::codegen::*;
pub use self::config::Config;
use self::error::Error;
use self::lexer::Lexer;
use self::parser::Parser;
use self::token::*;
use self::visit::*;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<Builder, Error> {
    info!("   -> Parsing {}", in_name);

    let file = fs::read_to_string(in_name).expect("Woot");

    parse_str(file, out_name, config)
}

pub fn preprocess(input: String) -> String {
    // Add a '.' after a '@' if it is followed by some word
    // This is a dirty trick to ditch having to modiffy the parser for that sugar
    let re = Regex::new(r"@(\w)").unwrap();
    let out = re.replace_all(&input, "@.$1");

    out.to_string()
}

pub fn parse_str(input: String, output_name: String, config: Config) -> Result<Builder, Error> {
    let preprocessed = preprocess(input.clone());

    let input: Vec<char> = preprocessed.chars().collect();

    let tokens = Lexer::new(input.clone()).collect();

    let ast = Parser::new(tokens.clone(), input.clone()).run()?;
    let _infer_builder = &mut InferBuilder::new(InferState::new());

    // ast.annotate(infer_builder);

    // ast.constrain(infer_builder);

    // infer_builder.solve();

    // println!("INFER {:#?}", infer_builder);

    if config.show_ast {
        // println!("AST {:#?}", ast);
        AstPrintContext::new(tokens.clone(), input.clone()).visit_source_file(&ast);
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
