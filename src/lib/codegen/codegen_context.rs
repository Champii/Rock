use either::Either;

use std::convert::TryInto;

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    values::{AnyValue, AnyValueEnum, BasicValueEnum, FunctionValue, PointerValue},
    FloatPredicate, IntPredicate,
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
    pub cur_func: Option<FunctionValue<'a>>,
}

impl<'a> CodegenContext<'a> {
    pub fn new(context: &'a Context, hir: &'a Root) -> Self {
        let module = context.create_module("mod");

        Self {
            context,
            module,
            hir,
            scopes: Scopes::new(),
            cur_func: None,
        }
    }

    pub fn lower_type(&mut self, t: &Type, builder: &'a Builder) -> BasicTypeEnum<'a> {
        match t {
            Type::Primitive(PrimitiveType::Int8) => self.context.i8_type().into(),
            Type::Primitive(PrimitiveType::Int64) => self.context.i64_type().into(),
            Type::Primitive(PrimitiveType::Float64) => self.context.f64_type().into(),
            Type::Primitive(PrimitiveType::Bool) => self.context.bool_type().into(),
            Type::Primitive(PrimitiveType::String(_)) => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .into(),
            Type::FuncType(f) => {
                let f2 = match self.module.get_function(&f.name) {
                    Some(f2) => f2,
                    None => {
                        let f = self.hir.get_function_by_name(&f.name).unwrap();

                        self.lower_function_decl(&f, builder);

                        self.module.get_function(&f.name).unwrap()
                    }
                };

                f2.get_type().ptr_type(AddressSpace::Generic).into()
            }
            _ => unimplemented!(),
        }
    }

    pub fn lower_hir(&mut self, root: &'a Root, builder: &'a Builder) {
        for (_, map) in &root.trait_methods {
            for (_, func) in map {
                self.lower_function_decl(&func, builder);
            }
        }
        for item in root.top_levels.values() {
            match &item.kind {
                TopLevelKind::Prototype(p) => self.lower_prototype(&p, builder),
                TopLevelKind::Function(f) => self.lower_function_decl(&f, builder),
            }
        }

        for body in root.bodies.values() {
            self.lower_fn_body(&body, builder);
        }
    }

    pub fn lower_prototype(&mut self, p: &'a Prototype, builder: &'a Builder) {
        let t = self.hir.get_type(p.hir_id.clone()).unwrap();

        if let Type::FuncType(f_type) = t {
            let ret_t = self.hir.types.get(&f_type.ret).unwrap();

            let args = p
                .signature
                .args
                .iter()
                .map(|arg| self.lower_type(arg, builder))
                .collect::<Vec<_>>();

            let fn_type = if let Type::Primitive(PrimitiveType::Void) = ret_t {
                self.context.void_type().fn_type(args.as_slice(), false)
            } else {
                self.lower_type(ret_t, builder)
                    .fn_type(args.as_slice(), false)
            };

            let fn_value = self.module.add_function(&p.name.name, fn_type, None);

            self.scopes.add(
                p.hir_id.clone(),
                fn_value
                    .as_global_value()
                    .as_pointer_value()
                    .as_basic_value_enum(),
            );
        }
    }
    pub fn lower_function_decl(&mut self, f: &FunctionDecl, builder: &'a Builder) {
        let mangled_name = f.get_name();
        if self.module.get_function(&mangled_name).is_some() {
            return;
        }

        let t = self.hir.get_type(f.hir_id.clone()).unwrap();

        if let Type::FuncType(f_type) = t {
            let ret_t = self.hir.types.get(&f_type.ret).unwrap();

            let args = f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(arg, builder))
                .collect::<Vec<_>>();

            let fn_type = if let Type::Primitive(PrimitiveType::Void) = ret_t {
                self.context.void_type().fn_type(args.as_slice(), false)
            } else {
                self.lower_type(ret_t, builder)
                    .fn_type(args.as_slice(), false)
            };

            let fn_value = self.module.add_function(&mangled_name, fn_type, None);

            self.scopes.add(
                f.name.hir_id.clone(),
                fn_value
                    .as_global_value()
                    .as_pointer_value()
                    .as_basic_value_enum(),
            );
            // FIXME: Hack
            self.scopes.add(
                f.hir_id.clone(),
                fn_value
                    .as_global_value()
                    .as_pointer_value()
                    .as_basic_value_enum(),
            );
        }
    }

    pub fn lower_argument_decl(
        &mut self,
        arg: &ArgumentDecl,
        builder: &'a Builder,
    ) -> BasicTypeEnum<'a> {
        let t = self.hir.get_type(arg.name.hir_id.clone()).unwrap();

        self.lower_type(&t, builder)
    }

    pub fn lower_fn_body(&mut self, fn_body: &'a FnBody, builder: &'a Builder) {
        if let Some(f) = self.module.get_function(&fn_body.get_name()) {
            self.cur_func = Some(f);

            let hir_top_reso = self
                .hir
                .resolutions
                .get(fn_body.name.hir_id.clone())
                .unwrap();

            let hir_top = if let Some(hir_top) = self.hir.get_top_level(hir_top_reso.clone()) {
                match &hir_top.kind {
                    TopLevelKind::Function(hir_f) => hir_f,
                    TopLevelKind::Prototype(_) => panic!(),
                }
            } else {
                self.hir
                    .trait_methods
                    .get(&fn_body.name.name)
                    .unwrap()
                    .iter()
                    .find(|(_applied_to, func_decl)| func_decl.name.hir_id == fn_body.name.hir_id)
                    .map(|tuple| tuple.1)
                    .unwrap()
            };

            for (i, arg) in hir_top.arguments.iter().enumerate() {
                self.scopes.add(
                    arg.name.hir_id.clone(),
                    f.get_nth_param(i.try_into().unwrap()).unwrap(),
                );
            }

            self.lower_body(&fn_body.body, "entry", builder);
        } else {
            panic!("Cannot find function {}", fn_body.name.name);
        }
    }

    pub fn lower_body(
        &mut self,
        body: &'a Body,
        name: &str,
        builder: &'a Builder,
    ) -> (AnyValueEnum<'a>, BasicBlock<'a>) {
        let basic_block = self
            .context
            .append_basic_block(self.cur_func.clone().unwrap(), name);

        builder.position_at_end(basic_block);

        let stmt = body
            .stmts
            .iter()
            .map(|stmt| self.lower_stmt(&stmt, builder))
            .last()
            .unwrap();
        (stmt, basic_block)
    }

    pub fn lower_stmt(&mut self, stmt: &'a Statement, builder: &'a Builder) -> AnyValueEnum<'a> {
        match &*stmt.kind {
            StatementKind::Expression(e) => self.lower_expression(&e, builder).as_any_value_enum(),
            StatementKind::If(e) => self.lower_if(&e, builder).0,
        }
    }

    pub fn lower_if(
        &mut self,
        r#if: &'a If,
        builder: &'a Builder,
    ) -> (AnyValueEnum<'a>, BasicBlock<'a>) {
        let block = builder.get_insert_block().unwrap();

        builder.position_at_end(block);

        let predicat = self.lower_expression(&r#if.predicat, builder);

        let (_, then_block) = self.lower_body(&r#if.body, "then", builder);

        let else_block = if let Some(e) = &r#if.else_ {
            let else_block = self.lower_else(e, builder);

            else_block
        } else {
            //new empty block
            let f = self.module.get_last_function().unwrap();
            let else_block = self.context.append_basic_block(f, "else");

            else_block
        };

        // FIXME: Need a last block if the 'if' is not the last statement in the fn body
        // let rest_block = self
        //     .context
        //     .append_basic_block(self.module.get_last_function().unwrap(), "rest");

        // builder.build_unconditional_branch(rest_block);

        // builder.position_at_end(then_block);

        // builder.build_unconditional_branch(rest_block);

        builder.position_at_end(block);

        let if_value =
            builder.build_conditional_branch(predicat.into_int_value(), then_block, else_block);

        builder.position_at_end(else_block);

        (if_value.as_any_value_enum(), block)
    }

    pub fn lower_else(&mut self, r#else: &'a Else, builder: &'a Builder) -> BasicBlock<'a> {
        match &r#else {
            Else::If(i) => {
                let block = self
                    .context
                    .append_basic_block(self.cur_func.clone().unwrap(), "if");

                builder.position_at_end(block);

                self.lower_if(i, builder).1
            }
            Else::Body(b) => self.lower_body(b, "else", builder).1,
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
            ExpressionKind::Return(expr) => {
                let val = self.lower_expression(expr, builder);

                builder.build_return(Some(&val.as_basic_value_enum()));

                val
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
                    TopLevelKind::Prototype(p) => {
                        Either::Left(self.module.get_function(&p.name.to_string()).unwrap())
                    }
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

    pub fn lower_literal(&mut self, lit: &Literal, builder: &'a Builder) -> BasicValueEnum<'a> {
        match &lit.kind {
            LiteralKind::Number(n) => {
                let i64_type = self.context.i64_type();

                i64_type.const_int((*n).try_into().unwrap(), false).into()
            }
            LiteralKind::Float(n) => {
                let f64_type = self.context.f64_type();

                f64_type.const_float((*n).try_into().unwrap()).into()
            }
            LiteralKind::Bool(b) => {
                let bool_type = self.context.bool_type();

                bool_type.const_int((*b).try_into().unwrap(), false).into()
            }
            LiteralKind::String(s) => {
                let global_str = builder.build_global_string_ptr(s, "str");

                global_str.as_basic_value_enum()
            }
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
        match op.kind {
            NativeOperatorKind::IAdd => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_add(left, right, "iadd")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::ISub => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_sub(left, right, "isub")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IMul => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_mul(left, right, "imul")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IDiv => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_signed_div(left, right, "idiv")
                    .as_basic_value_enum()
            }

            // float
            NativeOperatorKind::FAdd => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_add(left, right, "fadd")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FSub => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_sub(left, right, "fsub")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FMul => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_mul(left, right, "fmul")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FDiv => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_div(left, right, "fdiv")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IEq => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_compare(IntPredicate::EQ, left, right, "ieq")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IGT => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_compare(IntPredicate::SGT, left, right, "isgt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IGE => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_compare(IntPredicate::SGE, left, right, "isge")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::ILT => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_compare(IntPredicate::SLT, left, right, "islt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::ILE => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_compare(IntPredicate::SLE, left, right, "isle")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FEq => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OEQ, left, right, "feq")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FGT => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OGT, left, right, "fsgt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FGE => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OGE, left, right, "fsge")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FLT => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OLT, left, right, "fslt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FLE => {
                let left = self.lower_identifier(left, builder).into_float_value();
                let right = self.lower_identifier(right, builder).into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OLE, left, right, "fsle")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::BEq => {
                let left = self.lower_identifier(left, builder).into_int_value();
                let right = self.lower_identifier(right, builder).into_int_value();

                builder
                    .build_int_compare(IntPredicate::EQ, left, right, "beq")
                    .as_basic_value_enum()
            }
        }
    }
}
