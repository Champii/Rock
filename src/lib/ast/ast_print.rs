use crate::token::{Token, TokenId};

pub trait AstPrint {
    fn print(&self, _ctx: &mut AstPrintContext) {}
}

pub struct AstPrintContext {
    indent: usize,
    tokens: Vec<Token>,
    _input: Vec<char>,
}

impl AstPrintContext {
    pub fn new(tokens: Vec<Token>, input: Vec<char>) -> Self {
        Self {
            indent: 0,
            tokens,
            _input: input,
        }
    }

    pub fn increment(&mut self) {
        self.indent += 1;
    }

    pub fn decrement(&mut self) {
        self.indent -= 1;
    }

    pub fn get_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.get(token_id).cloned()
    }

    pub fn indent(&self) -> usize {
        self.indent
    }
}

#[macro_use]
macro_rules! derive_print {
    ($id:tt, $trait:tt, $method:ident, $ctx:tt, [ $($field:ident),* ]) => {
        impl $trait for crate::ast::$id {
            fn $method(&self, ctx: &mut $ctx) {
                let indent_str = String::from("  ").repeat(ctx.indent());

                println!("{}{:30}", indent_str, stringify!($id));

                ctx.increment();

                $(
                    self.$field.$method(ctx);
                )*

                ctx.decrement();
            }
        }
    };
}

predef_trait_visitor!(AstPrint, print, AstPrintContext, derive_print);
