use std::fmt;

use super::token::Token;

pub struct Error {
    pub input: Vec<char>,
    pub token: Token,
    pub msg: String,
}

impl Error {
    pub fn new(input: Vec<char>, token: Token, msg: String) -> Error {
        Error { input, token, msg }
    }

    pub fn print(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lines: Vec<_> = self.input.split(|c| *c == '\n').collect();

        let count: usize = lines.clone()[..lines.len() - 1]
            .iter()
            .map(|v| v.len())
            .sum();

        let count = count + lines.len();

        let line_start = self.token.start - count;
        // let line_end = self.token.end - count;

        let line_ind = format!("file.lang({}:{}) => ", self.token.line, line_start);

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
