# MIR-Backed Codegen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish roadmap Task 21 by moving executable LLVM body emission from HIR expression/statement trees to canonical MIR while preserving current behavior, ABI, products, and linking.

**Architecture:** Build monomorphized MIR as the executable backend boundary, add MIR-specific codegen modules under `lib/src/codegen/mir/`, and keep HIR/mono only for declarations, layout metadata, instance records, product/link metadata, and dependency object linkage. The final compile path must emit function bodies from MIR statements and terminators with no active HIR body fallback.

**Tech Stack:** Rust 2021, `rock-lib`, existing `inkwell` LLVM backend, `MirProgram`, `MonomorphizedProgram`, `InstanceRecord`, canonical IDs (`DefId`, `InstanceId`, `MirFunctionId`, `MirCallable`), existing integration suite.

---

## Approved Spec

- `docs/superpowers/specs/2026-05-23-mir-backed-codegen-design.md`

## File Structure

- Modify `lib/src/mir/identity.rs`: add boxed/non-recursive closure function identity support for codegen and tests.
- Modify `lib/src/mir/mod.rs`: re-export any new MIR identity variants/types and keep MIR core forms stable.
- Modify `lib/src/mir/builder/mod.rs`: add monomorphized MIR build entrypoint and closure body MIR construction.
- Modify `lib/src/mir/builder/expr.rs`: lower lambda bodies into MIR closure functions and keep closure rvalues as callable values.
- Modify `lib/src/codegen/mod.rs`: register new MIR backend module, split declaration setup from body emission, and expose MIR compile entrypoint.
- Create `lib/src/codegen/mir/mod.rs`: top-level MIR program/function lowering, local storage, block setup, function verification.
- Create `lib/src/codegen/mir/place.rs`: MIR place address/value lowering.
- Create `lib/src/codegen/mir/operand.rs`: MIR operand and constant lowering.
- Create `lib/src/codegen/mir/rvalue.rs`: MIR rvalue lowering.
- Create `lib/src/codegen/mir/terminator.rs`: MIR terminator lowering.
- Create `lib/src/codegen/mir/assert.rs`: MIR assertion/runtime failure lowering.
- Modify `lib/src/codegen/intrinsics.rs`: factor a value-based intrinsic helper reusable from MIR.
- Modify `lib/src/codegen/operators.rs`: keep value-based operator helpers reusable from MIR.
- Modify `lib/src/codegen/closures.rs`: keep callable ABI helpers reusable without requiring HIR lambda bodies.
- Modify `lib/src/lib.rs`: build monomorphized MIR, run borrowck/agreement on executable MIR, and call MIR-backed codegen.
- Modify focused tests in `lib/src/mir/builder/mod.rs`, `lib/src/codegen/mod.rs`, `lib/src/codegen/mir/mod.rs`, `lib/src/codegen/intrinsics.rs`, and `lib/tests/integration.rs` where this plan explicitly asks for coverage.

## Task 1: Build Executable MIR From Monomorphized Instances

**Files:**
- Modify: `lib/src/mir/identity.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Write failing monomorphized MIR tests**

Add tests in the `#[cfg(test)]` module of `lib/src/mir/builder/mod.rs`:

```rust
#[test]
fn build_monomorphized_program_keys_functions_by_instance_id() {
    use crate::mono::{InstanceKey, InstanceOrigin, InstanceRecord, MonomorphizedProgram};

    let function_id = DefId::new(CrateId(0), LocalDefId(10));
    let instance_id = crate::ids::InstanceId(7);
    let function = HirFunction {
        id: function_id,
        name: "main".to_string(),
        qualified_name: Some("main".to_string()),
        generic_params: Vec::new(),
        generic_param_ids: Vec::new(),
        generic_bounds: std::collections::HashMap::new(),
        params: Vec::new(),
        ret_type: Type::I64,
        body: HirBlock {
            stmts: vec![HirStmt::Expr(HirExpr {
                kind: HirExprKind::IntLiteral(42),
                ty: Type::I64,
                span: Default::default(),
            })],
            ty: Type::I64,
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };
    let program = HirProgram::from_parts(
        std::collections::HashMap::from([("main".to_string(), function.clone())]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    let mut instances = std::collections::BTreeMap::new();
    instances.insert(
        instance_id,
        InstanceRecord {
            id: instance_id,
            origin: InstanceOrigin::Function(function_id),
            substitution: Vec::new(),
            source_name: "main".to_string(),
            backend_symbol: "main".to_string(),
            declared: None,
            body: Some(function),
            provided_by_object: false,
            is_specialization: false,
        },
    );
    let monomorphized = MonomorphizedProgram { program, instances };

    let mir = MirBuilder::build_monomorphized(&monomorphized);

    let id = crate::mir::MirFunctionId::Instance(instance_id);
    let function = mir.function(id).expect("instance MIR body");
    assert_eq!(function.id, id);
    assert_eq!(function.name, "main");
}
```

- [ ] **Step 2: Run the red test**

Run: `cargo test -p rock-lib build_monomorphized_program_keys_functions_by_instance_id -- --nocapture`

Expected: FAIL because `MirBuilder::build_monomorphized` does not exist.

- [ ] **Step 3: Add monomorphized build entrypoint**

In `lib/src/mir/builder/mod.rs`, add this public entrypoint near `MirBuilder::build`:

```rust
pub fn build_monomorphized(program: &crate::mono::MonomorphizedProgram) -> MirProgram {
    let mut functions = std::collections::BTreeMap::new();

    for record in program.instances.values() {
        if record.provided_by_object {
            continue;
        }
        let Some(body) = record.body.as_ref() else {
            continue;
        };

        let mut builder = MirBuilder::new(&program.program);
        let mir_id = MirFunctionId::Instance(record.id);
        let mir_func = builder.build_function(mir_id, &record.backend_symbol, body);
        functions.insert(mir_id, mir_func);
    }

    MirProgram { functions }
}
```

Keep the existing `build(&HirProgram)` entrypoint for non-codegen tests until the pipeline switch is complete.

- [ ] **Step 4: Verify the green test**

Run: `cargo test -p rock-lib build_monomorphized_program_keys_functions_by_instance_id -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/builder/mod.rs
git commit -m "build executable mir from instances"
```

## Task 2: Represent Closure Bodies As MIR Functions

**Files:**
- Modify: `lib/src/mir/identity.rs`
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Write failing closure body MIR test**

Add this test in `lib/src/mir/builder/mod.rs`:

```rust
#[test]
fn build_function_emits_closure_body_function() {
    let function_id = DefId::new(CrateId(0), LocalDefId(20));
    let owner = crate::mir::MirFunctionId::Function(function_id);
    let lambda = HirExpr {
        kind: HirExprKind::Lambda {
            params: vec![HirParam {
                name: "x".to_string(),
                ty: Type::I64,
                mutable: false,
            }],
            body: HirBlock {
                stmts: vec![HirStmt::Expr(HirExpr {
                    kind: HirExprKind::Var("x".to_string()),
                    ty: Type::I64,
                    span: Default::default(),
                })],
                ty: Type::I64,
            },
            captures: Vec::new(),
        },
        ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
        span: Default::default(),
    };
    let function = HirFunction {
        id: function_id,
        name: "make_lambda".to_string(),
        qualified_name: Some("make_lambda".to_string()),
        generic_params: Vec::new(),
        generic_param_ids: Vec::new(),
        generic_bounds: std::collections::HashMap::new(),
        params: Vec::new(),
        ret_type: lambda.ty.clone(),
        body: HirBlock {
            stmts: vec![HirStmt::Expr(lambda)],
            ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };
    let program = HirProgram::from_parts(
        std::collections::HashMap::from([("make_lambda".to_string(), function)]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        Vec::new(),
        Vec::new(),
    );

    let mir = MirBuilder::build(&program);
    let closure_id = crate::mir::MirClosureId {
        owner,
        local_index: 0,
    };

    assert!(mir
        .function(crate::mir::MirFunctionId::Closure(Box::new(closure_id.clone())))
        .is_some());
}
```

- [ ] **Step 2: Run the red test**

Run: `cargo test -p rock-lib build_function_emits_closure_body_function -- --nocapture`

Expected: FAIL because `MirFunctionId::Closure` and closure body MIR insertion do not exist.

- [ ] **Step 3: Add closure function identity**

In `lib/src/mir/identity.rs`, extend `MirFunctionId` with a boxed closure key. Remove `Copy` from `MirFunctionId` and `MirClosureId` derives and use `.clone()` at call sites that previously relied on copying MIR function IDs:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MirFunctionId {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
    Closure(Box<MirClosureId>),
}
```

Update `display_fallback` with:

```rust
MirFunctionId::Closure(id) => format!("closure#{:?}:{}", id.owner, id.local_index),
```

- [ ] **Step 4: Store closure body functions during MIR build**

In `MirBuilder`, add a `nested_functions: Vec<MirFunction>` field initialized to `Vec::new()`. In `build_function`, append nested functions to the returned `MirProgram` from `build` and `build_monomorphized` by inserting `mir_func.id` for each nested function after the parent body is built.

Use this helper inside `MirBuilder`:

```rust
fn take_nested_functions(&mut self) -> Vec<MirFunction> {
    std::mem::take(&mut self.nested_functions)
}
```

In `HirExprKind::Lambda`, stop ignoring `params` and `body`. After creating `closure_id`, create a `HirFunction` for the closure body with:

```rust
let closure_function = HirFunction {
    id: crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(0)),
    name: lambda_name.clone(),
    qualified_name: Some(lambda_name.clone()),
    generic_params: Vec::new(),
    generic_param_ids: Vec::new(),
    generic_bounds: std::collections::HashMap::new(),
    params: params.clone(),
    ret_type: body.ty.clone(),
    body: body.as_ref().clone(),
    is_curried: false,
    is_method: false,
    self_receiver: None,
    is_unsafe: false,
};
```

Build it with `MirFunctionId::Closure(Box::new(closure_id.clone()))` and push it into `nested_functions`. Keep closure captures on the `Rvalue::Closure` and parent `closure_captures` list.

- [ ] **Step 5: Verify closure body MIR**

Run: `cargo test -p rock-lib build_function_emits_closure_body_function -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::identity -- --nocapture && cargo test -p rock-lib mir::builder -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/identity.rs lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs
git commit -m "represent closure bodies in mir"
```

## Task 3: Split Codegen Declaration Setup From Body Emission

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Write failing declaration-only setup test**

Add a test in `lib/src/codegen/mod.rs` that calls a new declaration setup method without compiling HIR bodies:

```rust
#[test]
fn prepare_program_declarations_registers_instance_symbols_without_bodies() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let function = function_with_expr(expr(HirExprKind::IntLiteral(1), Type::I64));
    let program = program_with_expr(expr(HirExprKind::IntLiteral(1), Type::I64));
    let instance_id = InstanceId(1);
    let mut instances = BTreeMap::new();
    instances.insert(
        instance_id,
        InstanceRecord {
            id: instance_id,
            origin: InstanceOrigin::Function(function.id),
            substitution: Vec::new(),
            source_name: "main".to_string(),
            backend_symbol: "main".to_string(),
            declared: None,
            body: Some(function),
            provided_by_object: false,
            is_specialization: false,
        },
    );
    let mono = MonomorphizedProgram { program, instances };

    codegen.prepare_program_declarations(&mono).unwrap();

    assert_eq!(codegen.instance_symbols_by_id.get(&instance_id), Some(&"main".to_string()));
    assert!(codegen.functions.contains_key("main"));
}
```

- [ ] **Step 2: Run the red test**

Run: `cargo test -p rock-lib prepare_program_declarations_registers_instance_symbols_without_bodies -- --nocapture`

Expected: FAIL because `prepare_program_declarations` does not exist.

- [ ] **Step 3: Extract declaration setup**

In `CodeGen`, extract the setup portion of `compile_program` into:

```rust
pub(crate) fn prepare_program_declarations(
    &mut self,
    program: &MonomorphizedProgram,
) -> Result<(), CodegenError> {
    let instances = &program.instances;
    let hir_program = &program.program;

    self.register_index_trait_targets(hir_program, instances);
    self.register_nominal_layouts(hir_program);
    self.register_instances(instances);
    self.register_impl_method_aliases(hir_program);
    let impls = hir_program
        .impls_in_order()
        .map(|(_, imp)| imp.clone())
        .collect::<Vec<_>>();
    self.register_trait_member_ids(hir_program);
    self.register_trait_impls(&impls);
    self.declare_runtime();
    for (_, ext) in hir_program.externs_in_order() {
        if self.functions.get(&ext.name).is_none() {
            self.declare_extern(ext);
        }
        self.extern_symbols_by_id.insert(ext.id, ext.name.clone());
    }

    Ok(())
}
```

Move the struct/enum layout registration loops into a private `register_nominal_layouts(&mut self, program: &HirProgram)` helper.

Keep `compile_program` behavior unchanged by making it call `prepare_program_declarations` and then the existing HIR body loop.

- [ ] **Step 4: Verify declaration setup**

Run: `cargo test -p rock-lib prepare_program_declarations_registers_instance_symbols_without_bodies -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs
git commit -m "split codegen declaration setup"
```

## Task 4: Add MIR Codegen Skeleton For Functions And Locals

**Files:**
- Create: `lib/src/codegen/mir/mod.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing MIR function skeleton test**

Create `lib/src/codegen/mir/mod.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use inkwell::context::Context;

    use crate::ids::{CrateId, DefId, InstanceId, LocalDefId};
    use crate::mir::{BasicBlock, LocalDecl, MirFunction, MirFunctionId, MirProgram, Mutability, Terminator};
    use crate::types::Type;

    use super::super::CodeGen;

    #[test]
    fn mir_codegen_declares_and_verifies_empty_return_function() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let function_id = MirFunctionId::Instance(InstanceId(0));
        let function = MirFunction {
            id: function_id,
            name: "main".to_string(),
            basic_blocks: vec![BasicBlock {
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
            }],
            local_decls: vec![LocalDecl {
                ty: Type::I64,
                mutability: Mutability::Mut,
                name: Some("return_place".to_string()),
                span: None,
            }],
            closure_captures: Vec::new(),
            arg_count: 0,
            ret_type: Type::I64,
        };
        let mir = MirProgram {
            functions: BTreeMap::from([(function_id, function)]),
        };

        codegen.declare_mir_function_for_test(function_id, "main", &[], &Type::I32);
        codegen.compile_mir_program(&mir).unwrap();

        assert!(codegen.get_ir().contains("define i32 @main"));
    }
}
```

- [ ] **Step 2: Register module and run red test**

In `lib/src/codegen/mod.rs`, add `mod mir;` near the other modules.

Run: `cargo test -p rock-lib mir_codegen_declares_and_verifies_empty_return_function -- --nocapture`

Expected: FAIL because `compile_mir_program` and the test declaration helper do not exist.

- [ ] **Step 3: Add MIR codegen state and skeleton methods**

In `lib/src/codegen/mir/mod.rs`, add:

```rust
use std::collections::HashMap;

use inkwell::values::{FunctionValue, PointerValue};

use crate::mir::{Local, MirFunction, MirFunctionId, MirProgram};
use crate::types::Type;

use super::{CodeGen, CodegenError};

pub(crate) struct MirFunctionContext<'ctx> {
    function: FunctionValue<'ctx>,
    locals: Vec<Option<(PointerValue<'ctx>, Type)>>,
    blocks: Vec<inkwell::basic_block::BasicBlock<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_mir_program(&mut self, program: &MirProgram) -> Result<(), CodegenError> {
        for (_, function) in program.functions() {
            self.compile_mir_function(function)?;
        }
        Ok(())
    }

    fn compile_mir_function(&mut self, function: &MirFunction) -> Result<(), CodegenError> {
        let llvm_function = self.mir_function_value(function)?;
        let mut ctx = self.create_mir_function_context(function, llvm_function)?;
        self.current_function = Some(llvm_function);
        self.builder.position_at_end(ctx.blocks[0]);

        for (block_idx, block) in function.basic_blocks.iter().enumerate() {
            self.builder.position_at_end(ctx.blocks[block_idx]);
            for statement in &block.statements {
                self.compile_mir_statement(function, &mut ctx, statement)?;
            }
            if let Some(terminator) = &block.terminator {
                self.compile_mir_terminator(function, &mut ctx, terminator)?;
            }
        }

        self.current_function = None;
        if !llvm_function.verify(true) {
            return Err(CodegenError::from(format!(
                "MIR function '{}' failed verification",
                function.name
            )));
        }
        Ok(())
    }
}
```

Add stubs in the same file for `compile_mir_statement` and `compile_mir_terminator`. The initial `Terminator::Return` lowering should return the default value for the MIR function return type, with `main` preserving i32 return ABI.

- [ ] **Step 4: Add test-only declaration helper**

In `CodeGen`, add under `#[cfg(test)]`:

```rust
pub(crate) fn declare_mir_function_for_test(
    &mut self,
    id: crate::mir::MirFunctionId,
    symbol: &str,
    params: &[Type],
    ret: &Type,
) {
    let fn_type = self.function_type(params, ret);
    let function = self.module.add_function(symbol, fn_type, None);
    self.functions.insert(symbol.to_string(), function);
    self.mir_function_symbols.insert(id, symbol.to_string());
}
```

Add `mir_function_symbols: HashMap<MirFunctionId, String>` to `CodeGen` and initialize it in `CodeGen::new`.

- [ ] **Step 5: Verify skeleton**

Run: `cargo test -p rock-lib mir_codegen_declares_and_verifies_empty_return_function -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/mir/mod.rs
git commit -m "add mir codegen function skeleton"
```

## Task 5: Lower MIR Places, Operands, And Straight-Line Assignments

**Files:**
- Create: `lib/src/codegen/mir/place.rs`
- Create: `lib/src/codegen/mir/operand.rs`
- Modify: `lib/src/codegen/mir/mod.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing straight-line MIR test**

Add this test to `lib/src/codegen/mir/mod.rs` tests:

```rust
#[test]
fn mir_codegen_stores_constant_assignment_to_return_place() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let function_id = MirFunctionId::Instance(InstanceId(0));
    let function = MirFunction {
        id: function_id,
        name: "main".to_string(),
        basic_blocks: vec![BasicBlock {
            statements: vec![crate::mir::StatementData::assign(
                crate::mir::Place { local: crate::mir::Local(0), projection: vec![] },
                crate::mir::Rvalue::Use(crate::mir::Operand::Constant(crate::mir::Constant::Int(42))),
                None,
            )],
            terminator: Some(Terminator::Return),
        }],
        local_decls: vec![LocalDecl {
            ty: Type::I64,
            mutability: Mutability::Mut,
            name: Some("return_place".to_string()),
            span: None,
        }],
        closure_captures: Vec::new(),
        arg_count: 0,
        ret_type: Type::I64,
    };
    let mir = MirProgram { functions: BTreeMap::from([(function_id, function)]) };

    codegen.declare_mir_function_for_test(function_id, "main", &[], &Type::I32);
    codegen.compile_mir_program(&mir).unwrap();

    let ir = codegen.get_ir();
    assert!(ir.contains("store i64 42"));
    assert!(ir.contains("ret i32 42"));
}
```

- [ ] **Step 2: Run the red test**

Run: `cargo test -p rock-lib mir_codegen_stores_constant_assignment_to_return_place -- --nocapture`

Expected: FAIL because statements, operands, local storage, and return-place loading are not implemented.

- [ ] **Step 3: Add place and operand modules**

Create `place.rs` with:

```rust
use inkwell::values::{BasicValueEnum, PointerValue};

use crate::mir::{Place, Projection};

use super::{CodeGen, CodegenError, MirFunctionContext};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_mir_place_address(
        &mut self,
        ctx: &MirFunctionContext<'ctx>,
        place: &Place,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        let Some(Some((mut ptr, mut ty))) = ctx.locals.get(place.local.0).cloned() else {
            return Err(CodegenError::from(format!("MIR local {:?} has no storage", place.local)));
        };

        for projection in &place.projection {
            match projection {
                Projection::Deref => {
                    let loaded = self.builder.build_load(self.llvm_type(&ty), ptr, "deref_ptr")
                        .map_err(|e| CodegenError::from(format!("Failed to load deref ptr: {e}")))?;
                    ptr = loaded.into_pointer_value();
                    if let crate::types::Type::Reference { inner, .. } | crate::types::Type::Pointer(inner) = ty {
                        ty = *inner;
                    }
                }
                Projection::Field { index, .. } => {
                    ptr = self.builder.build_struct_gep(self.llvm_type(&ty), ptr, *index as u32, "field")
                        .map_err(|e| CodegenError::from(format!("Failed to get field address: {e}")))?;
                }
                Projection::Index(index_local) => {
                    let Some(Some((index_ptr, index_ty))) = ctx.locals.get(index_local.0).cloned() else {
                        return Err(CodegenError::from(format!("MIR index local {:?} has no storage", index_local)));
                    };
                    let index = self.builder.build_load(self.llvm_type(&index_ty), index_ptr, "idx")
                        .map_err(|e| CodegenError::from(format!("Failed to load index: {e}")))?
                        .into_int_value();
                    ptr = unsafe { self.builder.build_gep(self.llvm_type(&ty), ptr, &[index], "index")
                        .map_err(|e| CodegenError::from(format!("Failed to get index address: {e}")))? };
                }
                Projection::Downcast(_) => {}
            }
        }

        Ok(ptr)
    }

    pub(crate) fn compile_mir_place_value(
        &mut self,
        ctx: &MirFunctionContext<'ctx>,
        place: &Place,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let ptr = self.compile_mir_place_address(ctx, place)?;
        let ty = ctx.local_type(place.local)?;
        self.builder.build_load(self.llvm_type(&ty), ptr, "place_val")
            .map_err(|e| CodegenError::from(format!("Failed to load place: {e}")))
    }
}
```

Create `operand.rs` with primitive constants and `Copy`/`Move` using `compile_mir_place_value`.

- [ ] **Step 4: Implement local allocation and assignment**

In `create_mir_function_context`, allocate every local in the entry block using `local_decls`. Store parameters into locals `1..=arg_count`, leaving local `0` as the return place.

In `compile_mir_statement`, implement:

```rust
crate::mir::StatementKind::Assign(place, rvalue) => {
    let value = self.compile_mir_rvalue(function, ctx, rvalue)?;
    let target = self.compile_mir_place_address(ctx, place)?;
    let target_ty = self.llvm_type(&ctx.local_type(place.local)?);
    let value = self.coerce_value(value, target_ty)?;
    self.builder.build_store(target, value)
        .map_err(|e| CodegenError::from(format!("Failed to store MIR assignment: {e}")))?;
}
```

In `Terminator::Return`, load local `0`, preserve `main` i32 truncation, and return it.

- [ ] **Step 5: Verify straight-line MIR assignment**

Run: `cargo test -p rock-lib mir_codegen_stores_constant_assignment_to_return_place -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir
git commit -m "lower mir locals and operands to llvm"
```

## Task 6: Lower Primitive MIR Rvalues, Assertions, And Intrinsics

**Files:**
- Create: `lib/src/codegen/mir/rvalue.rs`
- Create: `lib/src/codegen/mir/assert.rs`
- Modify: `lib/src/codegen/intrinsics.rs`
- Modify: `lib/src/codegen/mir/mod.rs`
- Test: `lib/src/codegen/mir/mod.rs`
- Test: `lib/src/codegen/intrinsics.rs`

- [ ] **Step 1: Write failing primitive rvalue tests**

Add tests for `BinaryOp`, `UnaryOp`, `Cast`, and `Assert(BoundsCheck)` in `codegen::mir` tests. Use MIR functions with one block assigning to local `0` and returning.

The bounds check test should assert that generated IR contains `arr_oob` and `oob_exit`:

```rust
assert!(ir.contains("arr_oob"));
assert!(ir.contains("oob_exit"));
```

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_primitive -- --nocapture`

Expected: FAIL because primitive MIR rvalue lowering and MIR assertions are incomplete.

- [ ] **Step 3: Factor value-based intrinsic helper**

In `lib/src/codegen/intrinsics.rs`, add:

```rust
pub(crate) fn compile_intrinsic_values(
    &mut self,
    name: &str,
    compiled_args: &[BasicValueEnum<'ctx>],
    _ret_ty: &Type,
) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
    match name {
        "I8Add" | "I16Add" | "I32Add" | "I64Add" | "U8Add" | "U16Add" | "U32Add" | "U64Add" => {
            let lhs = compiled_args[0].into_int_value();
            let rhs = compiled_args[1].into_int_value();
            Ok(Some(self.builder.build_int_add(lhs, rhs, "add")
                .map_err(|e| CodegenError::from(format!("Failed to build add: {e}")))?.into()))
        }
        "I8Sub" | "I16Sub" | "I32Sub" | "I64Sub" | "U8Sub" | "U16Sub" | "U32Sub" | "U64Sub" => {
            let lhs = compiled_args[0].into_int_value();
            let rhs = compiled_args[1].into_int_value();
            Ok(Some(self.builder.build_int_sub(lhs, rhs, "sub")
                .map_err(|e| CodegenError::from(format!("Failed to build sub: {e}")))?.into()))
        }
        other => Err(CodegenError::from(format!("Unknown intrinsic: {other}"))),
    }
}
```

Move the rest of the existing intrinsic match arms from `compile_intrinsic` into `compile_intrinsic_values` in the same style as the `Add` and `Sub` arms above. Make `compile_intrinsic` compile HIR args, then call `compile_intrinsic_values`.

- [ ] **Step 4: Implement rvalue lowering**

In `rvalue.rs`, implement `compile_mir_rvalue` for:

- `Use(operand)` via `compile_mir_operand`.
- `BinaryOp(op, lhs, rhs)` via `compile_binop` with the lhs local type.
- `UnaryOp(op, operand)` via `compile_unaryop`.
- `Cast(operand, target_ty)` using existing cast rules for primitive int/float/pointer cases.
- `Discriminant(place)` by loading the enum struct and extracting field `0`.

- [ ] **Step 5: Implement MIR assertions**

In `assert.rs`, implement `StatementKind::Assert(MirAssertKind::BoundsCheck)` by lowering base/index operands and reusing `emit_index_bounds_check` with the slice/array length. For array places, get length from the type. For slice/fat refs, extract the len field with existing slice helpers.

- [ ] **Step 6: Verify primitive MIR lowering**

Run: `cargo test -p rock-lib codegen::mir -- --nocapture && cargo test -p rock-lib intrinsics -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && cargo test -p rock-lib codegen::intrinsics -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir lib/src/codegen/intrinsics.rs
git commit -m "lower primitive mir rvalues"
```

## Task 7: Lower MIR Control Flow Terminators

**Files:**
- Create: `lib/src/codegen/mir/terminator.rs`
- Modify: `lib/src/codegen/mir/mod.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing control-flow tests**

Add tests in `codegen::mir` for:

- `Goto` emits `br label` to the target block.
- `SwitchInt` emits a conditional branch chain that tests each explicit target before branching to the otherwise block.
- `Drop` branches to its target and does not emit HIR code.

Use IR assertions:

```rust
assert!(ir.contains("switch_eq"));
assert!(ir.contains("br i1"));
assert!(ir.contains("br label"));
```

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_control_flow -- --nocapture`

Expected: FAIL because MIR terminator lowering only handles return.

- [ ] **Step 3: Implement terminator lowering**

In `terminator.rs`, implement:

```rust
pub(crate) fn compile_mir_terminator(
    &mut self,
    function: &MirFunction,
    ctx: &mut MirFunctionContext<'ctx>,
    terminator: &Terminator,
) -> Result<(), CodegenError> {
    match terminator {
        Terminator::Return => self.compile_mir_return(function, ctx),
        Terminator::Goto(target) => self.builder.build_unconditional_branch(ctx.blocks[target.0])
            .map_err(|e| CodegenError::from(format!("Failed to build MIR goto: {e}"))).map(|_| ()),
        Terminator::SwitchInt { discr, targets, otherwise } => {
            let discr_value = self.compile_mir_operand(function, ctx, discr)?.into_int_value();
            let function_value = ctx.function;
            let mut current_test = self.builder.get_insert_block().ok_or_else(|| {
                CodegenError::from("missing insertion block for MIR switch")
            })?;

            for (index, (value, target)) in targets.iter().enumerate() {
                self.builder.position_at_end(current_test);
                let next_test = self.context.append_basic_block(
                    function_value,
                    &format!("switch_next_{index}"),
                );
                let condition = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    discr_value,
                    discr_value.get_type().const_int(*value as u64, true),
                    "switch_eq",
                ).map_err(|e| CodegenError::from(format!("Failed to build MIR switch compare: {e}")))?;
                self.builder.build_conditional_branch(condition, ctx.blocks[target.0], next_test)
                    .map_err(|e| CodegenError::from(format!("Failed to build MIR switch branch: {e}")))?;
                current_test = next_test;
            }

            self.builder.position_at_end(current_test);
            self.builder.build_unconditional_branch(ctx.blocks[otherwise.0])
                .map_err(|e| CodegenError::from(format!("Failed to build MIR switch default: {e}")))?;
            Ok(())
        }
        Terminator::Drop { target, .. } => self.builder.build_unconditional_branch(ctx.blocks[target.0])
            .map_err(|e| CodegenError::from(format!("Failed to build MIR drop branch: {e}"))).map(|_| ()),
        Terminator::Call { .. } => self.compile_mir_call_terminator(function, ctx, terminator),
    }
}
```

For `SwitchInt`, lower `discr` with `compile_mir_operand`, create one integer comparison and conditional branch per explicit `(value, block)` target, then add an unconditional branch to `otherwise`.

- [ ] **Step 4: Verify control flow**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_control_flow -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir
git commit -m "lower mir control flow terminators"
```

## Task 8: Lower MIR Calls And Canonical Callables

**Files:**
- Modify: `lib/src/codegen/mir/operand.rs`
- Modify: `lib/src/codegen/mir/terminator.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing canonical call tests**

Add tests for:

- `Constant::Callable(MirCallable::Instance(id))` materializes a callable using `instance_symbols_by_id`.
- `Terminator::Call` to `Operand::Constant(Constant::Callable(MirCallable::Instance(id)))` emits a direct call to the backend symbol and stores into the destination.
- Unknown `MirCallable::Instance` returns `CodegenError` containing `Unknown MIR callable`.

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_canonical_calls -- --nocapture`

Expected: FAIL because callable operand and call terminator lowering are incomplete.

- [ ] **Step 3: Add canonical callable resolution**

In `CodeGen`, add:

```rust
fn mir_callable_symbol(&self, callable: &crate::mir::MirCallable) -> Result<String, CodegenError> {
    match callable {
        crate::mir::MirCallable::Function(id) => self.function_symbols_by_id
            .get(id)
            .cloned()
            .ok_or_else(|| CodegenError::from(format!("Unknown MIR function callable {:?}", id))),
        crate::mir::MirCallable::Extern(id) => self.extern_symbols_by_id
            .get(id)
            .cloned()
            .ok_or_else(|| CodegenError::from(format!("Unknown MIR extern callable {:?}", id))),
        crate::mir::MirCallable::Instance(id) => self.instance_symbols_by_id
            .get(id)
            .cloned()
            .ok_or_else(|| CodegenError::from(format!("Unknown MIR instance callable {:?}", id))),
        crate::mir::MirCallable::Method { instance: Some(id), .. } => self.instance_symbols_by_id
            .get(id)
            .cloned()
            .ok_or_else(|| CodegenError::from(format!("Unknown MIR method instance callable {:?}", id))),
        other => Err(CodegenError::from(format!("Unsupported MIR callable {:?}", other))),
    }
}
```

Do not search HIR method names or receiver type names in this helper.

- [ ] **Step 4: Implement callable operands and call terminators**

For callable constants, use `materialize_named_callable` with the resolved backend symbol when producing a first-class function value. For `Terminator::Call`, if the callable operand is a direct `Constant::Callable`, emit a direct `build_call` to the resolved LLVM function. For indirect callable values, extract code/env and use `build_indirect_call` with `callable_code_type`.

Store non-unit call results into `destination`, then branch to `target`.

- [ ] **Step 5: Verify canonical calls**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_canonical_calls -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && cargo test -p rock-lib codegen -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/mir
git commit -m "lower mir calls through canonical callables"
```

## Task 9: Lower MIR Aggregates, Fields, Indexes, And Enum Downcasts

**Files:**
- Modify: `lib/src/codegen/mir/place.rs`
- Modify: `lib/src/codegen/mir/rvalue.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing aggregate/projection tests**

Add tests that construct MIR directly and assert LLVM IR contains expected `insertvalue`, `extractvalue`, and `getelementptr` patterns for:

- Tuple aggregate and field projection.
- Struct aggregate using `AggregateKind::Struct { id, display_name }`.
- Enum aggregate using `AggregateKind::EnumVariant` with tag insertion.
- `Rvalue::Discriminant` extracting field `0`.
- `Projection::Downcast(variant_id)` followed by field projection for payload access.

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_aggregates_and_projections -- --nocapture`

Expected: FAIL because aggregate and projection lowering are incomplete.

- [ ] **Step 3: Implement aggregate lowering**

In `rvalue.rs`, implement `Rvalue::Aggregate` for:

- `Tuple`: build a struct value from operand values.
- `Array`: build an array value from operand values.
- `Struct`: use `struct_names_by_id`, `struct_info`, and type args from the destination type to compute field LLVM types and insert operands by index.
- `EnumVariant`: use `enum_names_by_id`, `enum_layout_types`, insert tag at field `0`, and insert payload at `variant_index + 1`.

- [ ] **Step 4: Implement projection lowering**

In `place.rs`, complete:

- `Projection::Field`: use `build_struct_gep` with current aggregate type.
- `Projection::Index`: handle arrays and fat slices; use existing bounds assertion behavior from MIR `Assert`, not an extra hidden check.
- `Projection::Downcast`: record the active variant for the next field projection by changing the current type to the selected payload type.

- [ ] **Step 5: Verify aggregate/projection lowering**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_aggregates_and_projections -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run behavior filters and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && cargo test -p rock-lib --test integration test_enum -- --nocapture && cargo test -p rock-lib --test integration test_array -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir
git commit -m "lower mir aggregates and projections"
```

## Task 10: Lower MIR References, Fat Slices, Casts, And Runtime Checks

**Files:**
- Modify: `lib/src/codegen/mir/place.rs`
- Modify: `lib/src/codegen/mir/rvalue.rs`
- Modify: `lib/src/codegen/mir/assert.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing reference/slice tests**

Add tests for:

- `Rvalue::Ref` of a scalar local emits a thin pointer.
- `Rvalue::Ref` of an array local to a slice reference emits the fat `{ ptr, len }` representation.
- Pointer cast preserves current pointer/int behavior.
- Bounds check assertion traps with `exit(1)` path.

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_references_and_checks -- --nocapture`

Expected: FAIL because fat reference/slice/cast details are incomplete.

- [ ] **Step 3: Implement references and casts**

In `rvalue.rs`, implement `Rvalue::Ref` by computing a place address. If destination type is a fat reference to slice/str, build the fat slice value using `make_fat_slice_value` and the known array length or existing slice length. Otherwise return the pointer.

Implement `Rvalue::Cast` by matching the existing HIR cast behavior for:

- integer width changes,
- integer/float conversions,
- pointer/integer casts,
- array reference to slice reference,
- reference to raw pointer.

- [ ] **Step 4: Complete assertion length extraction**

In `assert.rs`, make bounds checks work for arrays, slices, strings, references to arrays, references to slices, and raw slice pointers using the same layout rules as current codegen.

- [ ] **Step 5: Verify reference and check lowering**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_references_and_checks -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run behavior filters and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && cargo test -p rock-lib --test integration test_vec_index -- --nocapture && cargo test -p rock-lib --test integration test_array -- --nocapture && cargo test -p rock-lib --test integration test_str -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir
git commit -m "lower mir references and runtime checks"
```

## Task 11: Lower MIR Closures Without HIR Lambda Body Fallback

**Files:**
- Modify: `lib/src/codegen/mir/rvalue.rs`
- Modify: `lib/src/codegen/mir/operand.rs`
- Modify: `lib/src/codegen/mir/mod.rs`
- Modify: `lib/src/codegen/closures.rs`
- Test: `lib/src/codegen/mir/mod.rs`

- [ ] **Step 1: Write failing MIR closure codegen test**

Add a test that builds a `MirProgram` with a parent function assigning `Rvalue::Closure(MirClosure { id, captures })` and a `MirFunctionId::Closure(Box::new(id.clone()))` body. Assert generated IR contains the closure function symbol and a callable value insertion.

Use assertions:

```rust
assert!(ir.contains("__mir_closure"));
assert!(ir.contains("callable_code"));
assert!(!ir.contains("__lambda_"));
```

- [ ] **Step 2: Run red test**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_closure_values_from_mir_bodies -- --nocapture`

Expected: FAIL because `Rvalue::Closure` is not lowered from MIR closure bodies.

- [ ] **Step 3: Add MIR closure symbol registration**

When declaring MIR functions, assign closure MIR functions a stable symbol such as:

```rust
format!("__mir_closure_{}_{}", parent_symbol, closure_id.local_index)
```

Store it in `mir_function_symbols` keyed by `MirFunctionId::Closure(Box::new(closure_id.clone()))`.

- [ ] **Step 4: Implement closure value lowering**

In `rvalue.rs`, lower `Rvalue::Closure` by:

- Resolving `MirFunctionId::Closure(Box::new(closure.id.clone()))` to an LLVM function.
- Loading each capture place into an environment struct.
- Allocating the environment with `malloc` when captures are non-empty.
- Returning `build_callable_value(code_ptr, env_ptr)`.

For closure MIR function bodies, bind parameter `0` as env pointer internally and user parameters after it according to `callable_code_type`. Captured values should be loaded from the env into locals matching `MirFunction.closure_captures`.

- [ ] **Step 5: Verify MIR closure lowering**

Run: `cargo test -p rock-lib codegen::mir::tests::mir_codegen_lowers_closure_values_from_mir_bodies -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run closure behavior verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::mir -- --nocapture && cargo test -p rock-lib --test integration test_closure -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir lib/src/codegen/closures.rs
git commit -m "lower mir closures to callable values"
```

## Task 12: Wire Compile Pipeline To MIR-Backed Codegen

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing pipeline characterization test**

Add a codegen unit test that compiles a `MonomorphizedProgram` and a matching `MirProgram` through a new entrypoint:

```rust
#[test]
fn compile_mir_program_uses_mir_body_not_hir_body() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let hir_body = expr(HirExprKind::IntLiteral(1), Type::I64);
    let program = program_with_expr(hir_body);
    let function = function_with_expr(expr(HirExprKind::IntLiteral(1), Type::I64));
    let instance_id = InstanceId(0);
    let mut instances = BTreeMap::new();
    instances.insert(instance_id, InstanceRecord {
        id: instance_id,
        origin: InstanceOrigin::Function(function.id),
        substitution: Vec::new(),
        source_name: "main".to_string(),
        backend_symbol: "main".to_string(),
        declared: None,
        body: Some(function),
        provided_by_object: false,
        is_specialization: false,
    });
    let mono = MonomorphizedProgram { program, instances };
    let mir_id = crate::mir::MirFunctionId::Instance(instance_id);
    let mir_function = crate::mir::MirFunction {
        id: mir_id,
        name: "main".to_string(),
        basic_blocks: vec![crate::mir::BasicBlock {
            statements: vec![crate::mir::StatementData::assign(
                crate::mir::Place { local: crate::mir::Local(0), projection: vec![] },
                crate::mir::Rvalue::Use(crate::mir::Operand::Constant(crate::mir::Constant::Int(99))),
                None,
            )],
            terminator: Some(crate::mir::Terminator::Return),
        }],
        local_decls: vec![crate::mir::LocalDecl {
            ty: Type::I64,
            mutability: crate::mir::Mutability::Mut,
            name: Some("return_place".to_string()),
            span: None,
        }],
        closure_captures: Vec::new(),
        arg_count: 0,
        ret_type: Type::I64,
    };
    let mir = crate::mir::MirProgram { functions: BTreeMap::from([(mir_id, mir_function)]) };

    codegen.compile_program_from_mir(&mono, &mir).unwrap();

    let ir = codegen.get_ir();
    assert!(ir.contains("ret i32 99"));
    assert!(!ir.contains("ret i32 1"));
}
```

- [ ] **Step 2: Run red test**

Run: `cargo test -p rock-lib compile_mir_program_uses_mir_body_not_hir_body -- --nocapture`

Expected: FAIL because `compile_program_from_mir` does not exist.

- [ ] **Step 3: Add MIR compile entrypoint**

In `CodeGen`, add:

```rust
pub fn compile_program_from_mir(
    &mut self,
    program: &MonomorphizedProgram,
    mir: &crate::mir::MirProgram,
) -> Result<(), CodegenError> {
    self.prepare_program_declarations(program)?;
    self.declare_mir_functions(program, mir)?;
    self.compile_mir_program(mir)
}
```

`declare_mir_functions` must declare every non-object instance MIR body with `record.backend_symbol`, plus closure MIR functions with stable closure symbols.

- [ ] **Step 4: Update compile pipeline**

In `lib/src/lib.rs`, change the pipeline order to:

```rust
let mut monomorphized = mono::monomorphize_with_crates(hir, crate_ctx);
let _dce_report = dce::prune_unreachable_instances(&mut monomorphized);

let mir_program = mir::builder::MirBuilder::build_monomorphized(&monomorphized);
if config.has_debug_print(DebugPrint::Mir) {
    println!("{:#?}", mir_program);
}
if let Err(diagnostics) = mir::borrowck::BorrowChecker::run(&mir_program) {
    return Err(diagnostics);
}
let agreement = mir::agreement::check_mir_runtime_agreement(&mir_program);
if !agreement.is_clean() {
    let mut diagnostics = Diagnostics::default();
    diagnostics.push(diagnostic::Diagnostic::new(
        format!("MIR/codegen agreement failed: {:?}", agreement),
        Span::default(),
    ));
    return Err(diagnostics);
}
```

Then call:

```rust
codegen.compile_program_from_mir(&monomorphized, &mir_program)
```

Remove the earlier pre-monomorphized MIR build from the active path unless it is explicitly retained as a diagnostic-only pass with a comment.

- [ ] **Step 5: Verify MIR pipeline unit test**

Run: `cargo test -p rock-lib compile_mir_program_uses_mir_body_not_hir_body -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run initial integration smoke tests**

Run serially:

```bash
cargo test -p rock-lib --test integration test_hello_world -- --exact --nocapture
cargo test -p rock-lib --test integration test_arithmetic -- --exact --nocapture
cargo test -p rock-lib --test integration test_functions -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit pipeline switch**

Run: `cargo fmt --all --check && cargo test -p rock-lib compile_mir_program_uses_mir_body_not_hir_body -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/lib.rs lib/src/codegen/mod.rs lib/src/codegen/mir
git commit -m "wire codegen to mir program bodies"
```

## Task 13: Remove Active HIR Body Codegen From Compile Path

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/stmt.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Inspect: `lib/src/codegen/control_flow.rs` to confirm it is not called from the active MIR path.
- Test: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Write source-path audit test**

Add a test in `lib/src/codegen/mod.rs` that asserts `compile_program_from_mir` succeeds when a HIR body contains a value different from MIR, using the Task 12 test. Keep it as the active regression.

- [ ] **Step 2: Run audit searches**

Run:

```bash
rg "compile_function\(|compile_block\(|compile_stmt\(|compile_expr\(|compile_if\(|compile_match\(|compile_while\(|compile_for\(|compile_loop\(" lib/src/lib.rs lib/src/codegen --glob '*.rs'
```

Expected before cleanup: hits in HIR body modules and tests. The active compile path must not call these from `compile_impl` or `CodeGen::compile_program_from_mir`.

- [ ] **Step 3: Stop using HIR body compile entrypoint**

Keep `compile_program` only for tests during this task, or mark it `#[cfg(test)]` if production no longer uses it. Ensure `compile_impl` calls only `compile_program_from_mir`.

If any production helper calls `compile_expr`, `compile_stmt`, `compile_block`, or `compile_function` during MIR codegen, replace it with the corresponding MIR helper.

- [ ] **Step 4: Run audit searches after cleanup**

Run:

```bash
rg "compile_program\(&monomorphized\)|compile_function\(|compile_block\(|compile_stmt\(|compile_expr\(" lib/src/lib.rs lib/src/codegen --glob '*.rs'
```

Expected remaining production hits: HIR helper definitions and HIR-only unit tests are acceptable only if they are not reachable from `compile_impl` or `compile_program_from_mir`. No hit may show `compile_impl` calling the HIR body path.

- [ ] **Step 5: Run focused verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen lib/src/lib.rs
git commit -m "remove hir body codegen from active path"
```

## Task 14: Restore Full Runtime Parity Across Feature Areas

**Files:**
- Modify: `lib/src/codegen/mir/mod.rs`
- Modify: `lib/src/codegen/mir/place.rs`
- Modify: `lib/src/codegen/mir/operand.rs`
- Modify: `lib/src/codegen/mir/rvalue.rs`
- Modify: `lib/src/codegen/mir/terminator.rs`
- Modify: `lib/src/codegen/mir/assert.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Run broad behavior filters**

Run these commands serially and save failures for targeted TDD fixes:

```bash
cargo test -p rock-lib --test integration test_hello_world -- --exact --nocapture
cargo test -p rock-lib --test integration test_arithmetic -- --nocapture
cargo test -p rock-lib --test integration test_functions -- --nocapture
cargo test -p rock-lib --test integration test_generic -- --nocapture
cargo test -p rock-lib --test integration test_trait -- --nocapture
cargo test -p rock-lib --test integration test_enum -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_ptr -- --nocapture
cargo test -p rock-lib --test integration test_stdlib -- --nocapture
```

Expected: Any failures become focused TDD fixes in this task. Do not skip failures or mark them expected unless the spec is updated and approved.

- [ ] **Step 2: For each failing feature, write a focused regression**

For each failure, add or identify a focused integration test with `--exact` that reproduces the runtime mismatch. Run it to confirm failure before changing production code.

- [ ] **Step 3: Apply targeted MIR backend fixes**

Fix only the MIR backend or MIR builder behavior responsible for the failing feature. Do not reintroduce HIR body lowering as a fallback.

- [ ] **Step 4: Verify each fixed regression**

Run each focused failing command again with `--exact --nocapture`.

Expected: PASS for each fixed regression.

- [ ] **Step 5: Run full behavior filters and commit**

Run the full command list from Step 1 again.

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mir lib/src/mir/builder lib/tests/integration.rs
git commit -m "restore mir codegen runtime parity"
```

## Task 15: Artifact, Product, And Link Metadata Verification

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/lib.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Run artifact-focused tests**

Run:

```bash
cargo test -p rock-lib compile_with_products -- --nocapture
cargo test -p rock-lib product_link -- --nocapture
cargo test -p rock-lib --test integration test_generic_function_argument_app_ir_does_not_define_stdlib_object_symbols -- --exact --nocapture
cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_current_function -- --exact --nocapture
cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_generic_specialization -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 2: Fix any product/link regressions with TDD**

If a product/link test fails, write a focused test in `lib/src/lib.rs` or reuse the exact integration failure. Fix product/link metadata without changing artifact format unless an existing test requires it.

- [ ] **Step 3: Verify artifact and link behavior**

Run the Step 1 commands again.

Expected: PASS.

- [ ] **Step 4: Commit product/link fixes when files changed**

If files changed, commit:

```bash
git add lib/src/lib.rs lib/src/codegen/mod.rs lib/tests/integration.rs
git commit -m "preserve products for mir codegen"
```

If no files changed, do not create an empty commit.

## Task 16: Final Task 21 Audit And Verification

**Files:**
- Modify: `lib/src/codegen/mod.rs` when final review finds declaration or pipeline issues.
- Modify: `lib/src/codegen/mir/mod.rs` when final review finds MIR function lowering issues.
- Modify: `lib/src/codegen/mir/place.rs` when final review finds MIR place lowering issues.
- Modify: `lib/src/codegen/mir/operand.rs` when final review finds MIR operand lowering issues.
- Modify: `lib/src/codegen/mir/rvalue.rs` when final review finds MIR rvalue lowering issues.
- Modify: `lib/src/codegen/mir/terminator.rs` when final review finds MIR terminator lowering issues.
- Modify: `lib/src/codegen/mir/assert.rs` when final review finds MIR assertion lowering issues.
- Modify: `lib/src/mir/builder/mod.rs` when final review finds MIR construction issues.
- Modify: `lib/src/mir/builder/expr.rs` when final review finds expression-to-MIR construction issues.
- Modify: `lib/src/lib.rs` when final review finds pipeline issues.
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Audit active compile path for HIR body fallback**

Run:

```bash
rg "compile_program\(&monomorphized\)|compile_function\(|compile_block\(|compile_stmt\(|compile_expr\(|compile_if\(|compile_match\(|compile_while\(|compile_for\(|compile_loop\(" lib/src/lib.rs lib/src/codegen --glob '*.rs'
```

Expected: no production call chain from `compile_impl` or `compile_program_from_mir` to HIR body lowering. HIR body helper definitions and HIR-only tests may remain only if unreachable from the active compile path.

- [ ] **Step 2: Audit MIR backend coverage for active forms**

Run:

```bash
rg "Unsupported MIR|todo!\(|unimplemented!\(|panic!\(\".*MIR" lib/src/codegen/mir lib/src/mir/builder --glob '*.rs'
```

Expected: no unsupported active MIR forms in codegen. Explicit internal errors returning `CodegenError` are acceptable.

- [ ] **Step 3: Run focused MIR/codegen verification**

Run serially:

```bash
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib mir::borrowck -- --nocapture
cargo test -p rock-lib codegen -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Run behavior filters**

Run serially:

```bash
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo test -p rock-lib --test integration test_enum -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
cargo test -p rock-lib --test integration test_generic -- --nocapture
cargo test -p rock-lib --test integration test_stdlib -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run serially:

```bash
cargo fmt --all --check
cargo test -p rock-lib
```

Expected: PASS.

- [ ] **Step 6: Request final code review**

Use the requesting-code-review skill with this context:

```text
Description: Finished roadmap Task 21 by moving executable LLVM body emission from HIR expression/statement trees to canonical MIR while preserving current runtime behavior, ABI, products, and linking.

Spec: docs/superpowers/specs/2026-05-23-mir-backed-codegen-design.md
Plan: docs/superpowers/plans/2026-05-23-mir-backed-codegen.md

Review focus:
- Active compile path builds monomorphized MIR before codegen.
- Borrowck and MIR agreement run on the executable MIR used for codegen.
- LLVM bodies are emitted from MIR statements and terminators.
- No active HIR expression/statement/block/control-flow body fallback remains.
- Calls, methods, intrinsics, closures, aggregates, enum matches, bounds checks, casts, and drops are emitted from canonical MIR facts.
- HIR/mono remain metadata inputs only for layouts, symbols, instances, products, externs, and linking.
- Product artifact and object-link behavior are preserved.
```

Expected: reviewer returns APPROVED or findings. Fix Critical and Important findings before continuing.

- [ ] **Step 7: Update roadmap/audit docs after proof**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, under `### Task 21: Move Codegen To Consume MIR`, add:

```markdown

**Status:** Complete in `docs/superpowers/plans/2026-05-23-mir-backed-codegen.md`; executable LLVM body emission now consumes monomorphized MIR, with HIR/mono retained only for metadata, symbols, products, externs, and linking.
```

In `docs/superpowers/plans/master-audit-checklist.md`, add a corresponding Task 21 completion note near the compiler architecture roadmap section.

- [ ] **Step 8: Commit final cleanup/docs when files changed**

If review fixes or doc updates changed files, commit:

```bash
git add lib/src/codegen lib/src/mir lib/src/lib.rs lib/tests/integration.rs docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "finish mir backed codegen cutover"
```

If there are no uncommitted changes after prior task commits, do not create an empty commit.

## Completion Check

Task 21 is complete only when:

- `cargo test -p rock-lib mir::builder -- --nocapture` passes.
- `cargo test -p rock-lib mir::agreement -- --nocapture` passes.
- `cargo test -p rock-lib mir::borrowck -- --nocapture` passes.
- `cargo test -p rock-lib codegen -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_borrow_ -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_closure -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_enum -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_array -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_vec_index -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_generic -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_stdlib -- --nocapture` passes.
- `cargo test -p rock-lib` passes.
- `cargo fmt --all --check` passes.
- `git diff --check` passes.
- Final code review has no Critical or Important findings.
- The active compile path has no HIR body fallback for executable function bodies.
