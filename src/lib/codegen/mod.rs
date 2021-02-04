use std::convert::TryInto;

use inkwell::module::Module;
use inkwell::types::BasicType;
use inkwell::{builder::Builder, values::BasicValueEnum};
use inkwell::{context::Context, types::BasicTypeEnum};

use crate::{
    ast::{PrimitiveType, Type},
    hir::*,
    scopes::Scopes,
};

pub struct CodegenContext<'a> {
    pub context: &'a Context,
    pub hir: &'a Root,
    pub module: Module<'a>,
    pub scopes: Scopes<HirId, BasicValueEnum<'a>>,
}

impl<'a> CodegenContext<'a> {
    pub fn new(context: &'a Context, hir: &'a Root) -> Self {
        let module = context.create_module("mod");
        // let builder = context.create_builder();

        Self {
            context,
            module,
            hir,
            scopes: Scopes::new(),
            // builder,
        }
    }

    pub fn lower_type(&mut self, t: &Type) -> BasicTypeEnum<'a> {
        match t {
            Type::Primitive(p) => match p {
                PrimitiveType::Int64 => self.context.i64_type().into(),
                _ => self.context.i64_type().into(),
            },
            _ => self.context.i64_type().into(),
        }
    }

    pub fn lower_hir(&mut self, root: &'a Root, builder: &'a Builder) {
        for (_, item) in &root.top_levels {
            match &item.kind {
                TopLevelKind::Function(f) => self.lower_function_decl(&f, builder),
            }
        }

        for (_, body) in &root.bodies {
            self.lower_body(&body, builder);
        }
    }

    pub fn lower_function_decl(&mut self, f: &'a FunctionDecl, _builder: &'a Builder) {
        let t = self.hir.get_type(f.hir_id.clone()).unwrap();

        if let Type::FuncType(f_type) = t {
            let ret_t = self.hir.types.get(&f_type.ret).unwrap();

            let ret = self.lower_type(ret_t);

            let args = f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(arg))
                .collect::<Vec<_>>();

            let fn_type = ret.fn_type(args.as_slice(), false);

            self.module.add_function(&f.name.name, fn_type, None);
        }
    }

    pub fn lower_argument_decl(&mut self, arg: &'a ArgumentDecl) -> BasicTypeEnum<'a> {
        let t = self.hir.get_type(arg.name.hir_id.clone()).unwrap();

        self.lower_type(&t)
    }

    pub fn lower_body(&mut self, body: &'a Body, builder: &'a Builder) {
        match self.module.get_function(&body.name.name) {
            Some(f) => {
                let hir_top_reso = self.hir.resolutions.get(body.name.hir_id.clone()).unwrap();
                let hir_top = self.hir.get_top_level(hir_top_reso).unwrap();

                match &hir_top.kind {
                    TopLevelKind::Function(hir_f) => {
                        let mut i = 0;
                        for arg in &hir_f.arguments {
                            self.scopes
                                .add(arg.name.hir_id.clone(), f.get_nth_param(i).unwrap());

                            i += 1;
                        }
                    }
                }

                let basic_block = self.context.append_basic_block(f, "entry");

                // let builder = self.context.create_builder();
                builder.position_at_end(basic_block);

                let ret = self.lower_stmt(&body.stmt, builder);
                builder.build_return(Some(&ret));
            }
            None => (),
        }
    }

    pub fn lower_stmt(&mut self, stmt: &'a Statement, builder: &'a Builder) -> BasicValueEnum<'a> {
        match &*stmt.kind {
            StatementKind::Expression(e) => self.lower_expression(&e, builder),
            // _ => unimplemented!(),
        }
    }

    pub fn lower_expression(
        &mut self,
        expr: &'a Expression,
        builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        match &*expr.kind {
            ExpressionKind::Lit(l) => self.lower_literal(&l, builder),
            ExpressionKind::Identifier(id) => self.lower_identifier(&id, builder),
            ExpressionKind::FunctionCall(callee, args) => {
                self.lower_function_call(callee, args, builder)
            }
        }
    }

    pub fn lower_function_call(
        &mut self,
        callee: &Expression,
        args: &'a Vec<Expression>,
        builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        let terminal_hir_id = callee.get_terminal_hir_id();

        let f_id = self.hir.resolutions.get(terminal_hir_id).unwrap();
        if let Some(top) = self.hir.get_top_level(f_id) {
            match &top.kind {
                TopLevelKind::Function(f) => {
                    let f_value = self.module.get_function(&f.name.to_string()).unwrap();

                    let arguments = args
                        .iter()
                        .map(|arg: &'a _| self.lower_expression(arg, builder))
                        .collect::<Vec<_>>();

                    builder
                        .build_call(f_value, arguments.as_slice(), "call")
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                }
            }
        } else {
            panic!("Fn not found")
        }
    }

    pub fn lower_literal(&mut self, lit: &Literal, _builder: &'a Builder) -> BasicValueEnum<'a> {
        match &lit.kind {
            LiteralKind::Number(n) => {
                let i64_type = self.context.i64_type();

                i64_type.const_int((*n).try_into().unwrap(), false).into()
            }
            _ => unimplemented!(),
        }
    }

    pub fn lower_identifier(
        &mut self,
        id: &Identifier,
        _builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        let reso = self.hir.resolutions.get(id.hir_id.clone()).unwrap();

        self.scopes.get(reso).unwrap()
    }
}

pub fn generate(hir: &Root) {
    let context = Context::create();
    let builder = context.create_builder();

    let mut codegen_ctx = CodegenContext::new(&context, &hir);
    codegen_ctx.lower_hir(hir, &builder);

    codegen_ctx.module.verify().unwrap();
    codegen_ctx
        .module
        .write_bitcode_to_path(&std::path::Path::new("./out.ir"));
}
