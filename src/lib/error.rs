use std::fmt;

use super::ast::TypeInfer;
use super::token::{Token, TokenType};

pub enum Error {
    ParseError(ParseError),
    TypeError(TypeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParseError(pe) => pe.print(f),
            Error::TypeError(te) => te.print(f),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParseError(pe) => pe.print(f),
            Error::TypeError(te) => te.print(f),
        }
    }
}

impl Error {
    pub fn new_parse_error(input: Vec<char>, token: Token, msg: String) -> Error {
        Error::ParseError(ParseError::new(input, token, msg))
    }
}

pub struct TypeError {
    pub input: Vec<char>,
    pub msg: String,
    pub token: Token,
}

impl TypeError {
    pub fn new(input: Vec<char>, token: Token, expected: TypeInfer, found: TypeInfer) -> TypeError {
        let msg = format!("Expected {:?} but found {:?}", expected, found);

        TypeError { input, token, msg }
    }

    pub fn print(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lines: Vec<_> = self.input.split(|c| *c == '\n').collect();

        let count: usize = lines.clone()[..self.token.line - 1]
            .iter()
            .map(|v| v.len())
            .sum();

        let count = count + self.token.line;

        let line_start = if count > self.token.start {
            0
        } else {
            self.token.start - count
        };

        let line_ind = format!("file.rock({}:{}) => ", self.token.line, line_start);

        let mut arrow = String::new();

        let mut i = 0;

        while i <= line_start {
            arrow.push(' ');

            i += 1;
        }

        arrow.push('^');

        write!(
            f,
            "{}[Error]: {}\n{}\n{}",
            line_ind,
            self.msg,
            lines[self.token.line - 1]
                .iter()
                .cloned()
                .collect::<String>(),
            arrow,
        )
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f)
    }
}

impl fmt::Debug for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f)
    }
}

pub struct ParseError {
    pub input: Vec<char>,
    pub token: Token,
    pub msg: String,
}

impl ParseError {
    pub fn new(input: Vec<char>, token: Token, msg: String) -> ParseError {
        ParseError { input, token, msg }
    }

    pub fn new_empty() -> ParseError {
        ParseError {
            input: vec![],
            token: Token {
                t: TokenType::EOF,
                line: 1,
                start: 0,
                end: 0,
                txt: "".to_string(),
            },
            msg: "".to_string(),
        }
    }

    pub fn print(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lines: Vec<_> = self.input.split(|c| *c == '\n').collect();

        let count: usize = lines.clone()[..self.token.line - 1]
            .iter()
            .map(|v| v.len())
            .sum();

        let count = count + self.token.line;

        let line_start = if count > self.token.start {
            0
        } else {
            self.token.start - count
        };

        let line_ind = format!("file.rock({}:{}) => ", self.token.line, line_start);

        let mut arrow = String::new();

        let mut i = 0;

        while i <= line_start {
            arrow.push(' ');

            i += 1;
        }

        arrow.push('^');

        write!(
            f,
            "{}[Error]: {}\n{}\n{}",
            line_ind,
            self.msg,
            lines[self.token.line - 1]
                .iter()
                .cloned()
                .collect::<String>(),
            arrow,
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f)
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f)
    }
}
