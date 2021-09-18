use colored::*;
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

    pub fn make_indent_str(&self, t: ColoredString) -> String {
        format!(
            "{:<3}{}{}",
            "",
            String::from("| ").repeat(self.indent()).bright_black(),
            t
        )
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
        let ty = self
            .hir
            .node_types
            .get(&t.get_hir_id())
            .map_or_else(|| String::from("None"), |t| format!("{:?}", t));

        println!(
            "{}{:<40} {}",
            t.get_hir_id(),
            self.make_indent_str(t.class_name_self().magenta()),
            ty
        );
    }

    pub fn print_primitive<T>(&self, t: T)
    where
        T: Debug + std::fmt::Display,
    {
        println!("{:<9}{}", "", self.make_indent_str(t.to_string().yellow()),);
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

            // fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
            //     let indent_str = String::from("  ").repeat(self.indent());

            //     let types = self.hir.type_envs.get_fn_types(&f.get_hir_id())
            //         .map(|types| {
            //             types
            //                 .into_iter()
            //                 .map(|(sig, env)| {
            //                     sig.to_string()
            //                 })
            //                 .collect::<Vec<_>>()
            //                 .join(", ")
            //         });


            //     println!(
            //         "{:?}{}{:15}{:?}",
            //         f.get_hir_id(),
            //         indent_str,
            //         f.class_name_self(),
            //         types,
            //     );

            //     walk_function_decl(self, f);
            // }


            fn visit_primitive<T>(&mut self, val: T)
            where
                T: Debug + std::fmt::Display,
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
