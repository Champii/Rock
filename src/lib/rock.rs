#![feature(const_fn, associated_type_bounds)]

extern crate llvm_sys as llvm;

#[macro_use]
extern crate log;

use regex::Regex;
use std::fs;

mod ast;
mod codegen;
mod config;
mod context;
// mod desugar;
mod error;
mod generator;
mod lexer;
pub mod logger;
mod parser;
mod scope;
mod tests;
mod token;
mod type_checker;

// use self::ast::*;
use self::codegen::Builder;
pub use self::config::Config;
use self::error::Error;
use self::generator::Generator;
use self::lexer::Lexer;
use self::parser::Parser;
use self::token::{Token, TokenType};
use self::type_checker::TypeChecker;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<Builder, Error> {
    info!("   -> Parsing {}", in_name);

    let file = fs::read_to_string(in_name).expect("Woot");

    parse_str(file, out_name, config)
}

pub fn preprocess(input: String) -> String {
    // Add a '.' after a '@' if it is followed by some word
    let re = Regex::new(r"@(\w)").unwrap();
    let out = re.replace_all(&input, "@.$1");

    out.to_string()
}

pub fn parse_str(input: String, output_name: String, config: Config) -> Result<Builder, Error> {
    let preprocessed = preprocess(input.clone());

    let input: Vec<char> = preprocessed.chars().collect();

    let lexer = Lexer::new(input.clone());

    let ast = Parser::new(lexer).run()?;

    let mut tc = TypeChecker::new(ast);

    tc.ctx.input = input.clone();

    tc.infer()?;

    let ast = Generator::new(tc.ast, tc.ctx).generate()?;

    if config.show_ast {
        println!("AST {:#?}", ast);
    }

    let mut builder = Builder::new(&output_name, ast, config);

    builder.build();

    Ok(builder)
}

pub fn file_to_file(in_name: String, out_name: String, config: Config) -> Result<(), Error> {
    let mut builder = parse_file(in_name, out_name.clone(), config)?;

    builder.write(&out_name);

    Ok(())
}

pub fn run(in_name: String, entry: String, config: Config) -> Result<u64, Error> {
    let mut builder = parse_file(in_name, entry.clone(), config)?;

    Ok(builder.run(&entry))
}

pub fn run_str(input: String, entry: String, config: Config) -> Result<u64, Error> {
    info!("Parsing StdIn");

    let mut builder = parse_str(input, entry.clone(), config)?;

    Ok(builder.run(&entry))
}
