use paste::paste;
use std::fmt::Debug;

use crate::ast::Type;
use crate::helpers::*;
use crate::hir::visit::*;
use crate::hir::HasHirId;
use crate::hir::*;

pub struct HirPrinter<'a> {
    hir: &'a Root,
    indent: usize,
}

impl<'a> HirPrinter<'a> {
    pub fn new(hir: &'a Root) -> Self {
        Self { hir, indent: 0 }
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

    pub fn print<T: ClassName + HasHirId>(&self, t: T) {
        let indent_str = String::from("  ").repeat(self.indent());

        let t_id = self.hir.node_types.get(&t.get_hir_id());
        let ty = match t_id {
            Some(t_id) => self.hir.types.get(&t_id),
            None => None,
        };

        println!(
            "{:?}{}{:15}{:?}:{:?}",
            t.get_hir_id(),
            indent_str,
            t.class_name_self(),
            t_id,
            ty,
        );
    }

    pub fn print_primitive<T>(&self, t: T)
    where
        T: Debug,
    {
        let indent_str = String::from("  ").repeat(self.indent());

        println!("{:9}{}{:15?}", "", indent_str, t);
    }
}

macro_rules! impl_visitor_trait2 {
    ($(
        $name:ident
    )*) => {
        impl<'a> Visitor<'a> for HirPrinter<'a> {
            fn visit_name(&mut self, name: String) {
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
                    fn [<visit_ $name:snake>](&mut self, item: &'a $name) {
                        self.print(item.clone());

                        self.increment();

                        [<walk_ $name:snake>](self, item);

                        self.decrement();
                    }
                )*
            }
        }
    };
}

impl_visitor_trait2!(
    TopLevel
    Assign
    Prototype
    FunctionDecl
    ArgumentDecl
    Identifier
    FnBody
    Body
    Statement
    If
    Else
    FunctionCall
    Literal
    NativeOperator
);

pub fn print(root: &Root) {
    HirPrinter::new(root).visit_root(root)
}
