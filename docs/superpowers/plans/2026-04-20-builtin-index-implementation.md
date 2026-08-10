# Builtin Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Index` a real stdlib trait and route builtin array, pointer, and `[U8]` indexing through synthesized builtin trait impl support so the `assoc-types-ops` branch returns to a green `cargo test -p rock-lib` baseline without restoring a separate legacy builtin indexing path.

**Architecture:** Add a real stdlib `Index` trait, then teach compiler trait resolution, monomorphization, MIR place lowering, and codegen to recognize builtin synthesized `Index` impls for `Array<T>`, `*T`, and `[U8]`. Keep surface lowering of `a[b]` as `Deref(MethodCall("index"))`, resolve builtin `Index::Output` projections through the shared trait machinery, and let builtin dispatch reuse existing low-level address/index logic instead of a parallel early HIR node.

**Tech Stack:** Rust 2021, Rock stdlib `.rk` modules, compiler lowering/HIR/MIR/monomorphization/codegen pipeline, `cargo test -p rock-lib`.

---

### Task 1: Add The Real Stdlib `Index` Trait Surface

**Files:**
- Create: `stdlib/index.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`
- Test: `lib/tests/integration.rs:3542-3566`

- [ ] **Step 1: Write the failing integration test that depends on prelude `Index`**

In `lib/tests/integration.rs`, add this test near the existing associated-type operator tests:

```rust
#[test]
fn test_array_index_uses_stdlib_index_trait() {
    let output = compile_and_run(
        r#"
main = ->
    arr = [10, 20, 30]
    arr[1].println!
    0
"#,
    );

    assert_eq!(output.trim(), "20");
}
```

- [ ] **Step 2: Run the new test to verify it fails for the expected reason**

Run: `cargo test -p rock-lib --test integration test_array_index_uses_stdlib_index_trait -- --exact --nocapture`
Expected: FAIL because stdlib does not yet expose a real `Index` trait even though lowering wants the trait-style path.

- [ ] **Step 3: Add the stdlib trait declaration**

Create `stdlib/index.rk` with exactly this content:

```rock
// Index trait - provides `a[b]` indexing through an associated output type.

< trait Index Idx
    type Output
    @index: Self -> Idx -> &Self::Output
```

- [ ] **Step 4: Export the new module from stdlib**

In `stdlib/lib.rk`, add the module declaration alongside the other trait modules:

```rock
// Index trait - indexing with associated output type
< mod index
```

Place it after `< mod show` and before `< mod fp` so trait modules stay grouped.

- [ ] **Step 5: Re-export `Index` from the prelude**

In `stdlib/prelude.rk`, add the trait re-export:

```rock
< stdlib::index::*
```

Place it with the other trait exports.

- [ ] **Step 6: Run the focused tests to verify the surface is wired in**

Run:

```bash
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_array_index_uses_stdlib_index_trait -- --exact --nocapture
```

Expected:
- the custom `Index` test should still pass or fail only in the known builtin-dispatch area;
- the new array test should still fail, but now after the stdlib trait exists.

### Task 2: Add Builtin `Index` Metadata And Projection Resolution

**Files:**
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/types/mod.rs`
- Test: `lib/tests/integration.rs:557-572`
- Test: `lib/tests/integration.rs:2884-2906`

- [ ] **Step 1: Write the failing builtin projection tests**

In `lib/tests/integration.rs`, keep these existing tests as the red tests for this task:

```rust
#[test]
fn test_array_indexing() { /* existing test at lines 557-572 */ }

#[test]
fn test_str_indexing() { /* existing test at lines 2884-2906 */ }
```

Do not rewrite them; they already express the required concrete `Output` behavior.

- [ ] **Step 2: Run the array indexing test to verify the unresolved projection failure**

Run: `cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture`
Expected: FAIL with a message in the current family of `Field access on non-struct type: <[I64] as Index<I64>>::Output field: println`.

- [ ] **Step 3: Add builtin `Index` helper queries to the type layer**

In `lib/src/types/mod.rs`, add small shared helpers on `Type` for builtin `Index` recognition. Add methods with this shape near the existing `impl Type` helpers:

```rust
pub fn builtin_index_output(&self, idx: &Type) -> Option<Type> {
    match (self, idx) {
        (Type::Array(inner), Type::I64) if matches!(inner.as_ref(), Type::U8) => Some(Type::Char),
        (Type::Array(inner), Type::I64) => Some((**inner).clone()),
        (Type::Pointer(inner), Type::I64) => Some((**inner).clone()),
        _ => None,
    }
}

pub fn has_builtin_index_impl(&self, idx: &Type) -> bool {
    self.builtin_index_output(idx).is_some()
}
```

Keep these helpers minimal. Do not add mutable-index helpers or other operator traits here.

- [ ] **Step 4: Resolve builtin `Index::Output` in lowering**

In `lib/src/lower/types_helpers/helpers.rs`, update `resolve_projection_type` so it checks builtin `Index` support before scanning user impls.

Insert this branch after `resolved_base` and `resolved_trait_args` are computed:

```rust
if trait_name == "Index" && resolved_trait_args.len() == 1 {
    if let Some(output) = resolved_base.builtin_index_output(&resolved_trait_args[0]) {
        return self.resolve_projection_type(&output);
    }
}
```

Leave the existing user-impl lookup intact after this branch.

- [ ] **Step 5: Resolve builtin `Index::Output` in codegen too**

In `lib/src/codegen/types.rs`, make the same early builtin branch inside `CodeGen::resolve_projection_type`:

```rust
if trait_name == "Index" && resolved_trait_args.len() == 1 {
    if let Some(output) = resolved_base.builtin_index_output(&resolved_trait_args[0]) {
        return self.resolve_projection_type(&output);
    }
}
```

Do not remove the existing trait-impl lookup for user impls.

- [ ] **Step 6: Run the focused projection tests**

Run:

```bash
cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture
cargo test -p rock-lib --test integration test_str_indexing -- --exact --nocapture
```

Expected:
- the unresolved projection error should be gone;
- at least one test should still fail later in dispatch/codegen because builtin `index` calls are not yet compiled through the trait path.

### Task 3: Teach Monomorphization To Recognize Builtin `Index` Dispatch

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/process.rs`
- Test: `lib/tests/integration.rs:557-572`
- Test: `lib/tests/integration.rs:3542-3566`

- [ ] **Step 1: Use the current array/custom `Index` tests as the red tests**

Keep these two tests as the red surface for this task:

```rust
#[test]
fn test_array_indexing() { /* existing */ }

#[test]
fn test_index_dispatches_through_trait_with_associated_output() { /* existing */ }
```

- [ ] **Step 2: Run both tests and capture the current dispatch failure**

Run:

```bash
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture
```

Expected: custom `Index` should pass or nearly pass, while builtin array indexing still fails because monomorphization has no builtin dispatch identity for `index`.

- [ ] **Step 3: Add builtin-type name support for pointer receivers**

In `lib/src/mono/mod.rs`, extend `Monomorphizer::get_type_name_for_method` so raw pointers have a method-lookup name:

```rust
Type::Pointer(_) => Some("Ptr".to_string()),
```

Keep the existing `Array` and `[U8]` cases unchanged.

- [ ] **Step 4: Add a builtin `Index` dispatch classifier**

In `lib/src/mono/methods.rs`, add a helper on `Monomorphizer` with this shape near the other method helpers:

```rust
fn is_builtin_index_dispatch(recv_ty: &Type, method_name: &str, args: &[HirExpr]) -> bool {
    method_name == "index"
        && args.len() == 2
        && recv_ty.has_builtin_index_impl(&args[1].ty)
}
```

This helper is only for builtin `Index` dispatch, not generic method dispatch.

- [ ] **Step 5: Preserve builtin `index` method calls for codegen instead of forcing user-impl specialization**

In `lib/src/mono/process.rs`, inside the `HirExprKind::MethodCall` branch, guard the existing trait/standalone monomorphization calls:

```rust
if let Some(ref type_name) = recv_type_name {
    if !Self::is_builtin_index_dispatch(&processed_recv.ty, &method_name_clone, &all_args) {
        self.monomorphize_trait_method_call(type_name, &method_name_clone, &all_args, expr);
        if matches!(expr.kind, HirExprKind::MethodCall(..)) {
            self.monomorphize_standalone_method_call(type_name, &method_name_clone, &all_args, expr);
        }
    }
}
```

Builtin `index` stays as a `MethodCall` so codegen can lower it directly. User impls still follow the normal specialization path.

- [ ] **Step 6: Run the focused dispatch tests**

Run:

```bash
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture
```

Expected:
- custom trait `Index` dispatch still passes;
- builtin array indexing now reaches codegen as a trait-style `MethodCall` instead of failing in monomorphization.

### Task 4: Compile Builtin Trait-Lowered `index` Calls And References In Codegen

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/expr/call.rs`
- Modify: `lib/src/codegen/expr/access.rs`
- Test: `lib/tests/integration.rs:557-572`
- Test: `lib/tests/integration.rs:2884-2906`
- Test: `lib/tests/integration.rs:3156-3200`

- [ ] **Step 1: Use the builtin indexing tests as the red tests for codegen**

Keep these existing tests as the task red tests:

```rust
#[test]
fn test_array_indexing() { /* existing */ }

#[test]
fn test_str_indexing() { /* existing */ }

#[test]
fn test_ptr_index_roundtrip() { /* existing */ }
```

- [ ] **Step 2: Run the array indexing test to confirm builtin `MethodCall("index")` is still not codegenerated**

Run: `cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture`
Expected: FAIL in codegen or execution because builtin trait-lowered `index` still lacks direct compilation support.

- [ ] **Step 3: Add a builtin `Index` classifier to codegen**

In `lib/src/codegen/mod.rs`, add a small helper on `CodeGen`:

```rust
fn is_builtin_index_call(&self, recv_ty: &Type, method_name: &str, args: &[HirExpr]) -> bool {
    method_name == "index"
        && args.len() == 1
        && self.resolve_projection_type(recv_ty)
            .has_builtin_index_impl(&self.resolve_projection_type(&args[0].ty))
}
```

This helper should only detect compiler-backed builtin `Index` calls.

- [ ] **Step 4: Teach `compile_expr` to route builtin `MethodCall("index")` through pointer-producing code**

In `lib/src/codegen/expr/mod.rs`, inside the `HirExprKind::MethodCall` branch before normal mangled-function lookup, add this fast path:

```rust
if self.is_builtin_index_call(&recv.ty, method, args) {
    return self.compile_builtin_index_method(recv, &args[0], &expr.ty);
}
```

Implement `compile_builtin_index_method` in `lib/src/codegen/expr/access.rs`.

- [ ] **Step 5: Implement builtin `index` method value lowering**

In `lib/src/codegen/expr/access.rs`, add a helper that returns the reference/pointer value for trait-lowered builtin indexing:

```rust
pub(super) fn compile_builtin_index_method(
    &mut self,
    base_expr: &HirExpr,
    index_expr: &HirExpr,
    result_ty: &Type,
) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
    let index_result_ty = self.resolve_projection_type(result_ty);
    let elem_ptr = self.compile_index_ptr(base_expr, index_expr, &index_result_ty)?;
    Ok(Some(elem_ptr.into()))
}
```

Also extract the pointer-computation part of the current `compile_index` logic into a reusable helper with this shape:

```rust
fn compile_index_ptr(
    &mut self,
    base_expr: &HirExpr,
    index_expr: &HirExpr,
    result_ty: &Type,
) -> Result<PointerValue<'ctx>, CodegenError>
```

Use the existing logic already in `compile_index`:
- pointer receivers use GEP on the raw pointer;
- array receivers extract the fat-pointer payload and GEP into it;
- `[U8]` keeps the current bounds checks before the GEP.

Then rewrite `compile_index` itself to call `compile_index_ptr(...)` and load from the returned pointer.

- [ ] **Step 6: Teach `compile_ref` to preserve builtin indexed addresses**

In `lib/src/codegen/expr/access.rs`, add a match arm before the generic temp-allocation fallback:

```rust
crate::hir::HirExprKind::Deref(inner) => {
    if let crate::hir::HirExprKind::MethodCall(recv, method_name, args, _) = &inner.kind {
        if self.is_builtin_index_call(&recv.ty, method_name, args) {
            let ptr = self.compile_builtin_index_method(recv, &args[0], &inner.ty)?
                .ok_or(CodegenError::from("Builtin index produced no pointer"))?
                .into_pointer_value();
            return Ok(Some(ptr.into()));
        }
    }
}
```

This is what lets `&arr[i]` and later assignment paths keep a real address instead of a temporary copy.

- [ ] **Step 7: Run the focused builtin-codegen tests**

Run:

```bash
cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture
cargo test -p rock-lib --test integration test_str_indexing -- --exact --nocapture
cargo test -p rock-lib --test integration test_ptr_index_roundtrip -- --exact --nocapture
```

Expected: PASS.

### Task 5: Teach MIR Place Lowering To Understand Trait-Lowered Builtin Indexing

**Files:**
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/tests/integration.rs:631-647`
- Test: `lib/tests/integration.rs:3156-3200`

- [ ] **Step 1: Use assignment tests as the red tests**

Keep these existing tests as the task red tests:

```rust
#[test]
fn test_array_index_assignment() { /* existing */ }

#[test]
fn test_ptr_index_write() { /* existing */ }
```

- [ ] **Step 2: Run the array assignment test to confirm the place-lowering gap**

Run: `cargo test -p rock-lib --test integration test_array_index_assignment -- --exact --nocapture`
Expected: FAIL because MIR place lowering only understands `HirExprKind::Index`, not `Deref(MethodCall("index"))`.

- [ ] **Step 3: Add a builtin trait-lowered index recognizer in MIR**

In `lib/src/mir/builder/mod.rs`, add a helper on `MirBuilder` with this shape near `lower_place`:

```rust
fn builtin_index_parts<'a>(&self, expr: &'a HirExpr) -> Option<(&'a HirExpr, &'a HirExpr)> {
    match &expr.kind {
        HirExprKind::Deref(inner) => match &inner.kind {
            HirExprKind::MethodCall(recv, method_name, args, _)
                if method_name == "index"
                    && args.len() == 1
                    && recv.ty.has_builtin_index_impl(&args[0].ty) => Some((recv, &args[0])),
            _ => None,
        },
        _ => None,
    }
}
```

- [ ] **Step 4: Lower builtin trait-lowered indexing as a place**

In `lib/src/mir/builder/mod.rs`, update `lower_place` so the `HirExprKind::Deref(base)` branch checks `builtin_index_parts(expr)` before the generic deref branch.

Use this shape:

```rust
if let Some((recv, index)) = self.builtin_index_parts(expr) {
    let mut place = self.lower_place(recv)?;
    let index_local = if let Some(index_place) = self.lower_place(index) {
        index_place.local
    } else {
        let index_temp = self.new_local_from_expr(index.ty.clone(), index);
        self.emit_storage_live(index_temp, Some(index.span.clone()));
        let index_place = Place { local: index_temp, projection: vec![] };
        self.lower_expr(index, index_place);
        index_temp
    };
    place.projection.push(Projection::Index(index_local));
    return Some(place);
}
```

Keep the old generic `Deref(base)` handling after this special builtin-indexed-place branch.

- [ ] **Step 5: Run the focused MIR assignment tests**

Run:

```bash
cargo test -p rock-lib --test integration test_array_index_assignment -- --exact --nocapture
cargo test -p rock-lib --test integration test_ptr_index_write -- --exact --nocapture
```

Expected: PASS.

### Task 6: Run Recovery Verification And Fold It Back Into The Branch Goal

**Files:**
- Test: `lib/tests/integration.rs`
- Test: `lib/src/crate_artifact/tests.rs`
- Test: `lib/src/parser/**/tests/`

- [ ] **Step 1: Re-run the focused associated-type operator tests**

Run:

```bash
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_deref_target_type_drives_unary_deref -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 2: Re-run the builtin indexing matrix**

Run:

```bash
cargo test -p rock-lib --test integration test_array_indexing -- --exact --nocapture
cargo test -p rock-lib --test integration test_array_index_assignment -- --exact --nocapture
cargo test -p rock-lib --test integration test_str_indexing -- --exact --nocapture
cargo test -p rock-lib --test integration test_ptr_index_write -- --exact --nocapture
cargo test -p rock-lib --test integration test_ptr_index_roundtrip -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 3: Re-run the already-fixed parser and artifact regressions to protect the branch**

Run:

```bash
cargo test -p rock-lib parser::items::tests::ast_validation::empty_tuple -- --exact
cargo test -p rock-lib crate_artifact::tests::test_artifact_roundtrip_preserves_associated_types -- --exact
```

Expected: PASS.

- [ ] **Step 4: Run the full library suite**

Run: `cargo test -p rock-lib`
Expected: PASS.

- [ ] **Step 5: Commit the builtin `Index` implementation**

Run:

```bash
git add stdlib/index.rk stdlib/lib.rk stdlib/prelude.rk lib/src/types/mod.rs lib/src/lower/types_helpers/helpers.rs lib/src/mono/mod.rs lib/src/mono/methods.rs lib/src/mono/process.rs lib/src/codegen/mod.rs lib/src/codegen/types.rs lib/src/codegen/expr/mod.rs lib/src/codegen/expr/call.rs lib/src/codegen/expr/access.rs lib/src/mir/builder/mod.rs lib/tests/integration.rs docs/superpowers/specs/2026-04-20-builtin-index-design.md docs/superpowers/plans/2026-04-20-builtin-index-implementation.md
git commit -m "feat: synthesize builtin index trait support"
```

Expected: commit succeeds after all tests pass.
