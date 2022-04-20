use std::fmt::Debug;

use paste::paste;

use crate::{
    ast::{tree::*, visit::*},
    helpers::*,
    ty::*,
};

pub struct AstPrintContext {
    indent: usize,
}

impl AstPrintContext {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn increment(&mut self) {
        self.indent += 1;
    }

    pub fn decrement(&mut self) {
        self.indent -= 1;
    }

    pub fn indent(&self) -> usize {
        self.indent
    }

    pub fn print<T: ClassName>(&self, t: T) {
        let indent_str = String::from("  ").repeat(self.indent());

        println!("{}{:30}", indent_str, t.class_name_self());
    }

    pub fn print_primitive<T>(&self, t: T)
    where
        T: Debug,
    {
        let indent_str = String::from("  ").repeat(self.indent());

        println!("{}{:?}", indent_str, t);
    }
}

macro_rules! impl_visitor_trait {
    ($(
        $name:ty
    )+) => {
        impl<'ast> Visitor<'ast> for AstPrintContext {
            fn visit_name(&mut self, name: &str) {
                self.print_primitive(name);
            }

            fn visit_type(&mut self, t: &Type) {
                self.print_primitive(t);
            }

            fn visit_primitive<T>(&mut self, val: T)
            where
                T: Debug,
            {
                self.print_primitive(val);
            }

            paste! {
                $(
                    fn [<visit_ $name:snake>](&mut self, node: &'ast $name) {

                        self.print(node);

                        self.increment();

                        [<walk_ $name:snake>](self, node);

                        self.decrement();
                    }
                )+
            }
        }
    };
}

impl_visitor_trait!(
    Root
    TopLevel
    StructDecl
    Use
    Trait
    Assign
    Impl
    FunctionDecl
    Identifier
    Body
    Statement
    For
    While
    ForIn
    Expression
    If
    Else
    UnaryExpr
    StructCtor
    Operator
    PrimaryExpr
    SecondaryExpr
    Operand
    Argument
    Literal
    Array
    NativeOperator
    FuncType
);
