use std::convert::{TryFrom, TryInto};

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    passes::{PassManager, PassManagerBuilder},
    targets::{InitializationConfig, Target},
    types::{BasicType, BasicTypeEnum},
    values::{AnyValue, AnyValueEnum, BasicValue, BasicValueEnum, CallableValue, FunctionValue},
    AddressSpace, FloatPredicate, IntPredicate,
    OptimizationLevel::Aggressive,
};

use crate::{
    helpers::scopes::Scopes,
    hir::*,
    ty::{PrimitiveType, Type},
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

    pub fn optimize(&mut self) {
        let config = InitializationConfig::default();

        Target::initialize_native(&config).unwrap();

        let pass_manager_builder = PassManagerBuilder::create();

        pass_manager_builder.set_optimization_level(Aggressive);

        let pass_manager = PassManager::create(());

        pass_manager.add_promote_memory_to_register_pass();
        pass_manager.add_argument_promotion_pass();
        pass_manager.add_always_inliner_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_new_gvn_pass();
        pass_manager.add_function_attrs_pass();
        pass_manager.add_prune_eh_pass();
        pass_manager.add_loop_vectorize_pass();
        pass_manager.add_cfg_simplification_pass();
        pass_manager.add_constant_merge_pass();
        pass_manager.add_scalarizer_pass();
        pass_manager.add_merged_load_store_motion_pass();
        pass_manager.add_ind_var_simplify_pass();
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_licm_pass();
        pass_manager.add_loop_deletion_pass();
        pass_manager.add_loop_unswitch_pass();
        pass_manager.add_memcpy_optimize_pass();
        pass_manager.add_partially_inline_lib_calls_pass();
        pass_manager.add_lower_switch_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_simplify_lib_calls_pass();
        pass_manager.add_tail_call_elimination_pass();
        pass_manager.add_aggressive_inst_combiner_pass();
        pass_manager.add_instruction_simplify_pass();
        pass_manager.add_function_inlining_pass();
        pass_manager.add_global_optimizer_pass();
        pass_manager.add_dead_arg_elimination_pass();
        pass_manager.add_strip_symbol_pass();
        pass_manager.add_strip_dead_prototypes_pass();
        pass_manager.add_internalize_pass(true);
        pass_manager.add_sccp_pass();
        // FIXME: Struct init fail with this pass
        // pass_manager.add_dead_store_elimination_pass();
        pass_manager.add_aggressive_dce_pass();
        pass_manager.add_global_dce_pass();
        pass_manager.add_verifier_pass();

        pass_manager.run_on(&self.module);

        pass_manager_builder.populate_module_pass_manager(&pass_manager);
    }

    pub fn lower_type(&mut self, t: &Type, builder: &'a Builder) -> Result<BasicTypeEnum<'a>, ()> {
        Ok(match t {
            Type::Primitive(PrimitiveType::Int8) => self.context.i8_type().into(),
            Type::Primitive(PrimitiveType::Int64) => self.context.i64_type().into(),
            Type::Primitive(PrimitiveType::Float64) => self.context.f64_type().into(),
            Type::Primitive(PrimitiveType::Bool) => self.context.bool_type().into(),
            Type::Primitive(PrimitiveType::String) => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .into(),
            Type::Primitive(PrimitiveType::Array(inner, size)) => {
                // assuming all types are equals
                self.lower_type(inner, builder)?
                    .array_type(*size as u32)
                    .ptr_type(AddressSpace::Generic)
                    .into()
            }
            Type::Func(f) => {
                let ret_t = f.ret.clone();

                let args = f
                    .arguments
                    .iter()
                    .map(|arg| self.lower_type(arg, builder))
                    .collect::<Vec<_>>();

                let mut args_ret = vec![];

                for arg in args {
                    args_ret.push(arg?);
                }

                let args = args_ret;

                let fn_type = if let Type::Primitive(PrimitiveType::Void) = *ret_t {
                    self.context.void_type().fn_type(args.as_slice(), false)
                } else {
                    self.lower_type(&ret_t, builder)?
                        .fn_type(args.as_slice(), false)
                };

                fn_type.ptr_type(AddressSpace::Generic).into()
            }
            Type::Struct(s) => self
                .context
                .struct_type(
                    s.defs
                        .iter()
                        .map(|(_k, b)| self.lower_type(&(*b), builder).unwrap())
                        .collect::<Vec<_>>()
                        .as_slice(),
                    false,
                )
                .ptr_type(AddressSpace::Generic)
                .into(),
            _ => unimplemented!("Codegen: Cannot lower type {:#?}", t),
        })
    }

    pub fn lower_hir(&mut self, root: &'a Root, builder: &'a Builder) -> Result<(), ()> {
        for item in &root.top_levels {
            match &item.kind {
                TopLevelKind::Prototype(p) => self.lower_prototype(p, builder)?,
                TopLevelKind::Function(f) => self.lower_function_decl(f, builder)?,
            }
        }

        for body in root.bodies.values() {
            self.lower_fn_body(body, builder)?;
        }

        Ok(())
    }

    pub fn lower_prototype(&mut self, p: &'a Prototype, builder: &'a Builder) -> Result<(), ()> {
        let t = self.hir.node_types.get(&p.hir_id).unwrap();

        if let Type::Func(f_type) = t {
            let ret_t = f_type.ret.clone();

            let mut args = vec![];

            for arg in &p.signature.arguments {
                args.push(self.lower_type(arg, builder)?);
            }

            let fn_type = if let Type::Primitive(PrimitiveType::Void) = *ret_t {
                self.context.void_type().fn_type(args.as_slice(), false)
            } else {
                self.lower_type(&ret_t, builder)?
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

        Ok(())
    }

    pub fn lower_function_decl(
        &mut self,
        f: &FunctionDecl,
        builder: &'a Builder,
    ) -> Result<(), ()> {
        let mangled_name = f.get_name();

        // FIXME: This should not happen, panic here or return an error instead
        if self.module.get_function(&mangled_name).is_some() {
            return Ok(());
        }
        // Check if any argument is not solved
        if f.arguments
            .iter()
            .any(|arg| self.hir.node_types.get(&arg.name.hir_id).is_none())
        {
            panic!("SOME ARGUMENTS ARE NOT SOLVED");
        }

        let t = self.hir.node_types.get(&f.hir_id).unwrap();

        if let Type::Func(f_type) = t {
            let ret_t = f_type.ret.clone();

            let args = f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(arg, builder))
                .collect::<Vec<_>>();

            let mut args_ret = vec![];
            for arg in args {
                args_ret.push(arg?);
            }
            let args = args_ret;

            let fn_type = if let Type::Primitive(PrimitiveType::Void) = *ret_t {
                self.context.void_type().fn_type(args.as_slice(), false)
            } else {
                self.lower_type(&ret_t, builder)?
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
        } else {
            panic!("Not a function");
        }

        Ok(())
    }

    pub fn lower_argument_decl(
        &mut self,
        arg: &ArgumentDecl,
        builder: &'a Builder,
    ) -> Result<BasicTypeEnum<'a>, ()> {
        let t = self.hir.node_types.get(&arg.name.hir_id).unwrap();

        self.lower_type(t, builder)
    }

    pub fn lower_fn_body(&mut self, fn_body: &'a FnBody, builder: &'a Builder) -> Result<(), ()> {
        let top_f = self.hir.get_function_by_hir_id(&fn_body.fn_id).unwrap();

        if let Some(f) = self.module.get_function(&top_f.get_name()) {
            self.cur_func = Some(f);

            let f_decl = self.hir.get_function_by_hir_id(&fn_body.fn_id).unwrap();

            for (i, arg) in f_decl.arguments.iter().enumerate() {
                let param = f.get_nth_param(i.try_into().unwrap()).unwrap();

                param.set_name(&arg.name.name);

                self.scopes.add(arg.name.hir_id.clone(), param);
            }

            self.lower_body(&fn_body.body, "entry", builder)?;
        } else {
            panic!("Cannot find function {:?}", top_f.get_name());
        }

        Ok(())
    }

    pub fn lower_body(
        &mut self,
        body: &'a Body,
        name: &str,
        builder: &'a Builder,
    ) -> Result<(AnyValueEnum<'a>, BasicBlock<'a>), ()> {
        let basic_block = self
            .context
            .append_basic_block(self.cur_func.unwrap(), name);

        builder.position_at_end(basic_block);

        let stmt = body
            .stmts
            .iter()
            .map(|stmt| self.lower_stmt(stmt, builder))
            .last()
            .unwrap()?;

        Ok((stmt, basic_block))
    }

    pub fn lower_stmt(
        &mut self,
        stmt: &'a Statement,
        builder: &'a Builder,
    ) -> Result<AnyValueEnum<'a>, ()> {
        Ok(match &*stmt.kind {
            StatementKind::Expression(e) => self.lower_expression(e, builder)?.as_any_value_enum(),
            StatementKind::If(e) => self.lower_if(e, builder)?.0,
            StatementKind::Assign(a) => self.lower_assign(a, builder)?,
        })
    }

    pub fn lower_assign(
        &mut self,
        assign: &'a Assign,
        builder: &'a Builder,
    ) -> Result<AnyValueEnum<'a>, ()> {
        Ok(match &assign.name {
            AssignLeftSide::Identifier(id) => {
                let value = self.lower_expression(&assign.value, builder)?;

                self.scopes
                    .add(id.get_hir_id(), value.as_basic_value_enum());

                value
            }
            AssignLeftSide::Indice(indice) => {
                let ptr = self.lower_indice_ptr(indice, builder)?.into_pointer_value();

                let value = self.lower_expression(&assign.value, builder)?;

                builder.build_store(ptr, value);

                value
            }
            AssignLeftSide::Dot(dot) => {
                let ptr = self.lower_dot_ptr(dot, builder)?.into_pointer_value();

                let value = self.lower_expression(&assign.value, builder)?;

                builder.build_store(ptr, value);

                value
            }
        }
        .as_any_value_enum())
    }

    pub fn lower_if(
        &mut self,
        r#if: &'a If,
        builder: &'a Builder,
    ) -> Result<(AnyValueEnum<'a>, BasicBlock<'a>), ()> {
        let block = builder.get_insert_block().unwrap();

        builder.position_at_end(block);

        let predicat = self.lower_expression(&r#if.predicat, builder)?;

        let (_, then_block) = self.lower_body(&r#if.body, "then", builder)?;

        let else_block = if let Some(e) = &r#if.else_ {
            let else_block = self.lower_else(e, builder)?;

            else_block
        } else {
            //new empty block
            let f = self.module.get_last_function().unwrap();

            self.context.append_basic_block(f, "else")
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

        Ok((if_value.as_any_value_enum(), block))
    }

    pub fn lower_else(
        &mut self,
        r#else: &'a Else,
        builder: &'a Builder,
    ) -> Result<BasicBlock<'a>, ()> {
        Ok(match &r#else {
            Else::If(i) => {
                let block = self
                    .context
                    .append_basic_block(self.cur_func.unwrap(), "if");

                builder.position_at_end(block);

                self.lower_if(i, builder)?.1
            }
            Else::Body(b) => self.lower_body(b, "else", builder)?.1,
        })
    }

    pub fn lower_expression(
        &mut self,
        expr: &'a Expression,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        Ok(match &*expr.kind {
            ExpressionKind::Lit(l) => self.lower_literal(l, builder)?,
            ExpressionKind::Identifier(id) => self.lower_identifier_path(id, builder)?,
            ExpressionKind::FunctionCall(fc) => self.lower_function_call(fc, builder)?,
            ExpressionKind::StructCtor(s) => self.lower_struct_ctor(s, builder)?,
            ExpressionKind::Indice(i) => self.lower_indice(i, builder)?,
            ExpressionKind::Dot(d) => self.lower_dot(d, builder)?,
            ExpressionKind::NativeOperation(op, left, right) => {
                self.lower_native_operation(op, left, right, builder)?
            }
            ExpressionKind::Return(expr) => {
                let val = self.lower_expression(expr, builder)?;

                builder.build_return(Some(&val.as_basic_value_enum()));

                val
            }
        })
    }

    pub fn lower_struct_ctor(
        &mut self,
        s: &'a StructCtor,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let t = self.hir.node_types.get(&s.get_hir_id()).unwrap();
        let struct_t = t.as_struct_type();

        let llvm_struct_t_ptr = self.lower_type(t, builder).unwrap().into_pointer_type();
        let llvm_struct_t = llvm_struct_t_ptr.get_element_type().into_struct_type();

        // FIXME: types should be ordered already
        let defs = struct_t
            .defs
            .iter()
            .map(|(k, _b)| {
                let def = s
                    .defs
                    .iter()
                    .find(|(k2, _b2)| k2.name == *k)
                    .map(|(_k2, b2)| b2)
                    .unwrap();

                self.lower_expression(def, builder).unwrap()
            })
            .collect::<Vec<_>>();

        let ptr = builder.build_alloca(llvm_struct_t, "struct_ptr");

        for (i, def) in defs.iter().enumerate() {
            let inner_ptr = builder
                .build_struct_gep(ptr, i as u32, "struct_inner")
                .unwrap();

            builder.build_store(inner_ptr, *def);
        }

        Ok(ptr.into())
    }

    pub fn lower_function_call(
        &mut self,
        fc: &'a FunctionCall,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let terminal_hir_id = fc.op.get_terminal_hir_id();

        let f_id = self.hir.resolutions.get(&terminal_hir_id).unwrap();

        let callable_value = match self.hir.get_top_level(f_id) {
            Some(top) => CallableValue::try_from(match &top.kind {
                TopLevelKind::Prototype(p) => {
                    self.module.get_function(&p.name.to_string()).unwrap()
                }
                TopLevelKind::Function(f) => {
                    self.module.get_function(&f.get_name().to_string()).unwrap()
                }
            })
            .unwrap(),
            None => CallableValue::try_from(
                self.lower_expression(&fc.op, builder)?.into_pointer_value(),
            )
            .unwrap(),
        };

        let mut arguments = vec![];

        for arg in &fc.args {
            arguments.push(self.lower_expression(arg, builder)?);
        }

        Ok(builder
            .build_call(
                callable_value,
                arguments.as_slice(),
                format!("call_{}", fc.op.as_identifier().name).as_str(),
            )
            .try_as_basic_value()
            .left()
            .unwrap())
    }

    pub fn lower_indice_ptr(
        &mut self,
        indice: &'a Indice,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let op = self
            .lower_expression(&indice.op, builder)?
            .into_pointer_value();

        let indice = self
            .lower_expression(&indice.value, builder)?
            .into_int_value();

        let i64_type = self.context.i64_type();

        let const_0 = i64_type.const_zero();

        let ptr = unsafe { builder.build_gep(op, &[const_0, indice], "index") };
        // builder.build_load(ptr, "load_indice");
        Ok(ptr.as_basic_value_enum())
    }

    pub fn lower_indice(
        &mut self,
        indice: &'a Indice,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let ptr = self.lower_indice_ptr(indice, builder)?.into_pointer_value();

        Ok(builder.build_load(ptr, "load_indice"))
    }

    pub fn lower_dot_ptr(
        &mut self,
        dot: &'a Dot,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let op = self
            .lower_expression(&dot.op, builder)?
            .into_pointer_value();

        let t = self.hir.node_types.get(&dot.op.get_hir_id()).unwrap();

        let struct_t = t.as_struct_type();

        let indice = struct_t
            .defs
            .iter()
            .enumerate()
            .find(|(_i, (k, _v))| **k == dot.value.name)
            .map(|(i, _)| i)
            .unwrap();

        let i32_type = self.context.i32_type();

        let const_0 = i32_type.const_zero();
        let indice = i32_type.const_int(indice as u64, false);

        let ptr = unsafe { builder.build_gep(op, &[const_0, indice], "struct_index") };

        Ok(ptr.as_basic_value_enum())
    }

    pub fn lower_dot(
        &mut self,
        dot: &'a Dot,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let ptr = self.lower_dot_ptr(dot, builder)?.into_pointer_value();

        Ok(builder.build_load(ptr, "load_dot"))
    }

    pub fn lower_literal(
        &mut self,
        lit: &'a Literal,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        Ok(match &lit.kind {
            LiteralKind::Number(n) => {
                let i64_type = self.context.i64_type();

                i64_type.const_int((*n).try_into().unwrap(), false).into()
            }
            LiteralKind::Float(n) => {
                let f64_type = self.context.f64_type();

                f64_type.const_float(*n).into()
            }
            LiteralKind::Bool(b) => {
                let bool_type = self.context.bool_type();

                bool_type.const_int((*b).try_into().unwrap(), false).into()
            }
            LiteralKind::String(s) => {
                let global_str = builder.build_global_string_ptr(s, "str");

                global_str.as_basic_value_enum()
            }
            LiteralKind::Array(arr) => {
                let arr_type = self
                    .lower_type(self.hir.node_types.get(&lit.hir_id).unwrap(), builder)
                    .unwrap()
                    .into_pointer_type()
                    .get_element_type()
                    .into_array_type();

                let ptr = builder.build_alloca(arr_type, "array");

                arr.values.iter().enumerate().for_each(|(i, expr)| {
                    let expr = self.lower_expression(expr, builder).unwrap();

                    let i64_type = self.context.i64_type();

                    let const_i = i64_type.const_int(i as u64, false);
                    let const_0 = i64_type.const_zero();

                    let inner_ptr = unsafe {
                        builder.build_gep(ptr, &[const_0, const_i], format!("elem_{}", i).as_str())
                    };

                    builder.build_store(inner_ptr, expr);
                });

                ptr.as_basic_value_enum()
            }
        })
    }

    pub fn lower_identifier_path(
        &mut self,
        id: &IdentifierPath,
        _builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        self.lower_identifier(id.path.iter().last().unwrap(), _builder)
    }

    pub fn lower_identifier(
        &mut self,
        id: &Identifier,
        _builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        let reso = self.hir.resolutions.get(&id.hir_id).unwrap();

        let val = match self.scopes.get(reso) {
            None => {
                return Err(());
            }
            Some(val) => val,
        };

        Ok(val)
    }

    pub fn lower_native_operation(
        &mut self,
        op: &NativeOperator,
        left: &Identifier,
        right: &Identifier,
        builder: &'a Builder,
    ) -> Result<BasicValueEnum<'a>, ()> {
        Ok(match op.kind {
            NativeOperatorKind::IAdd => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_add(left, right, "iadd")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::ISub => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_sub(left, right, "isub")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IMul => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_mul(left, right, "imul")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IDiv => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_signed_div(left, right, "idiv")
                    .as_basic_value_enum()
            }

            // float
            NativeOperatorKind::FAdd => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_add(left, right, "fadd")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FSub => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_sub(left, right, "fsub")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FMul => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_mul(left, right, "fmul")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FDiv => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_div(left, right, "fdiv")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::IEq => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_compare(IntPredicate::EQ, left, right, "ieq")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Igt => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_compare(IntPredicate::SGT, left, right, "isgt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Ige => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_compare(IntPredicate::SGE, left, right, "isge")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Ilt => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_compare(IntPredicate::SLT, left, right, "islt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Ile => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_compare(IntPredicate::SLE, left, right, "isle")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::FEq => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OEQ, left, right, "feq")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Fgt => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OGT, left, right, "fsgt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Fge => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OGE, left, right, "fsge")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Flt => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OLT, left, right, "fslt")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::Fle => {
                let left = self.lower_identifier(left, builder)?.into_float_value();
                let right = self.lower_identifier(right, builder)?.into_float_value();

                builder
                    .build_float_compare(FloatPredicate::OLE, left, right, "fsle")
                    .as_basic_value_enum()
            }
            NativeOperatorKind::BEq => {
                let left = self.lower_identifier(left, builder)?.into_int_value();
                let right = self.lower_identifier(right, builder)?.into_int_value();

                builder
                    .build_int_compare(IntPredicate::EQ, left, right, "beq")
                    .as_basic_value_enum()
            }
        })
    }
}
