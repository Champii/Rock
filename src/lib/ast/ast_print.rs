use crate::token::{Token, TokenId};

pub trait AstPrint {
    fn print(&self, ctx: &mut AstPrintContext);
}

impl<T: AstPrint> AstPrint for Vec<T> {
    fn print(&self, ctx: &mut AstPrintContext) {
        for x in self {
            x.print(ctx);
        }
    }
}

pub struct AstPrintContext {
    indent: usize,
    tokens: Vec<Token>,
    input: Vec<char>,
}

impl AstPrintContext {
    pub fn new(tokens: Vec<Token>, input: Vec<char>) -> Self {
        Self {
            indent: 0,
            tokens,
            input,
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
    ($id:tt, [ $($field:ident),* ]) => {
        impl AstPrint for $id {
            fn print(&self, ctx: &mut AstPrintContext) {
                let indent_str = String::from("  ").repeat(ctx.indent());

                println!("{}{} {:?}", indent_str, stringify!($id), ctx.get_token(self.token).unwrap().t);

                ctx.increment();

                $(
                    self.$field.print(ctx);
                )*

                ctx.decrement();
            }
        }
    };
}
