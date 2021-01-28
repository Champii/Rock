use crate::ast::helper::*;
use crate::ast::*;
use crate::token::{Token, TokenId};
use crate::visit::*;

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

    pub fn print<T: ClassName>(&self, t: T) {
        let indent_str = String::from("  ").repeat(self.indent());

        println!("{}{:30}", indent_str, t.class_name_self());
    }
}

macro_rules! impl_visitor_trait {
    ($(
        $name:ident, $method:ident
    )*) => {
        impl<'ast> Visitor<'ast> for AstPrintContext {
            fn visit_name(&mut self, name: String) {
                self.print(name);
            }

            fn visit_primitive<T>(&mut self, val: T)
            where
                T: ClassName,
            {
                self.print(val);
            }

            $(
                concat_idents!(fn_name = visit_, $method {
                    fn fn_name(&mut self, $method: &'ast $name) {
                        self.print($method);

                        self.increment();

                        concat_idents!(fn2_name = walk_, $method {
                            fn2_name(self, $method);
                        });

                        self.decrement();
                    }
                });
            )*
        }
    };
}

impl_visitor_trait!(
    Root, root
    Mod, r#mod
    TopLevel, top_level
    FunctionDecl, function_decl
    Identifier, identifier
    ArgumentDecl, argument_decl
    Body, body
    Statement, statement
    Expression, expression
    If, r#if
    UnaryExpr, unary
    Operator, operator
    PrimaryExpr, primary
    SecondaryExpr, secondary
    Operand, operand
    Argument, argument
    Literal, literal
);