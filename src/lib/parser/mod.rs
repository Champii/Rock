use regex::Regex;

mod lexer;
mod parser;
mod token;

pub use lexer::*;
pub use parser::*;
pub use token::*;

use crate::Error;

pub fn preprocess(input: String) -> String {
    // Add a '.' after a '@' if it is followed by some word
    // This is a dirty trick to ditch having to modiffy the parser for that sugar
    let re = Regex::new(r"@(\w)").unwrap();
    let out = re.replace_all(&input, "@.$1");

    out.to_string()
}

pub fn parse(input: String) -> Result<(crate::ast::Root, Vec<Token>), Error> {
    let preprocessed = preprocess(input.clone());

    let input: Vec<char> = preprocessed.chars().collect();

    let tokens = Lexer::new(input.clone()).collect();

    Ok((Parser::new(tokens.clone(), input.clone()).run()?, tokens))
}
