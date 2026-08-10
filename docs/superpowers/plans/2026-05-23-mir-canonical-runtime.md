# MIR Canonical Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make MIR carry canonical identities and explicit runtime semantics required for a future MIR codegen backend while preserving the current HIR-based codegen path.

**Architecture:** Add an identity spine first, then migrate MIR builder output from string/unit placeholders to canonical callable, aggregate, enum, closure, cast, drop, and runtime-check forms. Keep readable names as debug metadata only. Add MIR/codegen agreement scaffolding without switching codegen to MIR.

**Tech Stack:** Rust 2021, `rock-lib`, existing HIR/MIR/mono IDs (`DefId`, `FieldId`, `VariantId`, `InstanceId`), MIR builder and borrowck tests, integration tests, `cargo fmt --all --check`, `cargo test -p rock-lib`.

---

## Approved Spec

- `docs/superpowers/specs/2026-05-23-mir-canonical-runtime-design.md`

## File Structure

- Create `lib/src/mir/identity.rs`: canonical MIR identity, callable, aggregate, closure, runtime helper, and assertion data types.
- Modify `lib/src/mir/mod.rs`: register identity module, re-export identity types, update MIR core enums/structs, add accessors and helpers.
- Modify `lib/src/mir/builder/mod.rs`: build canonical MIR functions, preserve display names, add builder helpers for callable/aggregate/match/runtime lowering.
- Modify `lib/src/mir/builder/expr.rs`: lower calls, methods, structs, enum variants, matches, closures, casts, and runtime checks to the new MIR forms.
- Modify `lib/src/mir/builder/blocks.rs`: keep drop insertion compatible with new runtime/drop forms.
- Modify `lib/src/mir/borrowck/**` and `lib/src/mir/dataflow/**`: handle new MIR statement/terminator/rvalue forms conservatively without changing borrowck semantics.
- Add `lib/src/mir/agreement.rs`: focused MIR shape checks that compare MIR runtime requirements with existing HIR/codegen-owned behavior.
- Modify `lib/src/mir/mod.rs` tests and `lib/src/mir/builder/mod.rs` tests for unit-level MIR shape coverage.
- Modify `lib/tests/integration.rs` only for behavior/diagnostic regressions that cannot be covered at MIR unit level.

## Task 1: Add MIR Identity Data Types

**Files:**
- Create: `lib/src/mir/identity.rs`
- Modify: `lib/src/mir/mod.rs`
- Test: `lib/src/mir/identity.rs`

- [ ] **Step 1: Write failing identity tests**

Create `lib/src/mir/identity.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ids::{CrateId, DefId, FieldId, InstanceId, LocalDefId, VariantId};

    #[test]
    fn mir_function_identity_is_canonical_not_display_name() {
        let id = DefId::new(CrateId(0), LocalDefId(1));
        let left = MirFunctionId::Function(id);
        let right = MirFunctionId::Function(id);

        assert_eq!(left, right);
        assert_eq!(left.display_fallback("alias"), "def#0:1");
    }

    #[test]
    fn callable_and_aggregate_identities_carry_existing_compiler_ids() {
        let function = DefId::new(CrateId(0), LocalDefId(2));
        let structure = DefId::new(CrateId(0), LocalDefId(3));
        let enumeration = DefId::new(CrateId(0), LocalDefId(4));
        let variant = VariantId(5);

        assert_eq!(
            MirCallable::Function(function).function_id(),
            Some(MirFunctionId::Function(function))
        );
        assert_eq!(
            MirCallable::Instance(InstanceId(7)).function_id(),
            Some(MirFunctionId::Instance(InstanceId(7)))
        );
        assert_eq!(
            MirAggregateIdentity::Struct {
                id: structure,
                display_name: "Point".to_string(),
            }
            .def_id(),
            structure
        );
        assert_eq!(
            MirAggregateIdentity::EnumVariant {
                enum_id: enumeration,
                variant_id: variant,
                enum_name: "Option".to_string(),
                variant_name: "Some".to_string(),
            }
            .def_id(),
            enumeration
        );
        assert_eq!(MirFieldIdentity::new(structure, FieldId(1)).field_id, FieldId(1));
    }
}
```

- [ ] **Step 2: Register the module and verify the tests fail**

In `lib/src/mir/mod.rs`, add the module registration near the other MIR modules:

```rust
pub mod identity;
```

Run: `cargo test -p rock-lib mir::identity -- --nocapture`

Expected: FAIL to compile with unresolved `MirFunctionId`, `MirCallable`, `MirAggregateIdentity`, and `MirFieldIdentity`.

- [ ] **Step 3: Implement identity types**

Insert this implementation above the tests in `lib/src/mir/identity.rs`:

```rust
use crate::ids::{DefId, FieldId, InstanceId, VariantId};
use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MirFunctionId {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
}

impl MirFunctionId {
    pub fn display_fallback(self, _fallback: &str) -> String {
        match self {
            MirFunctionId::Function(id) => format!("def#{}:{}", id.crate_id.0, id.local.0),
            MirFunctionId::Extern(id) => format!("extern#{}:{}", id.crate_id.0, id.local.0),
            MirFunctionId::Instance(id) => format!("instance#{}", id.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirClosureId {
    pub owner: MirFunctionId,
    pub local_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MirRuntimeHelper {
    BoundsCheck,
    PanicBounds,
    DropGlue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirCallable {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
    Method {
        impl_id: Option<DefId>,
        trait_id: Option<DefId>,
        trait_args: Vec<Type>,
        method_id: DefId,
        instance: Option<InstanceId>,
        display_name: String,
    },
    Closure(MirClosureId),
    Intrinsic(String),
    RuntimeHelper(MirRuntimeHelper),
}

impl MirCallable {
    pub fn function_id(&self) -> Option<MirFunctionId> {
        match self {
            MirCallable::Function(id) => Some(MirFunctionId::Function(*id)),
            MirCallable::Extern(id) => Some(MirFunctionId::Extern(*id)),
            MirCallable::Instance(id) => Some(MirFunctionId::Instance(*id)),
            MirCallable::Method { .. }
            | MirCallable::Closure(_)
            | MirCallable::Intrinsic(_)
            | MirCallable::RuntimeHelper(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirAggregateIdentity {
    Struct {
        id: DefId,
        display_name: String,
    },
    EnumVariant {
        enum_id: DefId,
        variant_id: VariantId,
        enum_name: String,
        variant_name: String,
    },
}

impl MirAggregateIdentity {
    pub fn def_id(&self) -> DefId {
        match self {
            MirAggregateIdentity::Struct { id, .. } => *id,
            MirAggregateIdentity::EnumVariant { enum_id, .. } => *enum_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirFieldIdentity {
    pub owner: DefId,
    pub field_id: FieldId,
}

impl MirFieldIdentity {
    pub fn new(owner: DefId, field_id: FieldId) -> Self {
        Self { owner, field_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirAssertKind {
    BoundsCheck,
    NonNull,
}
```

- [ ] **Step 4: Re-export identity types**

In `lib/src/mir/mod.rs`, add this after the `use` statements:

```rust
pub use identity::{
    MirAggregateIdentity, MirAssertKind, MirCallable, MirClosureId, MirFieldIdentity,
    MirFunctionId, MirRuntimeHelper,
};
```

- [ ] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib mir::identity -- --nocapture`

Expected: PASS with the two new identity tests.

- [ ] **Step 6: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::identity -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 7: Commit Task 1**

Run:

```bash
git add lib/src/mir/identity.rs lib/src/mir/mod.rs
git commit -m "add canonical mir identity types"
```

## Task 2: Re-Key MIR Functions By Canonical Identity

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Update the existing function identity test first**

In `lib/src/mir/builder/mod.rs`, replace the body of `build_uses_canonical_function_indexes_once_per_def_id` with:

```rust
let id = DefId::new(CrateId(0), LocalDefId(0));
let program = HirProgram::from_parts_with_canonical_names(
    std::collections::HashMap::from([
        ("alias_main".to_string(), empty_function(id, "alias_main")),
        ("main".to_string(), empty_function(id, "main")),
    ]),
    std::collections::HashMap::new(),
    std::collections::HashMap::new(),
    std::collections::HashMap::new(),
    vec![],
    vec![],
    &std::collections::HashMap::from([(id, "main".to_string())]),
);

let mir = MirBuilder::build(&program);
let mir_id = crate::mir::MirFunctionId::Function(id);

assert_eq!(mir.functions.len(), 1);
assert!(mir.functions.contains_key(&mir_id));
assert_eq!(mir.functions[&mir_id].name, "main");
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p rock-lib build_uses_canonical_function_indexes_once_per_def_id -- --nocapture`

Expected: FAIL to compile because `MirProgram.functions` is still keyed by `String` and `MirFunction` has no canonical ID.

- [ ] **Step 3: Change `MirProgram` and `MirFunction` shape**

In `lib/src/mir/mod.rs`, replace the `HashMap` import with `BTreeMap`:

```rust
use std::collections::BTreeMap;
```

Replace `MirProgram` and `MirFunction` definitions with:

```rust
#[derive(Debug, Clone)]
pub struct MirProgram {
    pub functions: BTreeMap<MirFunctionId, MirFunction>,
}

impl MirProgram {
    pub fn functions(&self) -> impl Iterator<Item = (&MirFunctionId, &MirFunction)> {
        self.functions.iter()
    }

    pub fn function(&self, id: MirFunctionId) -> Option<&MirFunction> {
        self.functions.get(&id)
    }
}

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub id: MirFunctionId,
    pub name: String,
    pub basic_blocks: Vec<BasicBlock>,
    pub local_decls: Vec<LocalDecl>,
    pub closure_captures: Vec<MirClosureCapture>,
    pub arg_count: usize,
    pub ret_type: Type,
}
```

- [ ] **Step 4: Update builder construction**

In `lib/src/mir/builder/mod.rs`, update the MIR imports to include `MirFunctionId`.

Change `MirBuilder::build` to:

```rust
pub fn build(program: &HirProgram) -> MirProgram {
    let mut functions = std::collections::BTreeMap::new();

    for (id, name, func) in program.functions_by_id() {
        let mut builder = MirBuilder::new(program);
        let mir_id = MirFunctionId::Function(id);
        let mir_func = builder.build_function(mir_id, name, func);
        functions.insert(mir_id, mir_func);
    }

    MirProgram { functions }
}
```

Change `build_function` signature and returned `MirFunction` fields:

```rust
fn build_function(
    &mut self,
    id: MirFunctionId,
    display_name: &str,
    func: &HirFunction,
) -> MirFunction {
```

and:

```rust
MirFunction {
    id,
    name: display_name.to_string(),
    basic_blocks: std::mem::take(&mut self.blocks),
    local_decls: std::mem::take(&mut self.locals),
    closure_captures: std::mem::take(&mut self.closure_captures),
    arg_count: func.params.len(),
    ret_type: func.ret_type.clone(),
}
```

- [ ] **Step 5: Update borrowck iteration**

In `lib/src/mir/borrowck/mod.rs`, replace:

```rust
for func in program.functions.values() {
```

with:

```rust
for (_, func) in program.functions() {
```

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib build_uses_canonical_function_indexes_once_per_def_id -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 8: Commit Task 2**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/borrowck/mod.rs
git commit -m "key mir functions by canonical identity"
```

## Task 3: Lower Function, Extern, And Instance Callables Canonically

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add failing callable tests**

Add these tests to the existing `tests` module in `lib/src/mir/builder/mod.rs`:

```rust
#[test]
fn resolved_function_var_lowers_to_callable_constant() {
    let id = DefId::new(CrateId(0), LocalDefId(11));
    let program = HirProgram::from_parts(
        std::collections::HashMap::from([("make".to_string(), empty_function(id, "make"))]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );
    let mut builder = MirBuilder::new(&program);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::Unit,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });

    let resolved = expr(
        HirExprKind::ResolvedVar(HirVarRef {
            name: "make".to_string(),
            target: HirVarTarget::Function(id),
        }),
        Type::Unit,
    );

    builder.lower_expr(
        &resolved,
        Place {
            local: Local(0),
            projection: vec![],
        },
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::Use(Operand::Constant(crate::mir::Constant::Callable(
                crate::mir::MirCallable::Function(target)
            )))
        ) if *target == id
    )));
}

#[test]
fn resolved_instance_var_lowers_to_callable_constant_without_name_lookup() {
    let program = empty_program();
    let mut builder = MirBuilder::new(&program);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::Unit,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let instance = crate::ids::InstanceId(3);
    let resolved = expr(
        HirExprKind::ResolvedVar(HirVarRef {
            name: "generic".to_string(),
            target: HirVarTarget::Instance(instance),
        }),
        Type::Unit,
    );

    builder.lower_expr(
        &resolved,
        Place {
            local: Local(0),
            projection: vec![],
        },
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::Use(Operand::Constant(crate::mir::Constant::Callable(
                crate::mir::MirCallable::Instance(target)
            )))
        ) if *target == instance
    )));
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib resolved_ -- --nocapture`

Expected: FAIL to compile because `Constant::Callable` is not implemented.

- [ ] **Step 3: Add callable constants**

In `lib/src/mir/mod.rs`, add this variant to `Constant`:

```rust
Callable(MirCallable),
```

- [ ] **Step 4: Add builder callable helper**

In `impl MirBuilder` in `lib/src/mir/builder/mod.rs`, add:

```rust
fn callable_for_var_target(&self, target: &HirVarTarget) -> MirCallable {
    match target {
        HirVarTarget::Function(id) => MirCallable::Function(*id),
        HirVarTarget::Extern(id) => MirCallable::Extern(*id),
        HirVarTarget::Instance(id) => MirCallable::Instance(*id),
    }
}
```

Ensure the imports include `HirVarTarget` and `MirCallable`.

- [ ] **Step 5: Migrate resolved callable lowering without name lookup**

In `lib/src/mir/builder/expr.rs`, delete the `HirExprKind::Var` function/extern fallback entirely:

```rust
} else if self.program.function_by_name(name).is_some() || self.is_extern(name) {
    self.emit_assign(dest, Rvalue::Use(Operand::Constant(Constant::Unit)), span);
}
```

Do not replace it with another `function_by_name`, `externs_by_name`, or `is_extern` semantic lookup. MIR callable identity must come from `HirExprKind::ResolvedVar`; a bare `HirExprKind::Var` remains local-only in MIR builder lowering.

If no production code still calls `MirBuilder::is_extern` after this deletion, remove the helper from `lib/src/mir/builder/mod.rs` in the same task.

Replace the `HirExprKind::ResolvedVar` arm with:

```rust
HirExprKind::ResolvedVar(reference) => {
    let callable = self.callable_for_var_target(&reference.target);
    self.emit_assign(
        dest,
        Rvalue::Use(Operand::Constant(Constant::Callable(callable))),
        span,
    );
}
```

Update the `use crate::mir::{...}` list in `expr.rs` to include `MirCallable`.

- [ ] **Step 6: Update stale test expectation**

The old test `resolved_var_is_unhandled_even_when_local_has_same_name` must now assert callable lowering. Rename it to `resolved_var_ignores_same_named_local_and_uses_canonical_target`, and replace its final assertion with:

```rust
assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
    &stmt.kind,
    StatementKind::Assign(
        _,
        Rvalue::Use(Operand::Constant(crate::mir::Constant::Callable(
            crate::mir::MirCallable::Function(target)
        )))
    ) if *target == DefId::new(CrateId(0), LocalDefId(99))
)));
```

- [ ] **Step 7: Run focused verification**

Run: `cargo test -p rock-lib resolved_ -- --nocapture`

Expected: PASS for the resolved callable tests.

- [ ] **Step 8: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 9: Commit Task 3**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs
git commit -m "lower mir callables with canonical targets"
```

## Task 4: Lower Method Call Targets Canonically

**Files:**
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add failing method callable test**

Add this test to `lib/src/mir/builder/mod.rs` tests:

```rust
#[test]
fn method_call_terminator_uses_selected_method_callable() {
    let program = empty_program();
    let mut builder = builder_with_var(&program, "value", Type::I64);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let impl_id = DefId::new(CrateId(0), LocalDefId(21));
    let trait_id = DefId::new(CrateId(0), LocalDefId(22));
    let method_id = DefId::new(CrateId(0), LocalDefId(23));
    let call = expr(
        HirExprKind::MethodCall(
            Box::new(expr(HirExprKind::Var("value".to_string()), Type::I64)),
            "show".to_string(),
            vec![],
            Some(crate::ast::SelfReceiverMode::Move),
            Some(crate::hir::HirMethodCallTarget {
                impl_id: Some(impl_id),
                trait_id: Some(trait_id),
                trait_args: vec![Type::I64],
                method_id,
                from_index_operator: false,
            }),
        ),
        Type::I64,
    );

    builder.lower_expr(
        &call,
        Place {
            local: Local(1),
            projection: vec![],
        },
    );

    assert!(builder.blocks.iter().any(|block| matches!(
        &block.terminator,
        Some(crate::mir::Terminator::Call {
            func: Operand::Constant(crate::mir::Constant::Callable(crate::mir::MirCallable::Method {
                impl_id: Some(found_impl),
                trait_id: Some(found_trait),
                trait_args,
                method_id: found_method,
                instance,
                display_name,
            })),
            ..
        }) if *found_impl == impl_id
            && *found_trait == trait_id
            && *found_method == method_id
            && trait_args == &vec![Type::I64]
            && instance.is_none()
            && display_name == "show"
    )));
}
```

Add this test to prove the post-monomorphization method-call shape keeps `InstanceId` through the existing callable path:

```rust
#[test]
fn monomorphized_method_call_reuses_instance_callable_target() {
    let program = empty_program();
    let mut builder = MirBuilder::new(&program);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let instance = crate::ids::InstanceId(9);
    let callee = expr(
        HirExprKind::ResolvedVar(HirVarRef {
            name: "Box::unwrap".to_string(),
            target: HirVarTarget::Instance(instance),
        }),
        Type::Function(vec![], Box::new(Type::I64)),
    );
    let call = expr(HirExprKind::Call(Box::new(callee), vec![]), Type::I64);

    builder.lower_expr(
        &call,
        Place {
            local: Local(0),
            projection: vec![],
        },
    );

    assert!(builder.blocks.iter().flat_map(|block| &block.statements).any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::Use(Operand::Constant(crate::mir::Constant::Callable(
                crate::mir::MirCallable::Instance(target)
            )))
        ) if *target == instance
    )));
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p rock-lib method_call_terminator_uses_selected_method_callable -- --nocapture && cargo test -p rock-lib monomorphized_method_call_reuses_instance_callable_target -- --nocapture`

Expected: `method_call_terminator_uses_selected_method_callable` FAILS because method calls still synthesize a unit-valued method local. `monomorphized_method_call_reuses_instance_callable_target` should PASS after Task 3; if it fails, fix the Task 3 `HirVarTarget::Instance` callable path before continuing.

- [ ] **Step 3: Add method callable helper**

In `lib/src/mir/builder/mod.rs`, add this helper in `impl MirBuilder`:

```rust
fn callable_for_method_target(
    &self,
    method_name: &str,
    target: Option<&crate::hir::HirMethodCallTarget>,
) -> MirCallable {
    match target {
        Some(target) => MirCallable::Method {
            impl_id: target.impl_id,
            trait_id: target.trait_id,
            trait_args: target.trait_args.clone(),
            method_id: target.method_id,
            instance: None,
            display_name: method_name.to_string(),
        },
        None => MirCallable::Intrinsic(method_name.to_string()),
    }
}
```

`instance: None` is only for non-monomorphized method-call HIR nodes. Generic method calls that mono rewrites to `HirVarTarget::Instance(instance_id)` must continue to lower through Task 3 as `MirCallable::Instance(instance_id)`.

- [ ] **Step 4: Replace method unit local lowering**

In `lib/src/mir/builder/expr.rs`, change the method-call match arm pattern from:

```rust
HirExprKind::MethodCall(receiver, method_name, args, self_receiver, _) => {
```

to:

```rust
HirExprKind::MethodCall(receiver, method_name, args, self_receiver, target) => {
```

Delete the `method_temp`, `method_place`, and unit assignment block. Replace the `Terminator::Call` `func` field with:

```rust
func: Operand::Constant(Constant::Callable(
    self.callable_for_method_target(method_name, target.as_ref()),
)),
```

Remove `let _ = method_name;` at the end of the method-call arm.

- [ ] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib method_call_terminator_uses_selected_method_callable -- --nocapture && cargo test -p rock-lib monomorphized_method_call_reuses_instance_callable_target -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run method behavior verification**

Run: `cargo test -p rock-lib --test integration test_impl_methods -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 8: Commit Task 4**

Run:

```bash
git add lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs
git commit -m "lower mir method calls with canonical targets"
```

## Task 5: Canonicalize Struct And Enum Aggregates

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/borrowck/**`
- Modify: `lib/src/mir/dataflow/**`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Update struct aggregate test and add enum aggregate/field projection tests**

In `test_lower_struct_literal_orders_operands_by_field_id`, replace the aggregate match with:

```rust
let Some(StatementData {
    kind:
        StatementKind::Assign(
            _,
            Rvalue::Aggregate(
                AggregateKind::Struct {
                    id: struct_id,
                    display_name,
                },
                operands,
            ),
        ),
    ..
}) = builder.blocks[0].statements.last()
else {
    panic!("expected struct aggregate assignment");
};

assert_eq!(*struct_id, owner);
assert_eq!(display_name, "Foo");
```

Add this enum test to `lib/src/mir/builder/mod.rs` tests:

```rust
#[test]
fn enum_variant_lowers_to_canonical_aggregate_with_payloads() {
    let enum_id = DefId::new(CrateId(0), LocalDefId(30));
    let variant_id = crate::ids::VariantId(2);
    let mut enums = std::collections::HashMap::new();
    enums.insert(
        "Option".to_string(),
        crate::hir::HirEnum {
            id: enum_id,
            name: "Option".to_string(),
            generic_params: vec![],
            variants: vec![crate::hir::HirVariant {
                id: variant_id,
                name: "Some".to_string(),
                fields: crate::hir::HirVariantFields::Positional(vec![Type::I64]),
            }],
        },
    );
    let program = HirProgram::from_parts(
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        enums,
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );
    let mut builder = MirBuilder::new(&program);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::Enum {
            id: enum_id,
            args: vec![],
        },
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let expr = expr(
        HirExprKind::EnumVariant(
            "Option".to_string(),
            "Some".to_string(),
            vec![expr(HirExprKind::IntLiteral(9), Type::I64)],
            Some(crate::hir::HirVariantLocation {
                owner: enum_id,
                variant_id,
                name: "Some".to_string(),
            }),
        ),
        Type::Enum {
            id: enum_id,
            args: vec![],
        },
    );

    builder.lower_expr(
        &expr,
        Place {
            local: Local(0),
            projection: vec![],
        },
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::Aggregate(
                AggregateKind::EnumVariant {
                    enum_id: found_enum,
                    variant_id: found_variant,
                    enum_name,
                    variant_name,
                },
                operands,
            ),
        ) if *found_enum == enum_id
            && *found_variant == variant_id
            && enum_name == "Option"
            && variant_name == "Some"
            && operands.len() == 1
    )));
}
```

Add this field projection identity test to the same test module:

```rust
#[test]
fn struct_field_access_projection_carries_field_identity() {
    let struct_id = DefId::new(CrateId(0), LocalDefId(31));
    let field_id = crate::ids::FieldId(4);
    let program = HirProgram::from_parts(
        std::collections::HashMap::new(),
        std::collections::HashMap::from([(
            "Point".to_string(),
            crate::hir::HirStruct {
                id: struct_id,
                name: "Point".to_string(),
                generic_params: vec![],
                fields: vec![crate::hir::HirField {
                    id: field_id,
                    name: "x".to_string(),
                    ty: Type::I64,
                    public: true,
                }],
            },
        )]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );
    let mut builder = builder_with_var(
        &program,
        "p",
        Type::Struct {
            id: struct_id,
            args: vec![],
        },
    );
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let field = expr(
        HirExprKind::FieldAccess(
            Box::new(expr(HirExprKind::Var("p".to_string()), Type::Struct {
                id: struct_id,
                args: vec![],
            })),
            "x".to_string(),
            Some(crate::hir::HirFieldLocation {
                owner: struct_id,
                field_id,
                name: "x".to_string(),
            }),
        ),
        Type::I64,
    );

    builder.lower_expr(
        &field,
        Place {
            local: Local(1),
            projection: vec![],
        },
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::Use(Operand::Copy(Place { projection, .. }))
        ) if projection.iter().any(|projection| matches!(
            projection,
            Projection::Field {
                index: 0,
                identity: Some(identity),
            } if identity.owner == struct_id && identity.field_id == field_id
        ))
    )));
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib test_lower_struct_literal_orders_operands_by_field_id -- --nocapture && cargo test -p rock-lib enum_variant_lowers_to_canonical_aggregate_with_payloads -- --nocapture && cargo test -p rock-lib struct_field_access_projection_carries_field_identity -- --nocapture`

Expected: FAIL because aggregate variants are still string/tuple shaped, enum variants lower to unit, and field projections do not carry field identity.

- [ ] **Step 3: Replace aggregate kinds**

In `lib/src/mir/mod.rs`, replace `AggregateKind` with:

```rust
#[derive(Debug, Clone)]
pub enum AggregateKind {
    Tuple,
    Array,
    Struct {
        id: crate::ids::DefId,
        display_name: String,
    },
    EnumVariant {
        enum_id: crate::ids::DefId,
        variant_id: crate::ids::VariantId,
        enum_name: String,
        variant_name: String,
    },
}
```

- [ ] **Step 4: Canonicalize struct field projections**

In `lib/src/mir/mod.rs`, replace the field projection variant with a form that can retain `FieldId` when HIR provides it:

```rust
Field {
    index: usize,
    identity: Option<MirFieldIdentity>,
},
```

In `lib/src/mir/builder/mod.rs`, add this helper:

```rust
fn field_projection(
    &self,
    index: usize,
    owner: Option<crate::ids::DefId>,
    field_id: Option<crate::ids::FieldId>,
) -> Projection {
    Projection::Field {
        index,
        identity: owner.zip(field_id).map(|(owner, field_id)| {
            MirFieldIdentity::new(owner, field_id)
        }),
    }
}
```

Update struct field access lowering in `lib/src/mir/builder/expr.rs` to use the canonical HIR field location when available:

```rust
place.projection.push(self.field_projection(
    field_idx,
    location.as_ref().map(|location| location.owner),
    location.as_ref().map(|location| location.field_id),
));
```

Tuple fields and enum payload fields must use `identity: None` because they currently have index-only identity.

- [ ] **Step 5: Update struct literal lowering**

In `lib/src/mir/builder/expr.rs`, change the struct literal arm to use the optional `DefId` sidecar. Replace:

```rust
HirExprKind::StructLiteral(name, _, fields) => {
```

with:

```rust
HirExprKind::StructLiteral(name, explicit_id, fields) => {
```

Immediately after lowering `fields` into `lowered_fields`, derive the canonical struct id from HIR sidecar/type information:

```rust
let struct_id = explicit_id.or_else(|| match &expr.ty {
    Type::Struct { id, .. } => Some(*id),
    _ => None,
});
let Some(struct_id) = struct_id else {
    panic!("struct literal MIR lowering requires canonical struct id for {name}");
};
```

Then replace the field-ordering lookup so it uses the canonical id, not the display name:

```rust
let field_operands = if let Some((_, struct_def)) = self.program.struct_by_id(struct_id) {
    let mut ordered: Vec<Option<Operand>> = vec![None; struct_def.fields.len()];
    for (field, operand) in lowered_fields {
        let field_idx = field
            .field
            .as_ref()
            .filter(|location| location.owner == struct_def.id)
            .and_then(|location| {
                struct_def.fields.iter().position(|struct_field| {
                    struct_field.id == location.field_id
                        && struct_field.name == location.name
                        && struct_field.name == field.name
                })
            });

        if let Some(field_idx) = field_idx {
            if let Some(slot) = ordered.get_mut(field_idx) {
                *slot = Some(operand);
            }
        }
    }
    ordered.into_iter().flatten().collect()
} else {
    panic!("struct literal MIR lowering missing struct definition for {struct_id:?}");
};
```

Replace the rvalue construction with:

```rust
let rvalue = Rvalue::Aggregate(
    AggregateKind::Struct {
        id: struct_id,
        display_name: name.clone(),
    },
    field_operands,
);
self.emit_assign(dest, rvalue, span);
```

- [ ] **Step 6: Update enum variant lowering**

Replace the `HirExprKind::EnumVariant` arm in `lib/src/mir/builder/expr.rs` with:

```rust
HirExprKind::EnumVariant(enum_name, variant_name, args, location) => {
    let mut arg_operands = Vec::new();
    for arg in args {
        let arg_temp = self.new_local_from_expr(arg.ty.clone(), arg);
        let arg_place = Place {
            local: arg_temp,
            projection: vec![],
        };
        self.lower_expr(arg, arg_place.clone());
        arg_operands.push(Operand::Copy(arg_place));
    }

    if let Some(location) = location {
        self.emit_assign(
            dest,
            Rvalue::Aggregate(
                AggregateKind::EnumVariant {
                    enum_id: location.owner,
                    variant_id: location.variant_id,
                    enum_name: enum_name.clone(),
                    variant_name: variant_name.clone(),
                },
                arg_operands,
            ),
            span,
        );
    }
}
```

- [ ] **Step 7: Update any aggregate and projection pattern matches**

Run: `rg "AggregateKind::Struct|Projection::Field" lib/src/mir --glob '*.rs'`

Expected: update all matches to the new struct variant shape and the new field projection shape. Do not add string-only semantic checks.

- [ ] **Step 8: Run focused verification**

Run: `cargo test -p rock-lib test_lower_struct_literal_orders_operands_by_field_id -- --nocapture && cargo test -p rock-lib enum_variant_lowers_to_canonical_aggregate_with_payloads -- --nocapture && cargo test -p rock-lib struct_field_access_projection_carries_field_identity -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && cargo test -p rock-lib mir::borrowck -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 10: Commit Task 5**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs lib/src/mir/borrowck lib/src/mir/dataflow
git commit -m "canonicalize mir aggregate identities"
```

## Task 6: Represent Enum Match Discriminants And Downcasts

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/builder/blocks.rs`
- Modify: `lib/src/mir/borrowck/**`
- Modify: `lib/src/mir/dataflow/**`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add failing enum match MIR test**

Add this test to `lib/src/mir/builder/mod.rs` tests:

```rust
#[test]
fn enum_match_lowers_to_discriminant_switch() {
    let enum_id = DefId::new(CrateId(0), LocalDefId(40));
    let none_id = crate::ids::VariantId(0);
    let some_id = crate::ids::VariantId(1);
    let mut enums = std::collections::HashMap::new();
    enums.insert(
        "Option".to_string(),
        crate::hir::HirEnum {
            id: enum_id,
            name: "Option".to_string(),
            generic_params: vec![],
            variants: vec![
                crate::hir::HirVariant {
                    id: none_id,
                    name: "None".to_string(),
                    fields: crate::hir::HirVariantFields::Unit,
                },
                crate::hir::HirVariant {
                    id: some_id,
                    name: "Some".to_string(),
                    fields: crate::hir::HirVariantFields::Positional(vec![Type::I64]),
                },
            ],
        },
    );
    let program = HirProgram::from_parts(
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        enums,
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );
    let mut builder = builder_with_var(
        &program,
        "opt",
        Type::Enum {
            id: enum_id,
            args: vec![],
        },
    );
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let match_expr = expr(
        HirExprKind::Match {
            scrutinee: Box::new(expr(
                HirExprKind::Var("opt".to_string()),
                Type::Enum {
                    id: enum_id,
                    args: vec![],
                },
            )),
            arms: vec![
                crate::hir::HirMatchArm {
                    pattern: crate::hir::HirPattern::Enum(
                        "Option".to_string(),
                        "None".to_string(),
                        Some(crate::hir::HirVariantLocation {
                            owner: enum_id,
                            variant_id: none_id,
                            name: "None".to_string(),
                        }),
                        vec![],
                    ),
                    guard: None,
                    body: HirBlock {
                        stmts: vec![crate::hir::HirStmt::Expr(expr(
                            HirExprKind::IntLiteral(0),
                            Type::I64,
                        ))],
                        ty: Type::I64,
                    },
                },
                crate::hir::HirMatchArm {
                    pattern: crate::hir::HirPattern::Enum(
                        "Option".to_string(),
                        "Some".to_string(),
                        Some(crate::hir::HirVariantLocation {
                            owner: enum_id,
                            variant_id: some_id,
                            name: "Some".to_string(),
                        }),
                        vec![],
                    ),
                    guard: None,
                    body: HirBlock {
                        stmts: vec![crate::hir::HirStmt::Expr(expr(
                            HirExprKind::IntLiteral(1),
                            Type::I64,
                        ))],
                        ty: Type::I64,
                    },
                },
            ],
        },
        Type::I64,
    );

    builder.lower_expr(
        &match_expr,
        Place {
            local: Local(1),
            projection: vec![],
        },
    );

    let switch_values: Vec<i64> = builder
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Some(crate::mir::Terminator::SwitchInt { targets, .. }) => Some(targets),
            _ => None,
        })
        .flat_map(|targets| targets.iter().map(|(value, _)| *value))
        .collect();
    assert!(switch_values.contains(&(none_id.0 as i64)));
    assert!(switch_values.contains(&(some_id.0 as i64)));
    assert!(builder.blocks.iter().flat_map(|block| &block.statements).any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(_, Rvalue::Discriminant(_))
    )));
}
```

Add this second test in the same test module to lock in payload binding, downcast projection, and guard lowering:

```rust
#[test]
fn enum_match_payload_binding_uses_downcast_field_projection() {
    let enum_id = DefId::new(CrateId(0), LocalDefId(41));
    let some_id = crate::ids::VariantId(1);
    let mut enums = std::collections::HashMap::new();
    enums.insert(
        "Option".to_string(),
        crate::hir::HirEnum {
            id: enum_id,
            name: "Option".to_string(),
            generic_params: vec![],
            variants: vec![crate::hir::HirVariant {
                id: some_id,
                name: "Some".to_string(),
                fields: crate::hir::HirVariantFields::Positional(vec![Type::I64]),
            }],
        },
    );
    let program = HirProgram::from_parts(
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        enums,
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );
    let mut builder = builder_with_var(
        &program,
        "opt",
        Type::Enum {
            id: enum_id,
            args: vec![],
        },
    );
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let match_expr = expr(
        HirExprKind::Match {
            scrutinee: Box::new(expr(
                HirExprKind::Var("opt".to_string()),
                Type::Enum {
                    id: enum_id,
                    args: vec![],
                },
            )),
            arms: vec![crate::hir::HirMatchArm {
                pattern: crate::hir::HirPattern::Enum(
                    "Option".to_string(),
                    "Some".to_string(),
                    Some(crate::hir::HirVariantLocation {
                        owner: enum_id,
                        variant_id: some_id,
                        name: "Some".to_string(),
                    }),
                    vec![crate::hir::HirPattern::Binding("value".to_string(), false)],
                ),
                guard: Some(expr(
                    HirExprKind::BinOp(
                        crate::hir::BinOp::Gt,
                        Box::new(expr(HirExprKind::Var("value".to_string()), Type::I64)),
                        Box::new(expr(HirExprKind::IntLiteral(0), Type::I64)),
                    ),
                    Type::Bool,
                )),
                body: HirBlock {
                    stmts: vec![crate::hir::HirStmt::Expr(expr(
                        HirExprKind::Var("value".to_string()),
                        Type::I64,
                    ))],
                    ty: Type::I64,
                },
            }],
        },
        Type::I64,
    );

    builder.lower_expr(
        &match_expr,
        Place {
            local: Local(1),
            projection: vec![],
        },
    );

    assert!(builder.blocks.iter().flat_map(|block| &block.statements).any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::Use(Operand::Copy(Place { projection, .. }))
        ) if projection.contains(&Projection::Downcast(some_id))
            && projection.iter().any(|projection| matches!(
                projection,
                Projection::Field {
                    index: 0,
                    identity: None,
                }
            ))
    )));
    assert!(builder.blocks.iter().any(|block| matches!(
        &block.terminator,
        Some(crate::mir::Terminator::SwitchInt { discr, .. })
            if matches!(discr, Operand::Copy(_))
    )));
    let value_local = builder
        .locals
        .iter()
        .position(|decl| decl.name.as_deref() == Some("value"))
        .map(Local)
        .expect("payload binding local should exist");
    let guard_false_cleanup = builder
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(crate::mir::Terminator::SwitchInt { targets, otherwise, .. })
                if targets.iter().any(|(value, _)| *value == 1) => Some(*otherwise),
            _ => None,
        })
        .expect("guard false edge should use a cleanup block");
    assert!(builder.blocks[guard_false_cleanup.0]
        .statements
        .iter()
        .any(|stmt| matches!(&stmt.kind, StatementKind::StorageDead(local) if *local == value_local)));
}
```

Add this third test in the same test module so non-enum match arms are not silently skipped and do not emit enum discriminants:

```rust
#[test]
fn literal_match_uses_value_comparison_not_discriminant() {
    let program = empty_program();
    let mut builder = builder_with_var(&program, "n", Type::I64);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let match_expr = expr(
        HirExprKind::Match {
            scrutinee: Box::new(expr(HirExprKind::Var("n".to_string()), Type::I64)),
            arms: vec![
                crate::hir::HirMatchArm {
                    pattern: crate::hir::HirPattern::Literal(
                        crate::hir::HirLiteralPattern::Int(0),
                    ),
                    guard: None,
                    body: HirBlock {
                        stmts: vec![crate::hir::HirStmt::Expr(expr(
                            HirExprKind::IntLiteral(1),
                            Type::I64,
                        ))],
                        ty: Type::I64,
                    },
                },
                crate::hir::HirMatchArm {
                    pattern: crate::hir::HirPattern::Wildcard,
                    guard: None,
                    body: HirBlock {
                        stmts: vec![crate::hir::HirStmt::Expr(expr(
                            HirExprKind::IntLiteral(2),
                            Type::I64,
                        ))],
                        ty: Type::I64,
                    },
                },
            ],
        },
        Type::I64,
    );

    builder.lower_expr(
        &match_expr,
        Place {
            local: Local(1),
            projection: vec![],
        },
    );

    assert!(!builder.blocks.iter().flat_map(|block| &block.statements).any(|stmt| {
        matches!(&stmt.kind, StatementKind::Assign(_, Rvalue::Discriminant(_)))
    }));
    assert!(builder.blocks.iter().flat_map(|block| &block.statements).any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(
            _,
            Rvalue::BinaryOp(
                crate::hir::BinOp::Eq,
                _,
                Operand::Constant(crate::mir::Constant::Int(0)),
            ),
        )
    )));
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p rock-lib enum_match_lowers_to_discriminant_switch -- --nocapture && cargo test -p rock-lib enum_match_payload_binding_uses_downcast_field_projection -- --nocapture && cargo test -p rock-lib literal_match_uses_value_comparison_not_discriminant -- --nocapture`

Expected: FAIL because `Rvalue::Discriminant` does not exist, `Projection::Downcast` does not carry `VariantId`, payload bindings are not lowered, and match lowering still emits unit.

- [ ] **Step 3: Add discriminant and downcast MIR forms**

In `lib/src/mir/mod.rs`, add this to `Rvalue`:

```rust
Discriminant(Place),
```

Change `Projection::Downcast(usize)` to:

```rust
Downcast(crate::ids::VariantId),
```

- [ ] **Step 4: Update borrowck and dataflow exhaustive matches**

Run: `cargo test -p rock-lib enum_match_lowers_to_discriminant_switch -- --nocapture`

Expected: compilation errors in exhaustive matches. For each new `Rvalue::Discriminant(place)` arm, treat it like reading `place`. For `Projection::Downcast(VariantId)`, keep existing place traversal behavior; only the payload type changed.

Use this rule when updating borrowck/dataflow helper functions:

```rust
Rvalue::Discriminant(place) => {
    // same read behavior as using the enum place without moving it
}
```

Where a function returns access events, classify `Discriminant(place)` as a shared read of `place`. Where a function tracks initialization, do not mark the enum place moved.

- [ ] **Step 5: Extract reusable scope cleanup blocks for arm-local bindings**

In `lib/src/mir/builder/blocks.rs`, extract the scope cleanup loop at the end of `lower_block` into helpers that can clean the current path or synthesize a cleanup block for a branch edge:

```rust
pub(super) fn finish_scope_locals(&mut self, block_locals: Vec<crate::mir::Local>) {
    let mut current_opt = self.current_block;
    for local in block_locals.into_iter().rev() {
        if let Some(current) = current_opt {
            self.blocks[current.0]
                .statements
                .push(StatementData::storage_dead(local, None));

            let ty = &self.locals[local.0].ty;
            if Self::needs_drop(ty) {
                let next_block = self.new_block();
                self.blocks[current.0].terminator = Some(Terminator::Drop {
                    place: Place {
                        local,
                        projection: vec![],
                    },
                    target: next_block,
                    unwind: None,
                });
                current_opt = Some(next_block);
                self.current_block = Some(next_block);
            }
        }
    }
}

pub(super) fn scope_cleanup_block(
    &mut self,
    block_locals: Vec<crate::mir::Local>,
    target: crate::mir::BasicBlockId,
) -> crate::mir::BasicBlockId {
    let cleanup_start = self.new_block();
    let previous_current = self.current_block;
    self.current_block = Some(cleanup_start);
    self.finish_scope_locals(block_locals);
    if let Some(current) = self.current_block {
        if self.blocks[current.0].terminator.is_none() {
            self.blocks[current.0].terminator = Some(Terminator::Goto(target));
        }
    }
    self.current_block = previous_current;
    cleanup_start
}
```

Then replace the old duplicated loop in `lower_block` with:

```rust
let block_locals = self.scope_locals.pop().unwrap_or_default();
self.finish_scope_locals(block_locals);
```

- [ ] **Step 6: Route abrupt exits through active cleanup**

In `lib/src/mir/builder/blocks.rs`, add a helper that emits `StorageDead`/`Drop` for currently active locals before installing an abrupt terminator. Use it for `return`, `break`, and `continue` so match-arm pattern bindings are cleaned even when an arm body does not fall through normally.

```rust
fn active_cleanup_locals_from(&self, depth: usize) -> Vec<crate::mir::Local> {
    self.scope_locals
        .iter()
        .skip(depth)
        .flat_map(|locals| locals.iter().copied())
        .collect()
}

pub(super) fn terminate_with_cleanup(
    &mut self,
    cleanup_depth: usize,
    terminator: Terminator,
) {
    let locals = self.active_cleanup_locals_from(cleanup_depth);
    self.finish_scope_locals(locals);
    if let Some(current) = self.current_block {
        self.blocks[current.0].terminator = Some(terminator);
    }
    self.current_block = None;
}
```

Change `loop_stack` in `MirBuilder` from a tuple to a small struct that records the cleanup boundary for loop exits:

```rust
#[derive(Debug, Clone, Copy)]
struct LoopTargets {
    break_target: BasicBlockId,
    continue_target: BasicBlockId,
    cleanup_depth: usize,
}
```

When lowering a `while`, push `LoopTargets { break_target: merge_block, continue_target: cond_block, cleanup_depth: self.scope_locals.len() }`. Replace direct `Return`, `Goto(break_target)`, and `Goto(continue_target)` installation in `lower_block` with:

```rust
self.terminate_with_cleanup(0, Terminator::Return);
self.terminate_with_cleanup(loop_targets.cleanup_depth, Terminator::Goto(loop_targets.break_target));
self.terminate_with_cleanup(loop_targets.cleanup_depth, Terminator::Goto(loop_targets.continue_target));
```

- [ ] **Step 7: Add pattern binding helpers**

In `impl MirBuilder` in `lib/src/mir/builder/mod.rs`, add these helpers before implementing match lowering:

```rust
fn enum_variant_field_types(
    &self,
    enum_id: crate::ids::DefId,
    variant_id: crate::ids::VariantId,
) -> Vec<Type> {
    self.program
        .enum_by_id(enum_id)
        .and_then(|(_, enumeration)| {
            enumeration
                .variants
                .iter()
                .find(|variant| variant.id == variant_id)
        })
        .map(|variant| match &variant.fields {
            crate::hir::HirVariantFields::Named(fields) => {
                fields.iter().map(|field| field.ty.clone()).collect()
            }
            crate::hir::HirVariantFields::Positional(fields) => fields.clone(),
            crate::hir::HirVariantFields::Unit => Vec::new(),
        })
        .unwrap_or_default()
}

fn bind_match_pattern_places(
    &mut self,
    pattern: &crate::hir::HirPattern,
    source_place: Place,
    source_ty: &Type,
    restored_bindings: &mut Vec<(String, Option<Local>)>,
) {
    match pattern {
        crate::hir::HirPattern::Binding(name, mutable) => {
            let local = self.new_local(source_ty.clone(), if *mutable { Mutability::Mut } else { Mutability::Not }, Some(name.clone()));
            self.emit_storage_live(local, None);
            let previous = self.var_map.insert(name.clone(), local);
            restored_bindings.push((name.clone(), previous));
            if let Some(scope_locals) = self.scope_locals.last_mut() {
                scope_locals.push(local);
            }
            self.emit_assign(
                Place {
                    local,
                    projection: vec![],
                },
                Rvalue::Use(self.operand_for_place(source_ty, source_place, false)),
                None,
            );
        }
        crate::hir::HirPattern::Enum(_, _, Some(location), subpatterns) => {
            let field_tys = self.enum_variant_field_types(location.owner, location.variant_id);
            for (index, subpattern) in subpatterns.iter().enumerate() {
                let mut field_place = source_place.clone();
                field_place.projection.push(Projection::Downcast(location.variant_id));
                field_place.projection.push(Projection::Field {
                    index,
                    identity: None,
                });
                let field_ty = field_tys.get(index).cloned().unwrap_or(Type::Unit);
                self.bind_match_pattern_places(
                    subpattern,
                    field_place,
                    &field_ty,
                    restored_bindings,
                );
            }
        }
        crate::hir::HirPattern::Tuple(subpatterns) => {
            if let Type::Tuple(field_tys) = source_ty {
                for (index, subpattern) in subpatterns.iter().enumerate() {
                    let mut field_place = source_place.clone();
                    field_place.projection.push(Projection::Field {
                        index,
                        identity: None,
                    });
                    let field_ty = field_tys.get(index).cloned().unwrap_or(Type::Unit);
                    self.bind_match_pattern_places(
                        subpattern,
                        field_place,
                        &field_ty,
                        restored_bindings,
                    );
                }
            }
        }
        crate::hir::HirPattern::Struct(_, Some(struct_id), _, fields) => {
            if let Some((_, structure)) = self.program.struct_by_id(*struct_id) {
                for field in fields {
                    let Some(location) = &field.field else {
                        continue;
                    };
                    let Some((index, field_def)) = structure
                        .fields
                        .iter()
                        .enumerate()
                        .find(|(_, field_def)| field_def.id == location.field_id)
                    else {
                        continue;
                    };
                    let mut field_place = source_place.clone();
                    field_place.projection.push(self.field_projection(
                        index,
                        Some(location.owner),
                        Some(location.field_id),
                    ));
                    self.bind_match_pattern_places(
                        &field.pattern,
                        field_place,
                        &field_def.ty,
                        restored_bindings,
                    );
                }
            }
        }
        crate::hir::HirPattern::Wildcard
        | crate::hir::HirPattern::Literal(_)
        | crate::hir::HirPattern::Or(_) => {}
        crate::hir::HirPattern::Struct(_, None, _, _) => {}
        crate::hir::HirPattern::Enum(_, _, None, _) => {}
    }
}

fn match_arm_variant_id(pattern: &crate::hir::HirPattern) -> Option<crate::ids::VariantId> {
    match pattern {
        crate::hir::HirPattern::Enum(_, _, Some(location), _) => Some(location.variant_id),
        _ => None,
    }
}

fn match_arm_is_catch_all(pattern: &crate::hir::HirPattern) -> bool {
    matches!(
        pattern,
        crate::hir::HirPattern::Wildcard | crate::hir::HirPattern::Binding(_, _)
    )
}

fn pattern_is_irrefutable_for_struct_match(pattern: &crate::hir::HirPattern) -> bool {
    match pattern {
        crate::hir::HirPattern::Wildcard | crate::hir::HirPattern::Binding(_, _) => true,
        crate::hir::HirPattern::Tuple(patterns) => patterns
            .iter()
            .all(Self::pattern_is_irrefutable_for_struct_match),
        crate::hir::HirPattern::Struct(_, _, _, fields) => fields
            .iter()
            .all(|field| Self::pattern_is_irrefutable_for_struct_match(&field.pattern)),
        crate::hir::HirPattern::Literal(_)
        | crate::hir::HirPattern::Enum(..)
        | crate::hir::HirPattern::Or(_) => false,
    }
}

fn lower_match_pattern_check(
    &mut self,
    pattern: &crate::hir::HirPattern,
    source_place: Place,
    source_ty: &Type,
    success: BasicBlockId,
    failure: BasicBlockId,
) {
    match pattern {
        crate::hir::HirPattern::Wildcard | crate::hir::HirPattern::Binding(_, _) => {
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::Goto(success));
            }
        }
        crate::hir::HirPattern::Enum(_, _, Some(location), _) => {
            let discr_temp = self.new_local(Type::I64, Mutability::Not, None);
            self.emit_storage_live(discr_temp, None);
            let discr_place = Place {
                local: discr_temp,
                projection: vec![],
            };
            self.emit_assign(
                discr_place.clone(),
                Rvalue::Discriminant(source_place),
                None,
            );
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::SwitchInt {
                    discr: Operand::Copy(discr_place),
                    targets: vec![(location.variant_id.0 as i64, success)],
                    otherwise: failure,
                });
            }
        }
        crate::hir::HirPattern::Literal(literal) => {
            let lit_temp = self.new_local(Type::Bool, Mutability::Not, None);
            self.emit_storage_live(lit_temp, None);
            let lit_place = Place {
                local: lit_temp,
                projection: vec![],
            };
            let literal_operand = match literal {
                crate::hir::HirLiteralPattern::Int(value) => Operand::Constant(Constant::Int(*value)),
                crate::hir::HirLiteralPattern::Bool(value) => Operand::Constant(Constant::Bool(*value)),
                crate::hir::HirLiteralPattern::String(value) => Operand::Constant(Constant::String(value.clone())),
                crate::hir::HirLiteralPattern::Char(value) => Operand::Constant(Constant::Char(*value)),
                crate::hir::HirLiteralPattern::Float(value) => Operand::Constant(Constant::Float(*value)),
            };
            self.emit_assign(
                lit_place.clone(),
                Rvalue::BinaryOp(
                    crate::hir::BinOp::Eq,
                    self.operand_for_place(source_ty, source_place, true),
                    literal_operand,
                ),
                None,
            );
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::SwitchInt {
                    discr: Operand::Copy(lit_place),
                    targets: vec![(1, success)],
                    otherwise: failure,
                });
            }
        }
        crate::hir::HirPattern::Tuple(_) | crate::hir::HirPattern::Struct(..)
            if Self::pattern_is_irrefutable_for_struct_match(pattern) =>
        {
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::Goto(success));
            }
        }
        crate::hir::HirPattern::Tuple(_) | crate::hir::HirPattern::Struct(..) => {
            panic!("MIR match lowering requires recursive checks for refutable pattern {pattern:?}");
        }
        crate::hir::HirPattern::Or(_)
        | crate::hir::HirPattern::Enum(_, _, None, _) => {
            panic!("MIR match lowering missing runtime pattern check for {pattern:?}");
        }
    }
}

fn restore_match_bindings(&mut self, restored_bindings: Vec<(String, Option<Local>)>) {
    for (name, previous) in restored_bindings.into_iter().rev() {
        match previous {
            Some(local) => {
                self.var_map.insert(name, local);
            }
            None => {
                self.var_map.remove(&name);
            }
        }
    }
}
```

- [ ] **Step 8: Implement source-order match lowering**

Replace the `HirExprKind::Match` arm in `lib/src/mir/builder/expr.rs` with an implementation that:

1. Lowers the scrutinee into a temporary local.
2. Creates a discriminant temporary of `Type::I64` only when at least one arm is an enum pattern.
3. Emits `Rvalue::Discriminant(scrut_place.clone())` only for enum-pattern matches; literal matches compare the scrutinee value directly.
4. Creates one check block per arm plus a merge block.
5. Starts with `Goto(first_check_block)` instead of jumping directly by discriminant; each check block decides whether its arm matches. This preserves source order when wildcard/binding arms appear before enum-specific arms.
6. For enum-specific arms, the check block uses `SwitchInt` with a single target for that variant ID and otherwise jumps to the next source-order arm check.
7. For wildcard/binding arms, the check block goes directly to guard/body handling.
8. For literal arms, call `lower_match_pattern_check` so they either enter guard/body handling or fall through to the next source-order arm. Do not silently skip non-enum patterns.
9. For tuple and struct arms, enter guard/body handling only when `pattern_is_irrefutable_for_struct_match` proves all nested subpatterns are bindings/wildcards/irrefutable structs or tuples; otherwise fail loudly with the `lower_match_pattern_check` panic. For or-pattern or unresolved enum patterns, fail loudly until those pattern forms get complete MIR checks; do not emit `Constant::Unit` or leave `dest` silently unassigned.
10. Pushes a temporary scope before binding pattern places, so guards can reference payload bindings and arm locals are cleaned up.
11. Calls `bind_match_pattern_places` before lowering the guard.
12. Lowers guards before arm bodies. A true guard enters the body block; a false guard goes through a cleanup block before the next source-order check block or merge block.
13. Calls `finish_scope_locals` and restores `var_map` entries after lowering the guard/body for each arm.
14. Sends arm body blocks to the merge block.

Use this skeleton:

```rust
HirExprKind::Match { scrutinee, arms } => {
    let scrut_temp = self.new_local_from_expr(scrutinee.ty.clone(), scrutinee);
    self.emit_storage_live(scrut_temp, Some(scrutinee.span.clone()));
    let scrut_place = Place {
        local: scrut_temp,
        projection: vec![],
    };
    self.lower_expr(scrutinee, scrut_place.clone());

    let discr_place = if arms
        .iter()
        .any(|arm| Self::match_arm_variant_id(&arm.pattern).is_some())
    {
        let discr_temp = self.new_local(Type::I64, Mutability::Not, None);
        self.emit_storage_live(discr_temp, Some(scrutinee.span.clone()));
        let discr_place = Place {
            local: discr_temp,
            projection: vec![],
        };
        self.emit_assign(
            discr_place.clone(),
            Rvalue::Discriminant(scrut_place.clone()),
            span.clone(),
        );
        Some(discr_place)
    } else {
        None
    };

    let current = match self.current_block {
        Some(current) => current,
        None => return,
    };
    let merge_block = self.new_block();
    let arm_checks: Vec<_> = arms.iter().map(|arm| (self.new_block(), arm)).collect();

    self.blocks[current.0].terminator = Some(Terminator::Goto(
        arm_checks
            .first()
            .map(|(block, _)| *block)
            .unwrap_or(merge_block),
    ));

    for (index, (check_block, arm)) in arm_checks.iter().enumerate() {
        self.current_block = Some(*check_block);
        let body_block = self.new_block();
        let current_variant = Self::match_arm_variant_id(&arm.pattern);
        let next_block = arm_checks
            .iter()
            .skip(index + 1)
            .next()
            .map(|(next, _)| *next)
            .unwrap_or(merge_block);

        let guard_entry_block = if let Some(variant_id) = current_variant {
            let pattern_matched_block = self.new_block();
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::SwitchInt {
                    discr: Operand::Copy(
                        discr_place
                            .clone()
                            .expect("enum arm requires discriminant place"),
                    ),
                    targets: vec![(variant_id.0 as i64, pattern_matched_block)],
                    otherwise: next_block,
                });
            }
            pattern_matched_block
        } else if Self::match_arm_is_catch_all(&arm.pattern) {
            let pattern_matched_block = self.new_block();
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::Goto(pattern_matched_block));
            }
            pattern_matched_block
        } else {
            let pattern_matched_block = self.new_block();
            self.lower_match_pattern_check(
                &arm.pattern,
                scrut_place.clone(),
                &scrutinee.ty,
                pattern_matched_block,
                next_block,
            );
            pattern_matched_block
        };

        self.current_block = Some(guard_entry_block);
        self.scope_locals.push(Vec::new());

        let mut restored_bindings = Vec::new();
        self.bind_match_pattern_places(
            &arm.pattern,
            scrut_place.clone(),
            &scrutinee.ty,
            &mut restored_bindings,
        );

        if let Some(guard) = &arm.guard {
            let guard_temp = self.new_local(Type::Bool, Mutability::Not, None);
            self.emit_storage_live(guard_temp, Some(guard.span.clone()));
            if let Some(scope_locals) = self.scope_locals.last_mut() {
                scope_locals.push(guard_temp);
            }
            let guard_place = Place {
                local: guard_temp,
                projection: vec![],
            };
            self.lower_expr(guard, guard_place.clone());
            let guard_false_cleanup = self.scope_cleanup_block(
                self.scope_locals.last().cloned().unwrap_or_default(),
                next_block,
            );
            if let Some(current) = self.current_block {
                self.blocks[current.0].terminator = Some(Terminator::SwitchInt {
                    discr: Operand::Copy(guard_place),
                    targets: vec![(1, body_block)],
                    otherwise: guard_false_cleanup,
                });
            }
        } else if let Some(current) = self.current_block {
            self.blocks[current.0].terminator = Some(Terminator::Goto(body_block));
        }

        self.current_block = Some(body_block);
        self.lower_block(&arm.body, dest.clone());
        let arm_locals = self.scope_locals.pop().unwrap_or_default();
        self.finish_scope_locals(arm_locals);
        self.restore_match_bindings(restored_bindings);
        if let Some(current) = self.current_block {
            if self.blocks[current.0].terminator.is_none() {
                self.blocks[current.0].terminator = Some(Terminator::Goto(merge_block));
            }
        }
    }

    self.current_block = Some(merge_block);
}
```

- [ ] **Step 9: Run focused verification**

Run: `cargo test -p rock-lib enum_match_lowers_to_discriminant_switch -- --nocapture && cargo test -p rock-lib enum_match_payload_binding_uses_downcast_field_projection -- --nocapture && cargo test -p rock-lib literal_match_uses_value_comparison_not_discriminant -- --nocapture`

Expected: PASS.

- [ ] **Step 10: Run enum behavior verification**

Run these commands serially:

```bash
cargo test -p rock-lib --test integration test_enum -- --nocapture
cargo test -p rock-lib --test integration test_match_expr -- --nocapture
cargo test -p rock-lib --test integration test_struct_pattern -- --nocapture
```

Expected: PASS.

- [ ] **Step 11: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && cargo test -p rock-lib mir::borrowck -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 12: Commit Task 6**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs lib/src/mir/builder/blocks.rs lib/src/mir/borrowck lib/src/mir/dataflow
git commit -m "represent enum matches in mir"
```

## Task 7: Add Closure Identity And Runtime Requirement Forms

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/borrowck/**`
- Modify: `lib/src/mir/dataflow/**`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add failing closure and runtime requirement tests**

Add these tests to `lib/src/mir/builder/mod.rs` tests:

```rust
#[test]
fn closure_rvalue_uses_stable_closure_identity() {
    let function_id = DefId::new(CrateId(0), LocalDefId(50));
    let function = HirFunction {
        body: HirBlock {
            stmts: vec![crate::hir::HirStmt::Expr(expr(
                HirExprKind::Lambda {
                    params: vec![],
                    body: HirBlock {
                        stmts: vec![],
                        ty: Type::Unit,
                    },
                    captures: vec![],
                },
                Type::Unit,
            ))],
            ty: Type::Unit,
        },
        ..empty_function(function_id, "make_closure")
    };
    let program = HirProgram::from_parts(
        std::collections::HashMap::from([("make_closure".to_string(), function)]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );
    let mir = MirBuilder::build(&program);
    let function = mir
        .function(crate::mir::MirFunctionId::Function(function_id))
        .expect("function MIR should exist");

    assert!(function.basic_blocks.iter().flat_map(|block| &block.statements).any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(_, Rvalue::Closure(closure))
            if closure.id.owner == crate::mir::MirFunctionId::Function(function_id)
                && closure.display_name.starts_with("lambda_")
    )));
}

#[test]
fn numeric_cast_lowers_to_runtime_cast_rvalue() {
    let program = empty_program();
    let mut builder = builder_with_var(&program, "n", Type::I64);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::F64,
        mutability: Mutability::Not,
        name: Some("dest".to_string()),
        span: None,
    });
    let cast_expr = expr(
        HirExprKind::Cast(
            Box::new(expr(HirExprKind::Var("n".to_string()), Type::I64)),
            Type::F64,
        ),
        Type::F64,
    );

    builder.lower_expr_with_context(
        &cast_expr,
        Place {
            local: Local(1),
            projection: vec![],
        },
        false,
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(_, Rvalue::Cast(Operand::Copy(_), Type::F64))
    )));
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib closure_rvalue_uses_stable_closure_identity -- --nocapture && cargo test -p rock-lib numeric_cast_lowers_to_runtime_cast_rvalue -- --nocapture`

Expected: FAIL because `MirClosure` has no `id`/`display_name`, and non-pointer casts still lower to copy.

- [ ] **Step 3: Update closure MIR shape**

In `lib/src/mir/mod.rs`, replace `MirClosure` with:

```rust
#[derive(Debug, Clone)]
pub struct MirClosure {
    pub id: MirClosureId,
    pub display_name: String,
    pub captures: Vec<MirClosureCapture>,
}
```

In `MirBuilder`, add a field:

```rust
current_function_id: Option<MirFunctionId>,
```

Initialize it to `None` in `new`. At the start of `build_function`, set it to `Some(id)`. Before returning, keep the field value; each `MirBuilder` is per-function.

Replace `new_lambda_name` with:

```rust
fn new_closure_id_and_name(&mut self) -> (MirClosureId, String) {
    let owner = self
        .current_function_id
        .expect("closure lowering requires current MIR function id");
    let index = self.lambda_counter as u32;
    let name = format!("lambda_{}", self.lambda_counter);
    self.lambda_counter += 1;
    (
        MirClosureId {
            owner,
            local_index: index,
        },
        name,
    )
}
```

- [ ] **Step 4: Update closure lowering**

In `lib/src/mir/builder/expr.rs`, replace the lambda lowering setup:

```rust
let lambda_name = self.new_lambda_name();
```

with:

```rust
let (closure_id, lambda_name) = self.new_closure_id_and_name();
```

Replace `Rvalue::Closure` construction with:

```rust
Rvalue::Closure(MirClosure {
    id: closure_id,
    display_name: lambda_name,
    captures: closure_captures,
})
```

- [ ] **Step 5: Make runtime casts explicit**

In `lib/src/mir/builder/expr.rs`, replace the cast lowering condition:

```rust
if matches!(target_ty, Type::Pointer(_)) {
```

with:

```rust
if inner.ty != *target_ty {
```

Keep the `Rvalue::Cast` assignment body unchanged. Keep the copy fallback only for identity casts where `inner.ty == *target_ty`.

- [ ] **Step 6: Update borrowck/dataflow for closure field rename**

Run: `cargo test -p rock-lib closure_rvalue_uses_stable_closure_identity -- --nocapture`

Expected: compiler errors at old `closure.function` references. Replace debug/name reads with `closure.display_name` or ignore the display field if only captures matter.

- [ ] **Step 7: Run focused verification**

Run: `cargo test -p rock-lib closure_rvalue_uses_stable_closure_identity -- --nocapture && cargo test -p rock-lib numeric_cast_lowers_to_runtime_cast_rvalue -- --nocapture`

Expected: PASS.

- [ ] **Step 8: Run closure/cast behavior verification**

Run:

```bash
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo test -p rock-lib --test integration test_type_conversion -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && cargo test -p rock-lib mir::borrowck -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 10: Commit Task 7**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs lib/src/mir/borrowck lib/src/mir/dataflow
git commit -m "complete mir closure and cast identity"
```

## Task 8: Record Runtime Checks And Drop Requirements In MIR

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/builder/blocks.rs`
- Modify: `lib/src/mir/borrowck/**`
- Modify: `lib/src/mir/dataflow/**`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add failing bounds-check and enum-drop tests**

Add these tests to `lib/src/mir/builder/mod.rs` tests:

```rust
#[test]
fn index_expression_records_bounds_check_requirement() {
    let program = empty_program();
    let mut builder = MirBuilder::new(&program);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::Array(Box::new(Type::I64), 4),
        mutability: Mutability::Not,
        name: Some("arr".to_string()),
        span: None,
    });
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("i".to_string()),
        span: None,
    });
    builder.var_map.insert("arr".to_string(), Local(0));
    builder.var_map.insert("i".to_string(), Local(1));
    let index_expr = expr(
        HirExprKind::Index(
            Box::new(expr(
                HirExprKind::Var("arr".to_string()),
                Type::Array(Box::new(Type::I64), 4),
            )),
            Box::new(expr(HirExprKind::Var("i".to_string()), Type::I64)),
        ),
        Type::I64,
    );

    builder.lower_expr_with_context(
        &index_expr,
        Place {
            local: Local(0),
            projection: vec![],
        },
        false,
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assert(crate::mir::MirAssert {
            kind: crate::mir::MirAssertKind::BoundsCheck,
            ..
        })
    )));
}

#[test]
fn needs_drop_includes_enums_and_closures() {
    let enum_ty = Type::Enum {
        id: DefId::new(CrateId(0), LocalDefId(60)),
        args: vec![],
    };
    assert!(MirBuilder::needs_drop(&enum_ty));
    assert!(MirBuilder::needs_drop(&Type::Function(vec![], Box::new(Type::Unit))));
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib index_expression_records_bounds_check_requirement -- --nocapture && cargo test -p rock-lib needs_drop_includes_enums_and_closures -- --nocapture`

Expected: FAIL because `StatementKind::Assert` and `MirAssert` do not exist, and `needs_drop` does not include enums/functions.

- [ ] **Step 3: Add MIR assertion statement**

In `lib/src/mir/mod.rs`, add:

```rust
#[derive(Debug, Clone)]
pub struct MirAssert {
    pub kind: MirAssertKind,
    pub operands: Vec<Operand>,
}
```

Add this variant to `StatementKind`:

```rust
Assert(MirAssert),
```

Add this constructor to `impl StatementData`:

```rust
pub fn assert(assertion: MirAssert, span: Option<Span>) -> Self {
    Self::new(StatementKind::Assert(assertion), span)
}
```

- [ ] **Step 4: Add builder assertion helper**

In `impl MirBuilder` in `lib/src/mir/builder/mod.rs`, add:

```rust
fn emit_assert(&mut self, assertion: crate::mir::MirAssert, span: Option<Span>) {
    if let Some(current) = self.current_block {
        self.blocks[current.0]
            .statements
            .push(StatementData::assert(assertion, span));
    }
}
```

- [ ] **Step 5: Add bounds-check helper and emit it on all index paths**

In `impl MirBuilder` in `lib/src/mir/builder/mod.rs`, add this helper after `emit_assert`:

```rust
fn emit_bounds_check_for_index_place(&mut self, indexed_place: &Place, span: Option<Span>) {
    let Some(Projection::Index(index_local)) = indexed_place.projection.last().copied() else {
        return;
    };
    let mut base_place = indexed_place.clone();
    base_place.projection.pop();
    self.emit_assert(
        crate::mir::MirAssert {
            kind: crate::mir::MirAssertKind::BoundsCheck,
            operands: vec![
                Operand::Copy(base_place),
                Operand::Copy(Place {
                    local: index_local,
                    projection: vec![],
                }),
            ],
        },
        span,
    );
}
```

In `lib/src/mir/builder/expr.rs`, update both `HirExprKind::Index` paths:

First, in the fast path:

```rust
if let Some(place) = self.lower_place(expr) {
    self.emit_bounds_check_for_index_place(&place, span.clone());
    let operand = self.operand_for_place(&expr.ty, place, false);
    self.emit_assign(dest, Rvalue::Use(operand), span);
}
```

Second, in the fallback path after pushing `Projection::Index(index_temp)` and before creating the operand, add:

```rust
self.emit_bounds_check_for_index_place(&place, span.clone());
```

Do not change codegen behavior; this records MIR requirements for Task 21.

- [ ] **Step 6: Update borrowck/dataflow statement matches**

Run: `cargo test -p rock-lib index_expression_records_bounds_check_requirement -- --nocapture`

Expected: compiler errors in exhaustive `StatementKind` matches. Handle `StatementKind::Assert(assertion)` as reads of all assertion operands and no writes. It must not mark locals initialized or moved.

- [ ] **Step 7: Broaden drop requirement modeling**

In `MirBuilder::needs_drop`, replace the match with:

```rust
match ty {
    Type::Array(_, _) => true,
    Type::Struct { .. } => true,
    Type::Enum { .. } => true,
    Type::Function(_, _) => true,
    _ => false,
}
```

- [ ] **Step 8: Run focused verification**

Run: `cargo test -p rock-lib index_expression_records_bounds_check_requirement -- --nocapture && cargo test -p rock-lib needs_drop_includes_enums_and_closures -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Run runtime behavior verification**

Run:

```bash
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 10: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::builder -- --nocapture && cargo test -p rock-lib mir::borrowck -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 11: Commit Task 8**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs lib/src/mir/builder/blocks.rs lib/src/mir/borrowck lib/src/mir/dataflow
git commit -m "record mir runtime requirements"
```

## Task 9: Add MIR Agreement Checks And Placeholder Audit

**Files:**
- Create: `lib/src/mir/agreement.rs`
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Test: `lib/src/mir/agreement.rs`

- [ ] **Step 1: Add failing agreement tests**

Create `lib/src/mir/agreement.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::mir::{
        BasicBlock, Constant, Local, LocalDecl, MirFunction, MirFunctionId, MirProgram,
        Mutability, Operand, Place, Rvalue, StatementData, StatementKind,
    };
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::types::Type;

    #[test]
    fn agreement_rejects_unit_callable_placeholder_assignment() {
        let id = DefId::new(CrateId(0), LocalDefId(1));
        let function_id = MirFunctionId::Function(id);
        let function = MirFunction {
            id: function_id,
            name: "main".to_string(),
            basic_blocks: vec![BasicBlock {
                statements: vec![StatementData::assign(
                    Place {
                        local: Local(0),
                        projection: vec![],
                    },
                    Rvalue::Use(Operand::Constant(Constant::Unit)),
                    None,
                )],
                terminator: None,
            }],
            local_decls: vec![LocalDecl {
                ty: Type::Function(vec![], Box::new(Type::Unit)),
                mutability: Mutability::Not,
                name: Some("callable".to_string()),
                span: None,
            }],
            closure_captures: vec![],
            arg_count: 0,
            ret_type: Type::Unit,
        };
        let program = MirProgram {
            functions: std::collections::BTreeMap::from([(function_id, function)]),
        };

        let report = check_mir_runtime_agreement(&program);

        assert_eq!(report.non_unit_placeholder_units, 1);
        assert!(!report.is_clean());
    }
}
```

- [ ] **Step 2: Register the module and verify the test fails**

In `lib/src/mir/mod.rs`, add:

```rust
pub mod agreement;
```

Run: `cargo test -p rock-lib mir::agreement -- --nocapture`

Expected: FAIL because `check_mir_runtime_agreement` is missing.

- [ ] **Step 3: Implement agreement report**

Insert this implementation above the tests in `lib/src/mir/agreement.rs`:

```rust
use crate::mir::{Constant, MirProgram, Rvalue, StatementKind};
use crate::types::Type;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MirAgreementReport {
    pub non_unit_placeholder_units: usize,
}

impl MirAgreementReport {
    pub fn is_clean(&self) -> bool {
        self.non_unit_placeholder_units == 0
    }
}

pub fn check_mir_runtime_agreement(program: &MirProgram) -> MirAgreementReport {
    let mut report = MirAgreementReport::default();

    for (_, function) in program.functions() {
        for block in &function.basic_blocks {
            for statement in &block.statements {
                if let StatementKind::Assign(place, Rvalue::Use(operand)) = &statement.kind {
                    let Some(local_decl) = function.local_decls.get(place.local.0) else {
                        continue;
                    };
                    if !matches!(local_decl.ty, Type::Unit)
                        && matches!(operand, crate::mir::Operand::Constant(Constant::Unit))
                    {
                        report.non_unit_placeholder_units += 1;
                    }
                }
            }
        }
    }

    report
}
```

- [ ] **Step 4: Add builder placeholder audit test**

Add this test to `lib/src/mir/builder/mod.rs` tests:

```rust
#[test]
fn builder_output_has_no_callable_unit_placeholders_for_simple_function_refs() {
    let callee_id = DefId::new(CrateId(0), LocalDefId(71));
    let caller_id = DefId::new(CrateId(0), LocalDefId(72));
    let caller = HirFunction {
        body: HirBlock {
            stmts: vec![crate::hir::HirStmt::Expr(expr(
                HirExprKind::ResolvedVar(HirVarRef {
                    name: "callee".to_string(),
                    target: HirVarTarget::Function(callee_id),
                }),
                Type::Function(vec![], Box::new(Type::Unit)),
            ))],
            ty: Type::Function(vec![], Box::new(Type::Unit)),
        },
        ..empty_function(caller_id, "caller")
    };
    let program = HirProgram::from_parts(
        std::collections::HashMap::from([
            ("callee".to_string(), empty_function(callee_id, "callee")),
            ("caller".to_string(), caller),
        ]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        vec![],
        vec![],
    );

    let mir = MirBuilder::build(&program);
    let report = crate::mir::agreement::check_mir_runtime_agreement(&mir);

    assert!(report.is_clean(), "unexpected MIR placeholders: {report:?}");
}
```

Add this assertion to the enum variant, enum match, method-call, numeric-cast, and bounds-check builder tests introduced in Tasks 4-8 after each test has built a `MirProgram` or can wrap the directly built `MirBuilder` output in a one-function `MirProgram`:

```rust
let report = crate::mir::agreement::check_mir_runtime_agreement(&mir);
assert!(report.is_clean(), "unexpected MIR placeholders: {report:?}");
```

The agreement report must fail for `Constant::Unit` assigned to any non-`Type::Unit` local, so it covers callable, method, enum variant, match-result, and cast placeholders. Keep the dedicated builder assertions for bounds checks and casts because agreement cannot infer a missing assertion from a finished MIR program without HIR context.

- [ ] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib mir::agreement -- --nocapture && cargo test -p rock-lib builder_output_has_no_callable_unit_placeholders_for_simple_function_refs -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::agreement -- --nocapture && cargo test -p rock-lib mir::builder -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 7: Commit Task 9**

Run:

```bash
git add lib/src/mir/agreement.rs lib/src/mir/mod.rs lib/src/mir/builder/mod.rs
git commit -m "add mir runtime agreement checks"
```

## Task 10: Final Task 19 Verification And Cleanup

**Files:**
- Modify if needed: `lib/src/mir/**`
- Test: existing `rock-lib` tests

- [x] **Step 1: Search for name identity leaks**

Run:

```bash
rg "HashMap<String, MirFunction>|AggregateKind::Struct\(String\)|function: String|Constant::Unit" lib/src/mir --glob '*.rs'
```

Expected: no string-keyed MIR function map, no string-only struct aggregate, no closure `function: String`. `Constant::Unit` hits are acceptable for real unit values and tests that intentionally construct unit values, but not for callable, enum, match, method, cast, or runtime-check lowering.

- [x] **Step 2: Search for unit placeholders in runtime-owned lowering**

Run:

```bash
rg "Rvalue::Use\(Operand::Constant\(Constant::Unit\)\)" lib/src/mir/builder --glob '*.rs'
```

Expected: remaining production hits are only real unit expression/statement results, not function refs, methods, enum variants, matches, casts, intrinsics, or runtime checks. If a hit is ambiguous, add or update a test before changing it.

- [x] **Step 3: Run focused MIR verification**

Run these commands serially:

```bash
cargo test -p rock-lib mir::identity -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib mir::borrowck -- --nocapture
```

Expected: all commands PASS.

- [x] **Step 4: Run runtime behavior filters**

Run these commands serially:

```bash
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_enum -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo test -p rock-lib --test integration test_type_conversion -- --nocapture
```

Expected: all commands PASS.

- [x] **Step 5: Run full verification**

Run these commands serially:

```bash
cargo fmt --all --check
cargo test -p rock-lib
git diff --check
```

Expected: all commands PASS.

- [x] **Step 6: Request final code review**

Use the requesting-code-review skill with this context:

```text
Description: Finished roadmap Task 19 by making MIR carry canonical function/callable/aggregate identities and runtime-complete forms for calls, methods, enum variants, matches, closures, casts, checks, and drops. Codegen still consumes HIR; MIR agreement checks guard the transition to Task 21.

Spec: docs/superpowers/specs/2026-05-23-mir-canonical-runtime-design.md
Plan: docs/superpowers/plans/2026-05-23-mir-canonical-runtime.md

Review focus:
- MIR identity equality uses DefId/InstanceId/FieldId/VariantId instead of display names
- Function/method/extern/instance callables no longer lower to unit placeholders
- Struct and enum aggregates are canonical and payload-preserving
- Match MIR records discriminants, switch targets, and downcast-ready places
- Runtime checks/casts/drops are explicit enough for future MIR codegen
- Borrowck behavior remains stable
- HIR codegen path remains unchanged
```

Expected: reviewer returns pass or findings. Fix Critical and Important findings before continuing.

- [ ] **Step 7: Commit final cleanup if needed**

If Step 1-6 required cleanup and there are uncommitted changes, run:

```bash
git add lib/src/mir lib/tests/integration.rs
git commit -m "finish mir canonical runtime boundary"
```

If there are no uncommitted changes after prior task commits, do not create an empty commit.

2026-05-25 note: skipped because commits were not requested in this session.

## Completion Check

- [x] MIR functions are keyed by canonical identity.
- [x] Callable MIR values carry canonical `DefId` or `InstanceId` targets where available.
- [x] Method call MIR retains selected method identity.
- [x] Struct and enum aggregate MIR carries canonical IDs and payload operands.
- [x] Struct field projections carry `MirFieldIdentity` where HIR provides `FieldId`.
- [x] Match MIR uses discriminant switches and can represent downcast payload access.
- [x] Closure MIR uses stable closure IDs and display names only for debugging.
- [x] Runtime-relevant casts are represented as `Rvalue::Cast`.
- [x] Bounds checks are recorded as MIR runtime requirements.
- [x] Drop insertion includes enums and closures where current type modeling can identify them.
- [x] MIR agreement checks catch callable unit placeholders.
- [x] Codegen still consumes HIR until Task 21.
- [x] Final verification passes: `cargo fmt --all --check`, `cargo test -p rock-lib`, `git diff --check`.

## 2026-05-25 Completion Note

Task 19 is complete for the scoped MIR canonical runtime slice. MIR functions are keyed by `MirFunctionId`, callable values carry canonical function/extern/instance/method identities, struct and enum aggregates carry canonical IDs and payload operands, matches lower through discriminants/switches/downcast-ready places, closures use stable closure IDs, runtime casts and bounds checks are explicit MIR forms, and drop insertion covers enums/closures where current type modeling can identify them.

Final cleanup extended MIR agreement checks to reject callable unit placeholders both as direct `Constant::Unit` call operands and as unit-typed local/temp call operands. Codegen remains on the current HIR/MIR hybrid path until Task 21.

Verification:
- Focused MIR checks passed: `mir::identity`, `mir::builder`, `mir::agreement`, and `mir::borrowck`.
- Runtime behavior filters passed: `test_borrow_`, `test_enum`, `test_array`, `test_vec_index`, `test_closure`, and `test_type_conversion`.
- Final verification passed with `cargo fmt --all && cargo fmt --all --check && cargo test -p rock-lib > /tmp/rock-lib-task19-final.log 2>&1 && git diff --check`.
- `/tmp/rock-lib-task19-final.log`: unit tests `1255 passed; 0 failed; 1 ignored`; integration tests `276 passed; 0 failed`; parser integration test `1 passed`; doctests `1 passed; 1 ignored`.
- Final code review returned `STATUS: PASS` after the unit-local callable placeholder agreement fix.
