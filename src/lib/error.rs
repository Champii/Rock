use std::fmt;

use crate::parser::Span;

// use super::ast::TypeInfer;
use super::parser::{Token, TokenType};

pub struct Error {
    pub input: Vec<char>,
    pub token: Token,
    pub msg: String,
}

impl Error {
    // pub fn new_parse_error(input: Vec<char>, token: Token, msg: String) -> Error {
    //         Error { input, token, msg }
    //     }

    //     // pub fn new_type_error(
    //     //     input: Vec<char>,
    //     //     token: Token,
    //     //     expected: TypeInfer,
    //     //     found: TypeInfer,
    //     // ) -> Error {
    //     //     let msg = format!("Expected '{:?}' but found '{:?}'", expected, found);

    //     //     Error { input, token, msg }
    //     // }

    //     pub fn new_undefined_type(input: Vec<char>, t: String) -> Error {
    //         let mut e = Self::new_empty();

    //         e.msg = format!("'{}' type is not defined", t);
    //         e.input = input;

    //         e
    //     }

    //     pub fn new_undefined_error(input: Vec<char>, name: String) -> Error {
    //         let mut te = Self::new_empty();

    //         te.input = input;
    //         te.msg = format!("'{}' is undefined", name);

    //         te
    //     }

    //     pub fn new_not_indexable_error(name: String) -> Error {
    //         let mut te = Self::new_empty();

    //         te.msg = format!("'{}' is not indexable", name);

    //         te
    //     }

    pub fn new_empty() -> Error {
        Error {
            input: vec![],
            token: Token {
                t: TokenType::EOF,
                span: Span::new(0, 0, 0),
                txt: "".to_string(),
            },
            msg: "".to_string(),
        }
    }

    pub fn print(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //         let lines: Vec<_> = self.input.split(|c| *c == '\n').collect();

        //         let count: usize = lines.clone()[..self.token.line - 1]
        //             .iter()
        //             .map(|v| v.len())
        //             .sum();

        //         let count = count + self.token.line;

        //         let line_start = if count > self.token.start {
        //             0
        //         } else {
        //             self.token.start - count
        //         };

        //         let line_ind = format!("file.rock({}:{}) => ", self.token.line, line_start);

        //         let mut arrow = String::new();

        //         let mut i = 0;

        //         while i <= line_start {
        //             arrow.push(' ');

        //             i += 1;
        //         }

        //         arrow.push('^');

        //         write!(
        //             f,
        //             "{}[Error]: {}\n{}\n{}",
        //             line_ind,
        //             self.msg,
        //             lines[self.token.line - 1]
        //                 .iter()
        //                 .cloned()
        //                 .collect::<String>(),
        //             arrow,
        //         )
        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f)
    }
}
