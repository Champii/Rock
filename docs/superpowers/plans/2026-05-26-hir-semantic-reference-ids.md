# HIR Semantic Reference IDs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete roadmap Task 3 by giving every resolved HIR semantic reference an authoritative `DefId`, `InstanceId`, field/variant ID, or scoped local ID while keeping names as compatibility/display metadata.

**Architecture:** Add a body-local `HirLocalId` identity layer, thread it through lowering scope and HIR binding/reference nodes, then add call-target sidecars and validate/remap the expanded sidecars in products/artifacts. This plan does not remove downstream compatibility string consumers; remaining string cleanup belongs to later roadmap tasks.

**Tech Stack:** Rust 2021, `rock-lib`, HIR, lowering, product artifacts, `serde`, focused Rust unit tests, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Files And Responsibilities

- Modify `lib/src/ids.rs`: define `HirLocalId`.
- Modify `lib/src/hir/mod.rs`: add `HirLocalRef` and `HirCallTarget`; extend params, lets, bindings, captures, variable refs, calls, and `for` variables with IDs; update HIR walkers/tests.
- Modify `lib/src/hir/type_ids.rs`: update HIR pattern/expression traversal for changed enum shapes.
- Modify `lib/src/lower/scope.rs`: replace tuple scope entries with `ScopeBinding` containing type, mutability, alias marker, and optional local ID.
- Modify `lib/src/lower/mod.rs`: add `IdGen<HirLocalId>` state and helpers to allocate/reset local IDs for body lowering.
- Modify `lib/src/lower/function.rs`: allocate local IDs for params and implicit `self`.
- Modify `lib/src/lower/statement.rs`: allocate local IDs for lets, reassignments, tuple destructuring temporaries, and tuple destructuring bindings.
- Modify `lib/src/lower/control_flow/pattern.rs`: allocate local IDs for pattern bindings and preserve aggregate field/variant sidecars.
- Modify `lib/src/lower/control_flow/secondary.rs`: add call-target sidecars and preserve field/method/aggregate sidecars through secondary lowering.
- Modify `lib/src/lower/control_flow/loops.rs`: allocate and store `HirLocalId` for `for` loop variables.
- Modify `lib/src/lower/paths.rs`: resolve locals as `HirVarTarget::Local`, keep unresolved `Var` only for recovery, and make closure captures store local IDs.
- Modify `lib/src/lower/expression.rs`: update intrinsic/custom-operator construction and any `Call`/`Var` constructors touched by the new HIR shapes.
- Modify `lib/src/lower/traits/conformance.rs`, `lib/src/infer/mod.rs`, `lib/src/type_services/**`, `lib/src/selection/**`: update HIR traversal match arms for changed shapes without expanding their architectural responsibility.
- Modify `lib/src/mir/**`, `lib/src/mono/**`, `lib/src/dce.rs`, `lib/src/codegen/**`: adjust consumers to compile with the new sidecars; use local IDs opportunistically only where this preserves current behavior with smaller code.
- Modify `lib/src/products.rs`: validate/remap new HIR reference sidecars and bump product format version.
- Modify `lib/src/crate_artifact/load.rs`: validate artifact-loaded sidecars after product remapping.
- Modify `rock-shared/src/sysroot.rs`: keep product artifact format version aligned.
- Modify docs at completion: `docs/superpowers/plans/master-audit-checklist.md`, `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, and this plan's verification notes.

---

## Task 1: Add HIR Reference Identity Types

**Files:**
- Modify: `lib/src/ids.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/hir/type_ids.rs`
- Modify: `lib/src/products.rs`
- Modify: `rock-shared/src/sysroot.rs`

- [x] **Step 1: Write the failing HIR model tests**

Add these tests to `lib/src/hir/mod.rs` inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn hir_var_target_can_identify_local_bindings() {
    let local = crate::ids::HirLocalId(7);
    let target = HirVarTarget::Local(local);

    assert_eq!(target, HirVarTarget::Local(crate::ids::HirLocalId(7)));
}

#[test]
fn hir_call_target_can_identify_direct_function_calls() {
    let function_id = def_id(44);
    let target = HirCallTarget::Function(function_id);

    assert_eq!(target, HirCallTarget::Function(function_id));
}
```

- [x] **Step 2: Run the failing HIR model tests**

Run: `cargo test -p rock-lib hir_var_target_can_identify_local_bindings`

Expected: FAIL because `HirLocalId` or `HirVarTarget::Local` does not exist.

Run: `cargo test -p rock-lib hir_call_target_can_identify_direct_function_calls`

Expected: FAIL because `HirCallTarget` does not exist.

- [x] **Step 3: Add the new ID type**

In `lib/src/ids.rs`, add `HirLocalId` with the existing ID macro group:

```rust
define_id!(HirLocalId);
```

Place it after `InstanceId` so it is clearly a HIR/body identity, not a definition identity.

- [x] **Step 4: Add HIR local and call target types**

In `lib/src/hir/mod.rs`, update the ID import:

```rust
use crate::ids::{AssocTypeId, DefId, FieldId, HirLocalId, InstanceId, TypeVarId, VariantId};
```

Add these definitions near `HirVarTarget`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HirLocalRef {
    pub id: HirLocalId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirCallTarget {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
    Local(HirLocalId),
    Intrinsic(String),
}
```

Extend `HirVarTarget`:

```rust
pub enum HirVarTarget {
    Local(HirLocalId),
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
}
```

- [x] **Step 5: Extend HIR binding and call shapes**

In `lib/src/hir/mod.rs`, change these structures and enum variants:

```rust
pub struct HirParam {
    pub name: String,
    pub local_id: HirLocalId,
    pub ty: Type,
    pub mutable: bool,
    pub is_ref: bool,
}

pub struct HirClosureCapture {
    pub name: String,
    pub local_id: HirLocalId,
    pub kind: HirClosureCaptureKind,
    pub ty: Type,
}

pub enum HirStmt {
    Let {
        name: String,
        local_id: HirLocalId,
        ty: Type,
        value: HirExpr,
        mutable: bool,
    },
    Expr(HirExpr),
    Return(Option<HirExpr>),
    Break(Option<HirExpr>),
    Continue,
}

pub enum HirExprKind {
    Call(Box<HirExpr>, Vec<HirExpr>, Option<HirCallTarget>),
    For {
        var: String,
        local_id: HirLocalId,
        iter: Box<HirExpr>,
        body: HirBlock,
    },
}

pub enum HirPattern {
    Binding {
        name: String,
        local_id: HirLocalId,
        mutable: bool,
    },
}
```

Keep every other existing variant unchanged.

- [x] **Step 6: Update product artifact format constants**

In `lib/src/products.rs`, change:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 21;
```

In `rock-shared/src/sysroot.rs`, change:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 21;
```

In `lib/src/products.rs`, update the shared-contract assertion from `20` to `21`.

- [x] **Step 7: Run the focused model tests**

Run: `cargo test -p rock-lib hir_var_target_can_identify_local_bindings`

Expected: PASS.

Run: `cargo test -p rock-lib hir_call_target_can_identify_direct_function_calls`

Expected: PASS.

- [x] **Step 8: Commit Task 1**

Run:

```bash
git add lib/src/ids.rs lib/src/hir/mod.rs lib/src/hir/type_ids.rs lib/src/products.rs rock-shared/src/sysroot.rs
git commit -m "add hir semantic reference id types"
```

---

## Task 2: Add Scope Binding Metadata And Local ID Allocation

**Files:**
- Modify: `lib/src/lower/scope.rs`
- Modify: `lib/src/lower/mod.rs`
- Test: `lib/src/lower/scope.rs`

- [x] **Step 1: Write failing scope tests**

Add this test module content to `lib/src/lower/scope.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ids::HirLocalId;
    use crate::types::Type;

    #[test]
    fn scope_distinguishes_local_bindings_from_aliases() {
        let mut scope = Scope::new();
        scope.define_local("value".to_string(), Type::I64, true, HirLocalId(3));
        scope.define_alias("println".to_string(), Type::Function(vec![Type::I64], Box::new(Type::I64)), false);

        let value = scope.lookup("value").expect("local should be in scope");
        assert_eq!(value.ty, Type::I64);
        assert!(value.mutable);
        assert_eq!(value.local_id, Some(HirLocalId(3)));
        assert!(!value.is_alias);

        let alias = scope.lookup("println").expect("alias should be in scope");
        assert!(alias.local_id.is_none());
        assert!(alias.is_alias);
    }

    #[test]
    fn scope_shadowing_returns_innermost_local_id() {
        let mut scope = Scope::new();
        scope.define_local("x".to_string(), Type::I64, false, HirLocalId(1));
        scope.push();
        scope.define_local("x".to_string(), Type::Bool, false, HirLocalId(2));

        assert_eq!(scope.lookup("x").unwrap().local_id, Some(HirLocalId(2)));
        scope.pop();
        assert_eq!(scope.lookup("x").unwrap().local_id, Some(HirLocalId(1)));
    }
}
```

- [x] **Step 2: Run failing scope tests**

Run: `cargo test -p rock-lib scope_distinguishes_local_bindings_from_aliases`

Expected: FAIL because `ScopeBinding` and `define_local` do not exist.

- [x] **Step 3: Replace tuple scope entries with `ScopeBinding`**

In `lib/src/lower/scope.rs`, replace the struct and methods with this shape:

```rust
use std::collections::HashMap;

use crate::ids::HirLocalId;
use crate::types::Type;

#[derive(Clone, Debug, PartialEq)]
pub struct ScopeBinding {
    pub ty: Type,
    pub mutable: bool,
    pub local_id: Option<HirLocalId>,
    pub is_alias: bool,
}

#[derive(Clone)]
pub struct Scope {
    vars: Vec<HashMap<String, ScopeBinding>>,
}

impl Scope {
    pub fn new() -> Self {
        Self { vars: vec![HashMap::new()] }
    }

    pub fn push(&mut self) {
        self.vars.push(HashMap::new());
    }

    pub fn pop(&mut self) {
        self.vars.pop();
    }

    pub fn define(&mut self, name: String, ty: Type, mutable: bool) {
        self.define_local_without_id(name, ty, mutable);
    }

    pub fn define_local_without_id(&mut self, name: String, ty: Type, mutable: bool) {
        self.insert(
            name,
            ScopeBinding {
                ty,
                mutable,
                local_id: None,
                is_alias: false,
            },
        );
    }

    pub fn define_local(&mut self, name: String, ty: Type, mutable: bool, local_id: HirLocalId) {
        self.insert(
            name,
            ScopeBinding {
                ty,
                mutable,
                local_id: Some(local_id),
                is_alias: false,
            },
        );
    }

    pub fn define_alias(&mut self, name: String, ty: Type, mutable: bool) {
        self.insert(
            name,
            ScopeBinding {
                ty,
                mutable,
                local_id: None,
                is_alias: true,
            },
        );
    }

    fn insert(&mut self, name: String, binding: ScopeBinding) {
        if let Some(scope) = self.vars.last_mut() {
            scope.insert(name, binding);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&ScopeBinding> {
        for scope in self.vars.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry);
            }
        }
        None
    }

    pub fn binding_scope_index(&self, name: &str) -> Option<usize> {
        for (index, scope) in self.vars.iter().enumerate().rev() {
            if scope.contains_key(name) {
                return Some(index);
            }
        }
        None
    }

    pub fn binding_is_alias(&self, name: &str) -> bool {
        self.lookup(name).is_some_and(|binding| binding.is_alias)
    }
}
```

- [x] **Step 4: Add local ID allocation state to `Lowerer`**

In `lib/src/lower/mod.rs`, import `HirLocalId` and add this field to `Lowerer`:

```rust
pub(crate) local_ids: IdGen<HirLocalId>,
```

Initialize it in `Lowerer::new`, `Lowerer::with_options`, and `Lowerer::from_declarations`:

```rust
local_ids: IdGen::<HirLocalId>::new(),
```

Add helper methods inside `impl Lowerer`:

```rust
pub(crate) fn fresh_local_id(&mut self) -> HirLocalId {
    self.local_ids.fresh()
}

pub(crate) fn reset_local_ids_for_body(&mut self) {
    self.local_ids = IdGen::<HirLocalId>::new();
}
```

- [x] **Step 5: Update scope consumers to field access**

Replace tuple destructuring of scope entries throughout `lib/src/lower/**`:

```rust
let Some(binding) = self.scope.lookup(name) else { ... };
let ty = binding.ty.clone();
let mutable = binding.mutable;
let binding_is_alias = binding.is_alias;
let local_id = binding.local_id;
```

Do not keep `let Some((ty, mutable)) = ...` patterns.

- [x] **Step 6: Run focused scope tests**

Run: `cargo test -p rock-lib scope_distinguishes_local_bindings_from_aliases`

Expected: PASS.

Run: `cargo test -p rock-lib scope_shadowing_returns_innermost_local_id`

Expected: PASS.

- [x] **Step 7: Commit Task 2**

Run:

```bash
git add lib/src/lower/scope.rs lib/src/lower/mod.rs lib/src/lower
git commit -m "track hir local ids in lowering scope"
```

---

## Task 3: Allocate IDs For Parameters, Lets, Reads, And Assignments

**Files:**
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/statement.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/hir/mod.rs`
- Test: `lib/src/lower/paths.rs`

- [x] **Step 1: Write failing local-reference lowering test**

Add this test to `lib/src/lower/paths.rs` tests:

```rust
#[test]
fn local_reads_resolve_to_shadowing_local_ids() {
    let mut lowerer = Lowerer::new();
    let outer = lowerer.fresh_local_id();
    lowerer.scope.define_local("x".to_string(), Type::I64, false, outer);
    lowerer.scope.push();
    let inner = lowerer.fresh_local_id();
    lowerer.scope.define_local("x".to_string(), Type::Bool, false, inner);

    let expr = lowerer.lower_identifier_path(&identifier_path(&["x"]));

    match expr.kind {
        HirExprKind::ResolvedVar(HirVarRef { target: HirVarTarget::Local(id), name }) => {
            assert_eq!(name, "x");
            assert_eq!(id, inner);
        }
        other => panic!("expected resolved local var, got {other:?}"),
    }
}
```

Also add this parameter-read test in the same module:

```rust
#[test]
fn parameter_reads_resolve_to_parameter_local_ids() {
    let resolved = compile_source_to_resolved_hir_for_test(
        r#"
identity = x -> x
"#,
    );
    let (_, function) = resolved.program.function_by_name("identity").unwrap();
    let param_id = function.params[0].local_id;

    let var = find_first_resolved_var(&function.body).unwrap();
    assert_eq!(var.name, "x");
    assert_eq!(var.target, HirVarTarget::Local(param_id));
}
```

This uses existing helpers in `lib/src/lower/paths.rs` tests: `identifier_path`, `compile_source_to_resolved_hir_for_test`, and `find_first_resolved_var`.

Add this reassignment test to `lib/src/lower/statement.rs` tests and extend the test imports to include `Assignment`, `AssignmentLHS`, `Ident`, `IdentPattern`, `Pattern`, and `PatternKind`:

```rust
fn ident(name: &str) -> Ident {
    Ident {
        name: name.to_string(),
        span: Span::default(),
    }
}

fn binding_pattern(name: &str) -> Pattern {
    Pattern {
        binding: Some(ident(name)),
        kind: PatternKind::Ident(IdentPattern {
            name: ident(name),
            mut_: false,
        }),
    }
}

#[test]
fn reassignment_targets_shadowing_local_id() {
    let mut lowerer = Lowerer::new();
    let outer = lowerer.fresh_local_id();
    lowerer.scope.define_local("x".to_string(), Type::I64, true, outer);
    lowerer.scope.push();
    let inner = lowerer.fresh_local_id();
    lowerer.scope.define_local("x".to_string(), Type::I64, true, inner);

    let stmt = lowerer.lower_assignment(&Assignment {
        lhs: AssignmentLHS::Pattern {
            pattern: binding_pattern("x"),
            type_annotation: None,
        },
        rhs: number_expr(2),
    });

    let HirStmt::Expr(expr) = stmt else {
        panic!("expected assignment expression statement");
    };
    let HirExprKind::Assign(lhs, _) = expr.kind else {
        panic!("expected assignment expression");
    };
    match lhs.kind {
        HirExprKind::ResolvedVar(HirVarRef { target: HirVarTarget::Local(id), name }) => {
            assert_eq!(name, "x");
            assert_eq!(id, inner);
        }
        other => panic!("expected assignment lhs local ref, got {other:?}"),
    }
}
```

- [x] **Step 2: Run the failing local-reference test**

Run: `cargo test -p rock-lib local_reads_resolve_to_shadowing_local_ids`

Expected: FAIL because local reads still lower to `HirExprKind::Var`.

Run: `cargo test -p rock-lib parameter_reads_resolve_to_parameter_local_ids`

Expected: FAIL because `HirParam` has no local ID or parameter reads stay as `Var`.

Run: `cargo test -p rock-lib reassignment_targets_shadowing_local_id`

Expected: FAIL because assignment LHS lowering still uses `HirExprKind::Var`.

- [x] **Step 3: Allocate parameter local IDs**

In `lib/src/lower/function.rs`, update `build_self_param`, `lower_function_decl_header`, and `lower_function_decl_header_with_sig_and_id` so each `HirParam` includes a `local_id` allocated with `fresh_local_id()`.

Use this construction pattern for every parameter:

```rust
let local_id = self.fresh_local_id();
self.scope.define_local(name.clone(), ty.clone(), mutable, local_id);
HirParam {
    name,
    local_id,
    ty,
    mutable,
    is_ref,
}
```

For implicit `self`, use:

```rust
let local_id = self.fresh_local_id();
HirParam {
    name: "self".to_string(),
    local_id,
    ty: self_ty,
    mutable: matches!(self_receiver, ast::SelfReceiverMode::Mut),
    is_ref: false,
}
```

- [x] **Step 4: Reset local IDs per body**

At the beginning of `lower_function_body_qualified` in `lib/src/lower/bodies.rs`, before parameters enter scope, add:

```rust
self.reset_local_ids_for_body();
```

Ensure body lowering re-defines parameter locals in scope using the IDs stored on `func.params`, not newly allocated IDs.

- [x] **Step 5: Allocate let and assignment IDs**

In `lib/src/lower/statement.rs`, change `HirStmt::Let` construction to allocate and store a local ID:

```rust
let local_id = self.fresh_local_id();
self.scope.define_local(name.clone(), ty.clone(), mutable, local_id);
HirStmt::Let {
    name,
    local_id,
    ty,
    value: rhs,
    mutable,
}
```

For reassignment to an existing local, build the lhs as a resolved local variable when `scope.lookup(&name).and_then(|binding| binding.local_id)` is present:

```rust
let lhs_kind = self
    .scope
    .lookup(&name)
    .and_then(|binding| binding.local_id)
    .map(|local_id| {
        HirExprKind::ResolvedVar(HirVarRef {
            name: name.clone(),
            target: HirVarTarget::Local(local_id),
        })
    })
    .unwrap_or_else(|| HirExprKind::Var(name.clone()));
```

- [x] **Step 6: Resolve local reads in path lowering**

In `lib/src/lower/paths.rs`, when a single-segment name resolves to a scope binding with `Some(local_id)`, return:

```rust
return HirExpr {
    ty,
    kind: HirExprKind::ResolvedVar(HirVarRef {
        name: hir_name,
        target: HirVarTarget::Local(local_id),
    }),
    span,
};
```

Keep top-level function/extern alias behavior as `Function`/`Extern`. Keep unresolved/error recovery as `HirExprKind::Var`.

- [x] **Step 7: Update HIR consumers for `HirVarTarget::Local`**

In `lib/src/products.rs`, `lib/src/mir/builder/**`, `lib/src/mono/**`, `lib/src/dce.rs`, `lib/src/codegen/**`, and HIR tests, update `match HirVarTarget` arms with:

```rust
HirVarTarget::Local(_) => {}
```

Keep current name-based behavior in consumers for now, except where a consumer already has a direct local-ID lookup. This preserves behavior without taking on later roadmap tasks.

- [x] **Step 8: Run focused local reference tests**

Run: `cargo test -p rock-lib local_reads_resolve_to_shadowing_local_ids`

Expected: PASS.

Run: `cargo test -p rock-lib parameter_reads_resolve_to_parameter_local_ids`

Expected: PASS.

Run: `cargo test -p rock-lib reassignment_targets_shadowing_local_id`

Expected: PASS.

Run: `cargo test -p rock-lib lower_function_sig`

Expected: PASS, confirming signature lowering still handles generics after `HirParam` changes.

- [x] **Step 9: Commit Task 3**

Run:

```bash
git add lib/src/hir/mod.rs lib/src/lower/function.rs lib/src/lower/statement.rs lib/src/lower/paths.rs lib/src/lower/bodies.rs lib/src/products.rs lib/src/mir lib/src/mono lib/src/dce.rs lib/src/codegen
git commit -m "resolve hir local variable references by id"
```

---

## Task 4: Add IDs For Pattern Bindings, Tuple Temps, Loops, And Captures

**Files:**
- Modify: `lib/src/lower/control_flow/pattern.rs`
- Modify: `lib/src/lower/statement.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/control_flow/loops.rs`
- Modify: `lib/src/hir/mod.rs`
- Test: `lib/src/lower/control_flow/pattern.rs`
- Test: `lib/src/lower/statement.rs`
- Test: `lib/src/lower/paths.rs`

- [x] **Step 1: Write failing pattern binding test**

Add this test in `lib/src/lower/control_flow/pattern.rs` tests:

```rust
#[test]
fn pattern_binding_allocates_local_id_and_defines_scope() {
    let mut lowerer = Lowerer::new();
    let pattern = ident_pattern("value", false);

    let hir = lowerer.lower_pattern(&pattern, &Type::I64);

    match hir {
        HirPattern::Binding { name, local_id, mutable } => {
            assert_eq!(name, "value");
            assert!(!mutable);
            assert_eq!(lowerer.scope.lookup("value").unwrap().local_id, Some(local_id));
        }
        other => panic!("expected binding pattern, got {other:?}"),
    }
}
```

Add these helpers in the `lib/src/lower/control_flow/pattern.rs` test module before the test:

```rust
fn ident(name: &str) -> ast::Ident {
    ast::Ident {
        name: name.to_string(),
        span: Default::default(),
    }
}

fn ident_pattern(name: &str, mutable: bool) -> ast::Pattern {
    ast::Pattern {
        binding: Some(ident(name)),
        kind: ast::PatternKind::Ident(ast::IdentPattern {
            name: ident(name),
            mut_: mutable,
        }),
    }
}
```

- [x] **Step 2: Run the failing pattern binding test**

Run: `cargo test -p rock-lib pattern_binding_allocates_local_id_and_defines_scope`

Expected: FAIL because `HirPattern::Binding` has no local ID or pattern lowering does not allocate one.

- [x] **Step 3: Allocate pattern binding IDs**

In `lib/src/lower/control_flow/pattern.rs`, replace binding construction with:

```rust
let ty = expected_ty.clone();
let local_id = self.fresh_local_id();
self.scope
    .define_local(ident_pat.name.name.clone(), ty, ident_pat.mut_, local_id);
HirPattern::Binding {
    name: ident_pat.name.name.clone(),
    local_id,
    mutable: ident_pat.mut_,
}
```

Update all pattern traversal match arms from `HirPattern::Binding(_, _)` to `HirPattern::Binding { .. }`.

- [x] **Step 4: Write failing tuple destructuring test**

Add this test to `lib/src/lower/statement.rs` tests:

```rust
#[test]
fn tuple_destructuring_assigns_ids_to_temp_and_bindings() {
    let mut lowerer = Lowerer::new();
    let stmts = lowerer.lower_tuple_destructuring(
        &[binding_pattern("left"), binding_pattern("right")],
        &tuple_expr(vec![int_expr(1), int_expr(2)]),
    );

    let HirStmt::Let { name: temp_name, local_id: temp_id, .. } = &stmts[0] else {
        panic!("expected tuple temp let");
    };
    assert!(temp_name.starts_with("__tuple_tmp_"));

    let HirStmt::Let { name: left_name, local_id: left_id, value, .. } = &stmts[1] else {
        panic!("expected left binding let");
    };
    assert_eq!(left_name, "left");
    assert_ne!(temp_id, left_id);
    match &value.kind {
        HirExprKind::TupleIndex(base, 0) => match &base.kind {
            HirExprKind::ResolvedVar(HirVarRef { target: HirVarTarget::Local(id), .. }) => {
                assert_eq!(id, temp_id);
            }
            other => panic!("expected tuple temp local ref, got {other:?}"),
        },
        other => panic!("expected tuple index, got {other:?}"),
    }
}
```

Add these helpers in the `lib/src/lower/statement.rs` test module before the tuple destructuring test:

```rust
fn ident(name: &str) -> Ident {
    Ident {
        name: name.to_string(),
        span: Span::default(),
    }
}

fn binding_pattern(name: &str) -> Pattern {
    Pattern {
        binding: Some(ident(name)),
        kind: PatternKind::Ident(IdentPattern {
            name: ident(name),
            mut_: false,
        }),
    }
}

fn int_expr(value: u64) -> Expression {
    number_expr(value)
}

fn tuple_expr(elements: Vec<Expression>) -> Expression {
    Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
        operand: Operand::Tuple(crate::ast::Tuple { elements }),
        secondaries: None,
        type_annotation: None,
    }))
}
```

- [x] **Step 5: Run the failing tuple destructuring test**

Run: `cargo test -p rock-lib tuple_destructuring_assigns_ids_to_temp_and_bindings`

Expected: FAIL because tuple temp reads still use `Var(String)` or lets do not have IDs.

- [x] **Step 6: Update tuple destructuring**

In `lib/src/lower/statement.rs`, allocate a temp local ID and define the temp with `define_local`. Build tuple-index bases as `ResolvedVar(Local(temp_id))`:

```rust
let tmp_local_id = self.fresh_local_id();
self.scope.define_local(tmp_name.clone(), tmp_ty.clone(), false, tmp_local_id);
stmts.push(HirStmt::Let {
    name: tmp_name.clone(),
    local_id: tmp_local_id,
    ty: tmp_ty.clone(),
    value: rhs_expr,
    mutable: false,
});
```

For each user binding, allocate `binding_local_id`, define it in scope, and store it on the `HirStmt::Let`.

- [x] **Step 7: Add loop variable local IDs**

First add this failing test module to `lib/src/lower/control_flow/loops.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{
        Block, Expression, Ident, IdentOrNumber, IdentOrType, IdentifierPath, Literal,
        LiteralKind, Operand, Pattern, PatternKind, PrimaryExpr, SecondaryExpr, Statement,
        UnaryExpr,
    };
    use crate::hir::{HirExprKind, HirStmt, HirVarRef, HirVarTarget};
    use crate::lexer::Span;
    use crate::lower::Lowerer;

    fn ident(name: &str) -> Ident {
        Ident {
            name: name.to_string(),
            span: Span::default(),
        }
    }

    fn binding_pattern(name: &str) -> Pattern {
        Pattern {
            binding: Some(ident(name)),
            kind: PatternKind::Ident(crate::ast::IdentPattern {
                name: ident(name),
                mut_: false,
            }),
        }
    }

    fn number_expr(value: u64) -> Expression {
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::Literal(Literal {
                kind: LiteralKind::Number(value),
                span: Span::default(),
            }),
            secondaries: None,
            type_annotation: None,
        }))
    }

    fn range_expr(start: u64, end: u64) -> Expression {
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::Literal(Literal {
                kind: LiteralKind::Number(start),
                span: Span::default(),
            }),
            secondaries: Some(vec![SecondaryExpr::DoubleDot(IdentOrNumber::Number(end))]),
            type_annotation: None,
        }))
    }

    fn var_expr(name: &str) -> Expression {
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::Ident(IdentifierPath {
                path: vec![IdentOrType::Ident(ident(name))],
            }),
            secondaries: None,
            type_annotation: None,
        }))
    }

    #[test]
    fn for_loop_variable_read_uses_loop_local_id() {
        let mut lowerer = Lowerer::new();
        let loop_expr = crate::ast::Loop::For(
            binding_pattern("item"),
            range_expr(0, 2),
            Block {
                statements: vec![Statement::Expression(var_expr("item"))],
            },
        );

        let hir = lowerer.lower_loop(&loop_expr);

        let HirExprKind::For { local_id, body, .. } = hir.kind else {
            panic!("expected for loop");
        };
        let HirStmt::Expr(expr) = &body.stmts[0] else {
            panic!("expected loop body expression");
        };
        match &expr.kind {
            HirExprKind::ResolvedVar(HirVarRef { target: HirVarTarget::Local(id), name }) => {
                assert_eq!(name, "item");
                assert_eq!(*id, local_id);
            }
            other => panic!("expected loop local read, got {other:?}"),
        }
    }
}
```

Run: `cargo test -p rock-lib for_loop_variable_read_uses_loop_local_id`

Expected: FAIL because `HirExprKind::For` has no local ID and loop-body reads still lower through names.

Then in `lib/src/lower/control_flow/loops.rs`, change `HirExprKind::For` construction to allocate a local ID before lowering the loop body:

```rust
let local_id = self.fresh_local_id();
self.scope.define_local(var_name.clone(), element_ty.clone(), false, local_id);
HirExprKind::For {
    var: var_name,
    local_id,
    iter: Box::new(iter),
    body,
}
```

Update all `HirExprKind::For` match arms to include `local_id: _`.

- [x] **Step 8: Add closure capture local IDs**

Add this failing test to `lib/src/lower/paths.rs` tests:

```rust
#[test]
fn lambda_capture_records_captured_local_id() {
    let mut lowerer = Lowerer::new();
    let captured_id = lowerer.fresh_local_id();
    lowerer.scope.define_local("base".to_string(), Type::I64, false, captured_id);

    let lambda = LambdaDecl {
        parameters: vec![ident_pattern("x")],
        body: Block {
            statements: vec![Statement::Expression(var_expr("base"))],
        },
        arrow_kind: LambdaArrowKind::Normal,
    };

    let hir = lowerer.lower_lambda(&lambda);
    match hir.kind {
        HirExprKind::Lambda { captures, .. } => {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].name, "base");
            assert_eq!(captures[0].local_id, captured_id);
        }
        other => panic!("expected lambda, got {other:?}"),
    }
}
```

Run: `cargo test -p rock-lib lambda_capture_records_captured_local_id`

Expected: FAIL because `HirClosureCapture` has no local ID.

In `lib/src/lower/paths.rs`, update `collect_lambda_captures` so capture lookup reads `binding.local_id`. Only push a capture when `binding.local_id` is `Some(local_id)`:

```rust
captures.push(HirClosureCapture {
    name,
    local_id,
    kind,
    ty: binding.ty.clone(),
});
```

Top-level function aliases must continue to be skipped as captures.

- [x] **Step 9: Run focused binding/capture tests**

Run: `cargo test -p rock-lib pattern_binding_allocates_local_id_and_defines_scope`

Expected: PASS.

Run: `cargo test -p rock-lib tuple_destructuring_assigns_ids_to_temp_and_bindings`

Expected: PASS.

Run: `cargo test -p rock-lib for_loop_variable_read_uses_loop_local_id`

Expected: PASS.

Run: `cargo test -p rock-lib lambda_capture_records_captured_local_id`

Expected: PASS.

Run: `cargo test -p rock-lib closure_capture`

Expected: existing closure capture tests PASS after capture struct changes.

- [x] **Step 10: Commit Task 4**

Run:

```bash
git add lib/src/hir/mod.rs lib/src/lower/control_flow/pattern.rs lib/src/lower/control_flow/loops.rs lib/src/lower/statement.rs lib/src/lower/paths.rs lib/src/lower/control_flow/secondary.rs lib/src/mir lib/src/products.rs
git commit -m "assign hir local ids to bindings and captures"
```

---

## Task 5: Add Direct Call Target Sidecars

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: HIR consumers matching `HirExprKind::Call`
- Test: `lib/src/lower/control_flow/secondary.rs`

- [x] **Step 1: Write failing direct-call target test**

Extend the `crate::hir` test import in `lib/src/lower/control_flow/secondary.rs` to include `HirCallTarget` and `HirExtern`. Add these tests in the same test module:

```rust
#[test]
fn direct_function_call_records_call_target() {
    let mut lowerer = Lowerer::new();
    let function_id = DefId::new(CrateId(0), LocalDefId(88));
    lowerer.functions.insert(
        "answer".to_string(),
        test_function(function_id, "answer", Type::I64),
    );
    lowerer.scope.define_alias(
        "answer".to_string(),
        Type::Function(Vec::new(), Box::new(Type::I64)),
        false,
    );

    let expr = call_expr("answer", Vec::new());
    let hir = lowerer.lower_expression(&expr);

    match hir.kind {
        HirExprKind::Call(_, _, Some(HirCallTarget::Function(id))) => assert_eq!(id, function_id),
        other => panic!("expected function call target, got {other:?}"),
    }
}

#[test]
fn direct_extern_call_records_call_target() {
    let mut lowerer = Lowerer::new();
    let extern_id = DefId::new(CrateId(0), LocalDefId(89));
    lowerer.externs.push(HirExtern {
        id: extern_id,
        name: "puts".to_string(),
        params: vec![Type::I32],
        ret: Type::I32,
        variadic: false,
    });
    lowerer.scope.define_alias(
        "puts".to_string(),
        Type::Function(vec![Type::I32], Box::new(Type::I32)),
        false,
    );

    let expr = call_expr("puts", vec![number_expression(0)]);
    let hir = lowerer.lower_expression(&expr);

    match hir.kind {
        HirExprKind::Call(_, _, Some(HirCallTarget::Extern(id))) => assert_eq!(id, extern_id),
        other => panic!("expected extern call target, got {other:?}"),
    }
}

#[test]
fn local_function_value_call_records_call_target() {
    let mut lowerer = Lowerer::new();
    let local_id = lowerer.fresh_local_id();
    lowerer.scope.define_local(
        "callback".to_string(),
        Type::Function(Vec::new(), Box::new(Type::I64)),
        false,
        local_id,
    );

    let expr = call_expr("callback", Vec::new());
    let hir = lowerer.lower_expression(&expr);

    match hir.kind {
        HirExprKind::Call(_, _, Some(HirCallTarget::Local(id))) => assert_eq!(id, local_id),
        other => panic!("expected local function-value call target, got {other:?}"),
    }
}
```

Use the existing `test_function` helper in this test module. Add this `call_expr` helper next to `ident_expression`:

```rust
fn call_expr(name: &str, args: Vec<Expression>) -> Expression {
    Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
        operand: Operand::Ident(IdentifierPath {
            path: vec![IdentOrType::Ident(Ident {
                name: name.to_string(),
                span: Span::default(),
            })],
        }),
        secondaries: Some(vec![SecondaryExpr::Arguments(
            args.into_iter()
                .map(|arg| crate::ast::Argument { arg })
                .collect(),
        )]),
        type_annotation: None,
    }))
}
```

- [x] **Step 2: Run failing direct-call target test**

Run: `cargo test -p rock-lib direct_function_call_records_call_target`

Expected: FAIL because `HirExprKind::Call` has no call target or the target is `None`.

Run: `cargo test -p rock-lib direct_extern_call_records_call_target`

Expected: FAIL because extern calls do not record `HirCallTarget::Extern`.

Run: `cargo test -p rock-lib local_function_value_call_records_call_target`

Expected: FAIL because local function-value calls do not record `HirCallTarget::Local`.

- [x] **Step 3: Add call target derivation helper**

In `lib/src/lower/control_flow/secondary.rs`, add a helper on `Lowerer`:

```rust
fn call_target_for_callee(&self, callee: &HirExpr) -> Option<HirCallTarget> {
    match &callee.kind {
        HirExprKind::ResolvedVar(reference) => match &reference.target {
            HirVarTarget::Function(id) => Some(HirCallTarget::Function(*id)),
            HirVarTarget::Extern(id) => Some(HirCallTarget::Extern(*id)),
            HirVarTarget::Instance(id) => Some(HirCallTarget::Instance(*id)),
            HirVarTarget::Local(id) => Some(HirCallTarget::Local(*id)),
        },
        HirExprKind::Var(name) if is_intrinsic_name(name) => {
            Some(HirCallTarget::Intrinsic(name.clone()))
        }
        _ => None,
    }
}
```

- [x] **Step 4: Store call targets on call nodes**

Every construction of `HirExprKind::Call(func, args)` becomes:

```rust
let target = self.call_target_for_callee(&expr);
HirExprKind::Call(Box::new(expr), hir_args, target)
```

For intrinsic lowering that returns `HirExprKind::Intrinsic`, no `Call` sidecar is needed because the intrinsic node is already explicit.

- [x] **Step 5: Update all call match arms**

Update every pattern matching call expressions from:

```rust
HirExprKind::Call(func, args)
```

to:

```rust
HirExprKind::Call(func, args, target)
```

Use `target` where validation/remapping needs it and `_` elsewhere.

- [x] **Step 6: Run focused call tests**

Run: `cargo test -p rock-lib direct_function_call_records_call_target`

Expected: PASS.

Run: `cargo test -p rock-lib direct_extern_call_records_call_target`

Expected: PASS.

Run: `cargo test -p rock-lib local_function_value_call_records_call_target`

Expected: PASS.

Run: `cargo test -p rock-lib unresolved_instance_callable_does_not_fallback_to_name`

Expected: PASS, proving Task 15 behavior was not regressed.

- [x] **Step 7: Commit Task 5**

Run:

```bash
git add lib/src/hir/mod.rs lib/src/lower/control_flow/secondary.rs lib/src/lower/expression.rs lib/src/lower/paths.rs lib/src/products.rs lib/src/crate_artifact lib/src/mir lib/src/mono lib/src/dce.rs lib/src/codegen
git commit -m "record hir direct call targets"
```

---

## Task 6: Tighten Aggregate, Field, Pattern, And Method Sidecar Invariants

**Files:**
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/control_flow/pattern.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/paths.rs`
- Test: `lib/src/lower/control_flow/pattern.rs`
- Test: `lib/src/lower/control_flow/secondary.rs`

- [x] **Step 1: Write failing aggregate sidecar tests**

Add this test to `lib/src/lower/paths.rs` tests, next to `lowers_known_struct_literal_fields_with_resolved_field_locations`:

```rust
#[test]
fn struct_literal_resolved_fields_all_have_locations() {
    let mut lowerer = Lowerer::new();
    lowerer.structs.insert("Point".to_string(), point_struct());
    let point_id = def_id(10);
    let x_field_id = FieldId(0);
    let expr = lowerer.lower_instance(&ast::Instance {
        name: ast::TypePath {
            path: vec![IdentOrType::Ident(ident("Point"))],
        },
        fields: HashMap::from([(ident("x"), int_expr("1")), (ident("y"), int_expr("2"))]),
    });

    match expr.kind {
        HirExprKind::StructLiteral(_, Some(id), fields) => {
            assert_eq!(id, point_id);
            assert_eq!(fields[0].field.as_ref().unwrap().owner, point_id);
            assert_eq!(fields[0].field.as_ref().unwrap().field_id, x_field_id);
        }
        other => panic!("expected resolved struct literal, got {other:?}"),
    }
}
```

This uses existing helpers in `lib/src/lower/paths.rs` tests: `point_struct`, `def_id`, `ident`, and `int_expr`.

- [x] **Step 2: Run failing aggregate sidecar test**

Run: `cargo test -p rock-lib struct_literal_resolved_fields_all_have_locations`

Expected: FAIL if any resolved field sidecar is missing.

- [x] **Step 3: Make resolved struct/enum expressions always populate sidecars**

In `lib/src/lower/paths.rs`, every branch that successfully resolves a struct must construct:

```rust
HirExprKind::StructLiteral(struct_name, Some(struct_id), fields)
```

Every field in `fields` must have:

```rust
field: Some(HirFieldLocation {
    owner: struct_id,
    field_id: field.id,
    name: field.name.clone(),
})
```

Every branch that successfully resolves an enum variant must construct `Some(HirVariantLocation { owner: enum_id, variant_id, name })`.

- [x] **Step 4: Make resolved struct/enum patterns always populate sidecars**

In `lib/src/lower/control_flow/pattern.rs`, resolved struct and enum patterns must use `Some(struct_id)` and `Some(HirVariantLocation { ... })`. Field patterns for named fields must use `Some(HirFieldLocation { ... })`.

- [x] **Step 5: Preserve method targets for selected calls**

In `lib/src/lower/control_flow/secondary.rs` and `lib/src/lower/expression.rs`, audit `HirExprKind::MethodCall` construction. Whenever a selected `HirMethodCallTarget` exists in `SelectionResult` or direct impl lookup, pass `Some(target)`. Only leave `None` when lowering genuinely has no selected target and existing later-task compatibility owns targetless handling.

- [x] **Step 6: Run focused sidecar tests**

Run: `cargo test -p rock-lib struct_literal_resolved_fields_all_have_locations`

Expected: PASS.

Run: `cargo test -p rock-lib pattern`

Expected: existing parser/lower pattern tests PASS.

Run: `cargo test -p rock-lib selection_service_selects_concrete_impl_method_by_identity`

Expected: PASS.

- [x] **Step 7: Commit Task 6**

Run:

```bash
git add lib/src/lower/paths.rs lib/src/lower/control_flow/pattern.rs lib/src/lower/control_flow/secondary.rs lib/src/lower/expression.rs lib/src/lower/traits/conformance.rs
git commit -m "complete hir aggregate and method reference sidecars"
```

---

## Task 7: Validate And Remap New Sidecars In Products And Artifacts

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `rock-shared/src/sysroot.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [x] **Step 1: Write failing product sidecar validation test**

Add this test to `lib/src/products.rs` tests:

```rust
#[test]
#[should_panic(expected = "compiler product emission received invalid current-crate DefId")]
fn compiler_products_reject_call_targets_with_invalid_current_crate_ids() {
    let function_id = DefId::new(CrateId(0), LocalDefId(1));
    let invalid_target = DefId::new(CrateId(u32::MAX), LocalDefId(99));
    let mut function = test_function(function_id, "caller", Vec::new());
    function.body.stmts.push(HirStmt::Expr(HirExpr {
        ty: Type::I64,
        kind: HirExprKind::Call(
            Box::new(HirExpr {
                ty: Type::Function(Vec::new(), Box::new(Type::I64)),
                kind: HirExprKind::ResolvedVar(HirVarRef {
                    name: "bad".to_string(),
                    target: HirVarTarget::Function(invalid_target),
                }),
                span: Default::default(),
            }),
            Vec::new(),
            Some(HirCallTarget::Function(invalid_target)),
        ),
        span: Default::default(),
    }));

    let hir = resolved_hir_with_function(function);
    let _ = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("test"),
        &hir,
        Vec::new(),
        Vec::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
}
```

Extend the `crate::hir` test import in `lib/src/products.rs` to include `HirCallTarget` and `HirStmt`. Use the existing `test_function` helper and add this helper in the same test module:

```rust
fn resolved_hir_with_function(function: HirFunction) -> ResolvedHirProgram {
    let mut functions = HashMap::new();
    functions.insert(function.name.clone(), function);
    let program = HirProgram::from_parts(
        functions,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    let current_def_ids = program.functions.keys().copied().collect();

    ResolvedHirProgram::new(
        program,
        ResolverTables::default(),
        current_def_ids,
        CrateId(0),
        local_def_ids_after(100),
    )
}
```

- [x] **Step 2: Run failing product validation test**

Run: `cargo test -p rock-lib compiler_products_reject_call_targets_with_invalid_current_crate_ids`

Expected: FAIL because call-target sidecars are not validated.

- [x] **Step 3: Validate call targets and local IDs in product emission**

In `lib/src/products.rs`, update `assert_valid_product_expr_ids`:

```rust
HirExprKind::Call(func, args, target) => {
    if let Some(target) = target {
        assert_valid_product_call_target(target);
    }
    assert_valid_product_expr_ids(func);
    for arg in args {
        assert_valid_product_expr_ids(arg);
    }
}
```

Add:

```rust
fn assert_valid_product_call_target(target: &HirCallTarget) {
    match target {
        HirCallTarget::Function(id) | HirCallTarget::Extern(id) => {
            assert_valid_product_input_def_id(*id);
        }
        HirCallTarget::Instance(_) | HirCallTarget::Local(_) | HirCallTarget::Intrinsic(_) => {}
    }
}
```

Update `ResolvedVar` validation for `HirVarTarget::Local(_)`.

- [x] **Step 4: Remap call target product IDs**

In `lib/src/products.rs`, update `remap_expr_location_product_ids` for calls:

```rust
HirExprKind::Call(func, args, target) => {
    if let Some(target) = target {
        remap_call_target_product_ids(target, id_remap);
    }
    remap_expr_location_product_ids(func, id_remap);
    for arg in args {
        remap_expr_location_product_ids(arg, id_remap);
    }
}
```

Add:

```rust
fn remap_call_target_product_ids(
    target: &mut HirCallTarget,
    id_remap: &BTreeMap<ProductDefId, BTreeSet<ProductDefId>>,
) {
    match target {
        HirCallTarget::Function(id) | HirCallTarget::Extern(id) => {
            remap_product_def_id_owner(id, id_remap);
        }
        HirCallTarget::Instance(_) | HirCallTarget::Local(_) | HirCallTarget::Intrinsic(_) => {}
    }
}
```

- [x] **Step 5: Update artifact loader validation**

In `lib/src/crate_artifact/load.rs`, extend existing expression validation that checks `ResolvedVar`, field locations, variant locations, and method targets so it also checks `HirCallTarget::Function` and `HirCallTarget::Extern` against known product definitions. `HirCallTarget::Local` validates structurally by confirming the local ID exists in the same body's parameter/let/pattern/capture set.

- [x] **Step 6: Run product and artifact tests**

Run: `cargo test -p rock-lib compiler_products_reject_call_targets_with_invalid_current_crate_ids`

Expected: PASS.

Run: `cargo test -p rock-lib product_artifact_format_version_matches_shared_contract`

Expected: PASS with version `21`.

Run: `cargo test -p rock-lib product_artifact`

Expected: PASS.

- [x] **Step 7: Commit Task 7**

Run:

```bash
git add lib/src/products.rs lib/src/crate_artifact/load.rs rock-shared/src/sysroot.rs
git commit -m "validate hir reference sidecars in products"
```

---

## Task 8: Final Task 3 Audit, Docs, Review, And Verification

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/2026-05-26-hir-semantic-reference-ids.md`

- [x] **Step 1: Run final focused reference audit greps**

Run:

```bash
rg "HirExprKind::Var\(|HirPattern::Binding|HirExprKind::Call\(|HirExprKind::For \{|HirClosureCapture \{|HirParam \{|HirStmt::Let \{" lib/src
```

Expected: every resolved semantic reference construction either carries `HirLocalId`, `HirVarTarget`, `HirCallTarget`, `HirFieldLocation`, `HirVariantLocation`, or is explicitly unresolved/error recovery in nearby code.

- [x] **Step 2: Update master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, update Identity And Arenas evidence and done lists:

```markdown
- `lib/src/hir/mod.rs` now carries scoped `HirLocalId` identity for HIR params, lets, pattern bindings, tuple temporaries, loop variables, local reads, assignments, and closure captures.
- `lib/src/hir/mod.rs`, `lib/src/lower/**`, `lib/src/products.rs`, and `lib/src/crate_artifact/load.rs` now carry, preserve, serialize, remap, and validate HIR reference sidecars for direct calls, local references, fields, variants, aggregates, and method targets.
```

Move the Task 3 local/reference/call-edge line out of Identity And Arenas `Still to do`. Keep compatibility string-map removal under Tasks 4-5 and backend metadata cleanup under later tracks.

- [x] **Step 3: Update ordered roadmap Task 3 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, change Task 3 in the reconciliation table to `Complete` and update Task 3 section status:

```markdown
**Status:** Complete. HIR semantic references now carry authoritative `DefId`, `InstanceId`, field/variant IDs, selected method targets, call targets, or scoped `HirLocalId`s where lowering resolves them. Remaining compatibility strings are display/diagnostic metadata or belong to Tasks 4-5, 11-15, 18, and 21.
```

- [x] **Step 4: Add final verification notes to this plan**

Append a `## Final Verification` section to this file with the exact command outputs and grep classification.

- [x] **Step 5: Request code review**

Use the `requesting-code-review` skill. The review prompt must ask the reviewer to verify:

- Task 3 scope is complete without bleeding into Tasks 4-5 or 11-15.
- All resolved local references carry `HirLocalId`.
- All direct call sidecars are present when known.
- Aggregate/field/variant/method sidecars are present for resolved references.
- Product/artifact remapping validates the new sidecars.
- Documentation marks only Task 3 complete and keeps later compatibility work open.

- [x] **Step 6: Run final verification commands**

Run these commands in order:

```bash
cargo test -p rock-lib hir_semantic_reference
cargo test -p rock-lib local_id
cargo test -p rock-lib resolved_reference
cargo test -p rock-lib product_artifact
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

If a focused filter matches zero tests, replace it with the exact test names added in this implementation and record the replacement in `## Final Verification`.

- [x] **Step 7: Commit final docs**

Run:

```bash
git add docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/2026-05-26-hir-semantic-reference-ids.md
git commit -m "mark task 3 hir references complete"
```

---

## Completion Criteria

- All tasks above have been implemented with RED/GREEN verification for new behavior tests.
- Final code review reports no Critical or Important findings.
- `cargo test -p rock-lib` passes.
- `cargo fmt --all --check` passes.
- `git diff --check` passes.
- `master-audit-checklist.md` and `2026-05-17-compiler-architecture-ordered-roadmap.md` mark Task 3 complete while preserving later-task work under the correct tracks.

## Final Verification

Audit command run:

```bash
rg "HirExprKind::Var\(|HirPattern::Binding|HirExprKind::Call\(|HirExprKind::For \{|HirClosureCapture \{|HirParam \{|HirStmt::Let \{" lib/src
```

Audit classification:
- HIR semantic reference shapes in `lib/src/hir/mod.rs` carry the required sidecars: `HirParam.local_id`, `HirClosureCapture.local_id`, `HirStmt::Let.local_id`, `HirPattern::Binding.local_id`, `HirExprKind::For.local_id`, `HirExprKind::ResolvedVar(HirVarRef { target: HirVarTarget })`, `HirExprKind::Call(..., Option<HirCallTarget>)`, `FieldAccess(..., Option<HirFieldLocation>)`, struct literal/pattern field locations, enum variant locations, and `MethodCall(..., Option<HirMethodCallTarget>)`.
- Lowering constructions for params, lets, tuple temporaries, pattern bindings, loop variables, closure captures, local reads/assignments, direct function/extern references, known direct calls, local function-value calls, field accesses, aggregate literals/patterns, enum variants, and selected methods attach the relevant `HirLocalId`, `HirVarTarget`, `HirCallTarget`, `HirFieldLocation`, `HirVariantLocation`, `DefId`, `InstanceId`, or selected method target when resolution succeeds.
- Remaining `HirExprKind::Var` and targetless `Call` matches in the audit output are HIR traversal/consumer arms, compatibility name consumers, tests/manual fixtures, intrinsic/error recovery such as `<error>`, or explicit unresolved fallback paths where lowering could not resolve a semantic target.
- Product and artifact paths in `lib/src/products.rs` and `lib/src/crate_artifact/load.rs` remap and validate function/extern targets, call targets, field/variant/method sidecars, scoped local call targets, and scoped resolved local variable targets; artifact local validation follows traversal-time lexical scope so nested/later locals do not leak into invalid sidecars.

Commands run:
- `cargo test -p rock-lib hir_semantic_reference` passed, but matched zero tests: `0 passed; 0 failed; 1308 filtered out` in `src/lib.rs`, `0 passed; 0 failed; 277 filtered out` in `tests/integration.rs`, and `0 passed; 0 failed; 1 filtered out` in `tests/test_parse_struct_with_fields.rs`.
- Replacement for zero-test `hir_semantic_reference`: `cargo test -p rock-lib hir_reference_shapes_carry_resolved_targets` passed: `1 passed; 0 failed` in `src/lib.rs`.
- `cargo test -p rock-lib local_id` passed: `10 passed; 0 failed` in `src/lib.rs`; integration and parse-struct test binaries matched zero tests.
- `cargo test -p rock-lib resolved_reference` passed, but matched zero tests: `0 passed; 0 failed; 1308 filtered out` in `src/lib.rs`, `0 passed; 0 failed; 277 filtered out` in `tests/integration.rs`, and `0 passed; 0 failed; 1 filtered out` in `tests/test_parse_struct_with_fields.rs`.
- Replacements for zero-test `resolved_reference`: `cargo test -p rock-lib lower_direct_function_reference_records_function_id`, `cargo test -p rock-lib lower_direct_extern_reference_records_extern_id`, `cargo test -p rock-lib direct_function_call_records_call_target`, `cargo test -p rock-lib remap_expr_location_product_ids_remaps_new_reference_sidecars`, and `cargo test -p rock-lib remap_expr_location_product_ids_remaps_call_target_sidecars` each passed: `1 passed; 0 failed` in `src/lib.rs`.
- Final review blocker replacements: `cargo test -p rock-lib remap_function_child_locations_rejects_unknown_local_resolved_var_target` passed, `cargo test -p rock-lib custom_operator_scope_local_lowers_callee_to_local_call_target` passed, and `cargo test -p rock-lib call_target` passed with `17 passed; 0 failed` in `src/lib.rs`.
- `cargo test -p rock-lib product_artifact` passed: `79 passed; 0 failed` in `src/lib.rs`; integration and parse-struct test binaries matched zero tests.
- Final code review approved after blocker fixes. Reviewer re-ran `cargo test -p rock-lib remap_function_child_locations_rejects_unknown_local_resolved_var_target`, `cargo test -p rock-lib custom_operator_scope_local_lowers_callee_to_local_call_target`, `cargo test -p rock-lib call_target`, `cargo test -p rock-lib product_artifact`, `cargo fmt --all --check`, and `git diff --check HEAD`; all passed.
- `cargo test -p rock-lib` passed: `1307 passed; 0 failed; 1 ignored` in `src/lib.rs`, `277 passed; 0 failed` in `tests/integration.rs`, `1 passed; 0 failed` in `tests/test_parse_struct_with_fields.rs`, and doctests passed with `1 passed; 0 failed; 1 ignored`.
- `cargo fmt --all --check` passed with no output.
- `git diff --check` passed with no output.
- Follow-up completion: after this verification and documentation update, the completed Task 3 source and documentation changes are ready to be committed together per user request.
