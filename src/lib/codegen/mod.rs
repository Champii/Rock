use std::convert::TryInto;

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::BasicValue;
use inkwell::{builder::Builder, values::BasicValueEnum};

use crate::hir::*;

pub struct CodegenContext<'a> {
    pub context: &'a Context,
    pub module: Module<'a>,
}

impl<'a> CodegenContext<'a> {
    pub fn new(context: &'a Context) -> Self {
        let module = context.create_module("mod");
        // let builder = context.create_builder();

        Self {
            context,
            module,
            // builder,
        }
    }

    pub fn lower_hir(&mut self, root: &Root) {
        for (_, item) in &root.top_levels {
            match &item.kind {
                TopLevelKind::Function(f) => self.lower_function_decl(&f),
            }
        }

        for (_, body) in &root.bodies {
            self.lower_body(&body);
        }

        self.module.print_to_stderr();
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) {
        let void_type = self.context.void_type();
        let fn_type = void_type.fn_type(&[], false);

        self.module.add_function(&f.name.name, fn_type, None);
    }

    pub fn lower_body(&mut self, body: &Body) {
        match self.module.get_function(&body.name.name) {
            Some(f) => {
                let basic_block = self.context.append_basic_block(f, "entry");

                let builder = self.context.create_builder();
                builder.position_at_end(basic_block);

                let ret = self.lower_stmt(&body.stmt);
                builder.build_return(Some(&ret));
            }
            None => (),
        }
    }

    pub fn lower_stmt(&mut self, stmt: &Statement) -> BasicValueEnum {
        match &*stmt.kind {
            StatementKind::Expression(e) => self.lower_expression(&e),
            // _ => unimplemented!(),
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> BasicValueEnum {
        match &*expr.kind {
            ExpressionKind::Lit(l) => self.lower_literal(&l),
            // ExpressionKind::FunctionCall(callee, args) => self.lower_function_call(&callee, &args),
            _ => unimplemented!(),
        }
    }

    // pub fn lower_function_call(
    //     &mut self,
    //     callee: &Expression,
    //     args: &Vec<Expression>,
    // ) -> BasicValueEnum {
    // }

    pub fn lower_literal(&mut self, lit: &Literal) -> BasicValueEnum {
        match &lit.kind {
            LiteralKind::Number(n) => {
                let i64_type = self.context.i64_type();

                i64_type.const_int((*n).try_into().unwrap(), false).into()
            }
            _ => unimplemented!(),
        }
    }
}
