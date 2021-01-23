#![feature(const_fn, associated_type_bounds)]

extern crate llvm_sys as llvm;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;

use infer::*;
use regex::Regex;
use std::fs;

#[macro_use]
pub mod ast;
#[macro_use]
pub mod infer;
#[macro_use]
pub use crate::infer::*;

mod codegen;
pub mod config;
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
// use self::generator::Generator;
use self::lexer::Lexer;
use self::parser::Parser;
use self::token::{Token, TokenType};
// use self::type_checker::TypeChecker;
use self::ast::ast_print::*;
// use self::ast::helper::*;

// pub trait Test {
//     fn test(&self, ctx: &mut InferBuilder)
//     where
//         Self: std::fmt::Debug + HasName + Annotate,
//     {
//         println!("1{:?}", self);
//         self.annotate(ctx);
//     }
// }

// impl<T: Annotate + HasName> Test for T {}
// impl<T: Annotate> Test for T {
//     fn test(&self, ctx: &mut InferBuilder)
//     where
//         Self: std::fmt::Debug + HasName + Annotate,
//     {
//         println!("2{:?}", self);
//         self.annotate(ctx);
//     }
// }

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
    let mut infer_builder = &mut InferBuilder::new(InferState::new());

    ast.annotate(infer_builder);

    ast.constrain(infer_builder);

    infer_builder.solve();

    println!("INFER {:#?}", infer_builder);

    // ast.test();

    // info!("   -> TypeCheck {}", output_name);
    // let mut tc = TypeChecker::new(ast);

    // tc.ctx.input = input.clone();

    // tc.infer()?;

    // info!("   -> Generate {}", output_name);
    // let ast = Generator::new(tc.ast, tc.ctx).generate()?;

    if config.show_ast {
        // println!("AST {:#?}", ast);
        ast.print(&mut AstPrintContext::new(tokens.clone(), input.clone()));
    }

    // info!("   -> Codegen {}", output_name);
    let mut builder = Builder::new(&output_name, ast, config);

    // builder.build();

    // Ok(builder)
    Ok(builder)
}

pub fn file_to_file(in_name: String, out_name: String, config: Config) -> Result<(), Error> {
    let mut builder = parse_file(in_name, out_name.clone(), config)?;

    // builder.write(&out_name);

    Ok(())
}

pub fn run(in_name: String, entry: String, config: Config) -> Result<u64, Error> {
    let mut builder = parse_file(in_name, entry.clone(), config)?;

    // Ok(builder.run(&entry))
    Ok(0)
}

pub fn run_str(input: String, entry: String, config: Config) -> Result<u64, Error> {
    info!("Parsing StdIn");

    let mut builder = parse_str(input, entry.clone(), config)?;

    Ok(0)
    // Ok(builder.run(&entry))
}
