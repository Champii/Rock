use std::fmt;

use super::ast::TypeInfer;
use super::token::{Token, TokenType};

// pub enum Error {
//     ParseError(ParseError),
//     TypeError(TypeError),
// }

// impl fmt::Display for Error {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             Error::ParseError(pe) => pe.print(f),
//             Error::TypeError(te) => te.print(f),
//         }
//     }
// }

// impl fmt::Debug for Error {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             Error::ParseError(pe) => pe.print(f),
//             Error::TypeError(te) => te.print(f),
//         }
//     }
// }

// impl Error {
//     pub fn new_parse_error(input: Vec<char>, token: Token, msg: String) -> Error {
//         Error::ParseError(ParseError::new(input, token, msg))
//     }

//     pub fn new_type_error(expected: TypeInfer, found: TypeInfer) -> Error {
//         Error::TypeError(TypeError::new(expected, found))
//     }
// }

// pub struct TypeError {
//     pub input: Vec<char>,
//     pub msg: String,
//     pub token: Token,
// }

// impl TypeError {
//     pub fn new(expected: TypeInfer, found: TypeInfer) -> TypeError {
//         let mut te = Self::new_empty();

//         te.set_msg(expected, found);

//         te
//     }


//     pub fn new_empty() -> TypeError {
//         TypeError {
//             input: vec![],
//             token: Token {
//                 t: TokenType::EOF,
//                 line: 1,
//                 start: 0,
//                 end: 0,
//                 txt: "".to_string(),
//             },
//             msg: "".to_string(),
//         }
//     }

//     pub fn set_msg(&mut self, expected: TypeInfer, found: TypeInfer) {
//         self.msg = format!("Expected '{}' but found '{}'", expected.unwrap(), found.unwrap());
//     }

//     pub fn print(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
//     }
// }

// impl fmt::Display for TypeError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         self.print(f)
//     }
// }

// impl fmt::Debug for TypeError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         self.print(f)
//     }
// }

pub struct Error {
    pub input: Vec<char>,
    pub token: Token,
    pub msg: String,
}

impl Error {
    pub fn new_parse_error(input: Vec<char>, token: Token, msg: String) -> Error {
        Error { input, token, msg }
    }

    pub fn new_type_error(input: Vec<char>, token: Token, expected: TypeInfer, found: TypeInfer) -> Error {
        let msg = format!("Expected '{}' but found '{}'", expected.unwrap(), found.unwrap());

        Error {input, token, msg}
    }

    pub fn new_undefined_error(name: String) -> Error {
        let mut te = Self::new_empty();

        te.msg = format!("'{}' is undefined", name);

        te
    }

    pub fn new_not_indexable_error(name: String) -> Error {
        let mut te = Self::new_empty();

        te.msg = format!("'{}' is not indexable", name);

        te
    }

    pub fn new_empty() -> Error {
        Error {
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
