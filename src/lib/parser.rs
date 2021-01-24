use crate::token::TokenId;

use super::ast::*;
use super::error::Error;
use super::Token;

#[macro_export]
macro_rules! expect {
    ($tok:expr, $self:expr) => {
        if $tok != $self.cur_tok().t {
            // panic!("Expected {:?} but found {:?}", $expr, $tok);
            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok();

            $self.consume();

            cur_tok
        }
    };
}

#[macro_export]
macro_rules! expect_or_restore {
    ($tok:expr, $self:expr) => {
        if $self.cur_tok().t != $tok {
            $self.restore();

            error_expect!($tok, $self);
        } else {
            let cur_tok = $self.cur_tok();

            $self.consume();

            cur_tok
        }
    };
}

#[macro_export]
macro_rules! error_expect {
    ($expected:expr, $self:expr) => {
        crate::parser::macros::error!(
            format!("Expected {:?} but got {:?}", $expected, $self.cur_tok().t),
            $self
        );
    };
}

#[macro_export]
macro_rules! error {
    ($msg:expr, $self:expr) => {
        return Err(Error::new_parse_error(
            $self.input.clone(),
            $self.cur_tok(),
            $msg,
        ));
    };
}

#[macro_export]
macro_rules! try_or_restore {
    ($expr:expr, $self:expr) => {
        match $expr {
            Ok(t) => t,
            Err(e) => {
                $self.restore();

                return Err(e);
            }
        }
    };
}

#[macro_export]
macro_rules! try_or_restore_expect {
    ($expr:expr, $expected:expr, $self:expr) => {
        try_or_restore_and!($expr, error_expect!($expected, $self), $self);
    };
}

#[macro_export]
macro_rules! try_or_restore_and {
    ($expr:expr, $and:expr, $self:expr) => {
        match $expr {
            Ok(t) => t,
            Err(_e) => {
                $self.restore();

                #[allow(unreachable_code)]
                return $and;
            }
        }
    };
}

pub mod macros {
    pub use crate::error;
    pub use crate::error_expect;
    pub use crate::expect;
    pub use crate::expect_or_restore;
    pub use crate::try_or_restore;
    pub use crate::try_or_restore_and;
    pub use crate::try_or_restore_expect;
}

// TODO: Create getters and setters instead of exposing publicly
pub struct Parser {
    pub input: Vec<char>,
    pub tokens: Vec<Token>,
    pub cur_tok_id: TokenId,
    save: Vec<TokenId>,
    pub block_indent: u8,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, input: Vec<char>) -> Parser {
        Parser {
            input,
            tokens,
            save: vec![0],
            cur_tok_id: 0,
            block_indent: 0,
        }
    }

    pub fn run(&mut self) -> Result<SourceFile, Error> {
        SourceFile::parse(self)
    }

    pub fn cur_tok(&self) -> Token {
        match self.tokens.get(self.cur_tok_id as usize) {
            Some(a) => a.clone(),
            _ => unreachable!(),
        }
    }

    pub fn consume(&mut self) {
        self.cur_tok_id += 1;
    }

    pub fn save(&mut self) {
        self.save.push(self.cur_tok_id);
    }

    pub fn save_pop(&mut self) {
        self.save.pop().unwrap();
    }

    pub fn restore(&mut self) {
        let save = self.save.pop().unwrap();

        self.cur_tok_id = save;
    }

    pub fn seek(&self, distance: usize) -> Token {
        match self.tokens.get(self.cur_tok_id as usize + distance) {
            Some(a) => a.clone(),
            _ => unreachable!(),
        }
    }
}
