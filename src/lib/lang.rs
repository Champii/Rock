extern crate llvm_sys as llvm;

use std::fs;

mod ast;
mod codegen;
mod error;
mod lexer;
mod parser;
mod scope;
mod tests;
mod token;

use self::codegen::Builder;
use self::error::Error;
use self::lexer::Lexer;
use self::parser::Parser;
use self::token::{Token, TokenType};

pub fn parse_file(in_name: String) -> Result<Builder, Error> {
    let file = fs::read_to_string(in_name).expect("Woot");

    parse_str(file)
}

pub fn parse_str(input: String) -> Result<Builder, Error> {
    let mut lexer = Lexer::new(input.chars().collect());

    // println!("LEX {:#?}", lexer.all());

    let mut parser = Parser::new(lexer);

    let ast = parser.run()?;

    // println!("AST {:#?}", ast);

    let mut builder = Builder::new("STDIN\0", ast);

    builder.build();

    Ok(builder)
}

pub fn file_to_file(in_name: String, out_name: String) -> Result<(), Error> {
    let mut builder = parse_file(in_name)?;

    builder.write(&out_name);

    Ok(())
}

pub fn run(in_name: String, entry: String) -> Result<u64, Error> {
    let mut builder = parse_file(in_name)?;

    Ok(builder.run(&entry))
}

pub fn run_str(input: String, entry: String) -> Result<u64, Error> {
    let mut builder = parse_str(input)?;

    Ok(builder.run(&entry))
}
