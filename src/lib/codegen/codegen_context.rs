use either::Either;
use std::convert::TryInto;

use inkwell::{
    builder::Builder,
    values::{BasicValueEnum, FunctionValue, PointerValue},
};
use inkwell::{context::Context, types::BasicTypeEnum};
use inkwell::{module::Module, values::BasicValue};
use inkwell::{types::BasicType, AddressSpace};

use crate::{
    ast::{PrimitiveType, Type},
    helpers::scopes::Scopes,
    hir::*,
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

        Self {
            context,
            module,
            hir,
            scopes: Scopes::new(),
        }
    }

    pub fn lower_type(&mut self, t: &Type) -> BasicTypeEnum<'a> {
        match t {
            Type::Primitive(PrimitiveType::Int64) => self.context.i64_type().into(),
            Type::Primitive(PrimitiveType::Bool) => self.context.bool_type().into(),
            Type::FuncType(f) => {
                let f2 = self.module.get_function(&f.name).unwrap();

                f2.get_type().ptr_type(AddressSpace::Generic).into()
            }
            _ => unimplemented!(),
        }
    }

    pub fn lower_hir(&mut self, root: &'a Root, builder: &'a Builder) {
        for item in root.top_levels.values() {
            match &item.kind {
                TopLevelKind::Function(f) => self.lower_function_decl(&f, builder),
            }
        }

        for body in root.bodies.values() {
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

            let fn_value = self.module.add_function(&f.name.name, fn_type, None);

            self.scopes.add(
                f.hir_id.clone(),
                fn_value
                    .as_global_value()
                    .as_pointer_value()
                    .as_basic_value_enum(),
            );
        }
    }

    pub fn lower_argument_decl(&mut self, arg: &'a ArgumentDecl) -> BasicTypeEnum<'a> {
        let t = self.hir.get_type(arg.name.hir_id.clone()).unwrap();

        self.lower_type(&t)
    }

    pub fn lower_body(&mut self, body: &'a Body, builder: &'a Builder) {
        if let Some(f) = self.module.get_function(&body.name.name) {
            let hir_top_reso = self.hir.resolutions.get(body.name.hir_id.clone()).unwrap();
            let hir_top = self.hir.get_top_level(hir_top_reso).unwrap();

            match &hir_top.kind {
                TopLevelKind::Function(hir_f) => {
                    for (i, arg) in hir_f.arguments.iter().enumerate() {
                        self.scopes.add(
                            arg.name.hir_id.clone(),
                            f.get_nth_param(i.try_into().unwrap()).unwrap(),
                        );
                    }
                }
            }

            let basic_block = self.context.append_basic_block(f, "entry");

            builder.position_at_end(basic_block);

            let ret = self.lower_stmt(&body.stmt, builder);

            builder.build_return(Some(&ret));
        }
    }

    pub fn lower_stmt(&mut self, stmt: &'a Statement, builder: &'a Builder) -> BasicValueEnum<'a> {
        match &*stmt.kind {
            StatementKind::Expression(e) => self.lower_expression(&e, builder),
        }
    }

    pub fn lower_expression(
        &mut self,
        expr: &'a Expression,
        builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        match &*expr.kind {
            ExpressionKind::Lit(l) => self.lower_literal(&l, builder),
            ExpressionKind::Identifier(id) => self.lower_identifier_path(&id, builder),
            ExpressionKind::FunctionCall(fc) => self.lower_function_call(fc, builder),
            ExpressionKind::NativeOperation(op, left, right) => {
                self.lower_native_operation(op, left, right, builder)
            }
        }
    }

    pub fn lower_function_call(
        &mut self,
        fc: &'a FunctionCall,
        builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        let terminal_hir_id = fc.op.get_terminal_hir_id();

        let f_id = self.hir.resolutions.get(terminal_hir_id.clone()).unwrap();

        let f_value: Either<FunctionValue, PointerValue> =
            match self.hir.get_top_level(f_id.clone()) {
                Some(top) => match &top.kind {
                    TopLevelKind::Function(f) => {
                        Either::Left(self.module.get_function(&f.name.to_string()).unwrap())
                    }
                },
                None => Either::Right(self.lower_expression(&fc.op, builder).into_pointer_value()),
            };

        let arguments = fc
            .args
            .iter()
            .map(|arg: &'a _| self.lower_expression(arg, builder))
            .collect::<Vec<_>>();

        builder
            .build_call(f_value, arguments.as_slice(), "call")
            .try_as_basic_value()
            .left()
            .unwrap()
    }

    pub fn lower_literal(&mut self, lit: &Literal, _builder: &'a Builder) -> BasicValueEnum<'a> {
        match &lit.kind {
            LiteralKind::Number(n) => {
                let i64_type = self.context.i64_type();

                i64_type.const_int((*n).try_into().unwrap(), false).into()
            }
            LiteralKind::Bool(b) => {
                let bool_type = self.context.bool_type();

                bool_type.const_int((*b).try_into().unwrap(), false).into()
            }
            _ => unimplemented!(),
        }
    }

    pub fn lower_identifier_path(
        &mut self,
        id: &IdentifierPath,
        _builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        self.lower_identifier(id.path.iter().last().unwrap(), _builder)
    }

    pub fn lower_identifier(
        &mut self,
        id: &Identifier,
        _builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        let reso = self.hir.resolutions.get((&id.hir_id).clone()).unwrap();

        self.scopes.get(reso).unwrap()
    }

    pub fn lower_native_operation(
        &mut self,
        op: &NativeOperator,
        left: &Identifier,
        right: &Identifier,
        builder: &'a Builder,
    ) -> BasicValueEnum<'a> {
        let left = self.lower_identifier(left, builder).into_int_value();
        let right = self.lower_identifier(right, builder).into_int_value();

        match op.kind {
            NativeOperatorKind::Add => builder.build_int_add(left, right, ""),
            NativeOperatorKind::Sub => builder.build_int_sub(left, right, ""),
            NativeOperatorKind::Mul => builder.build_int_mul(left, right, ""),
            NativeOperatorKind::Div => builder.build_int_signed_div(left, right, ""),
        }
        .as_basic_value_enum()
    }
}
