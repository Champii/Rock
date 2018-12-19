extern crate llvm_sys as llvm;

use std::fs;
use std::process::Command;

mod ast;
mod codegen;
mod error;
mod lexer;
mod parser;
mod scope;
mod token;

use self::lexer::Lexer;
use self::parser::Parser;
use self::token::{Token, TokenType};

// impl Token {
//     pub fn new()
// }

fn main() {
    let file = fs::read_to_string("./test.lang").expect("Woot");
    // let mut lexer = Lexer::new("main -> p(\"lol\", 3, \"ja\")".chars().collect());
    let mut lexer = Lexer::new(file.chars().collect());

    // println!("{:#?}", lexer.all());
    // println!("{:?}", lexer.next());
    // println!("{:?}", lexer.next());
    // println!("{:?}", lexer.next());
    // println!("{:?}", lexer.next());
    // println!("{:?}", lexer.next());
    // println!("{:?}", lexer.next());
    let mut parser = Parser::new(lexer);

    let ast = match parser.run() {
        Ok(ast) => ast,
        Err(e) => {
            println!("{}", e);

            return;
        }
    };

    // println!("{:#?}", ast);

    ast.build("./test.o\0");

    Command::new("clang")
        .arg("test.o")
        .output()
        .expect("failed to execute process");
}
