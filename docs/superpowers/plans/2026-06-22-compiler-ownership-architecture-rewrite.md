# Compiler Ownership Architecture Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish Rock's production MIR ownership core while fixing the reviewed receiver, lifetime, drop, artifact, stdlib, and test-cache defects.

**Architecture:** Add explicit receiver adjustment semantics, then centralize ownership validation over MIR for moves, loans, temporary escapes, and drop obligations. Keep changes staged so each invariant has failing tests before implementation and can be reviewed independently.

**Tech Stack:** Rust `rock-lib`, existing HIR/MIR/type-context infrastructure, Rock stdlib files, integration tests in `lib/tests/integration.rs`, focused unit tests near `selection`, `lower`, `mir`, `products`, and `crate_artifact`.

---

## File Structure

- Modify `lib/src/selection/types.rs`: expand `ReceiverAdjustment` into an explicit receiver coercion plan.
- Modify `lib/src/selection/service.rs`: enforce receiver mutability and return adjustment plans.
- Modify `lib/src/lower/types_helpers/helpers.rs`: generate legal receiver adjustment candidates, including mutable autoref only for mutable lvalues.
- Modify `lib/src/lower/control_flow/secondary.rs`: apply selected receiver adjustment consistently for method calls and method values.
- Modify `lib/src/lower/expression.rs`: apply the same selection/adjustment rules for operator desugaring.
- Modify `lib/src/mir/mod.rs`: add ownership metadata types for temporary scopes, reference origins, and drop obligations if the task determines side tables are cleaner than terminator changes.
- Modify `lib/src/mir/builder/mod.rs`, `lib/src/mir/builder/blocks.rs`, and `lib/src/mir/builder/expr.rs`: preserve temporary/reference/drop metadata during MIR construction and drop elaboration.
- Modify `lib/src/mir/borrowck/mod.rs`: make MIR ownership validation the central check for moves, loans, temporary escapes, and drop obligations.
- Modify `lib/src/codegen/mir_llvm/terminator.rs`: require direct drop glue for every direct drop terminator.
- Modify `lib/src/codegen/metadata.rs`: expose validated drop metadata without codegen policy decisions.
- Modify `lib/src/products.rs`, `lib/src/crate_system/extern_store.rs`, `lib/src/crate_system/context.rs`, and `lib/src/mono/external.rs`: share one downstream-specialization predicate.
- Modify `stdlib/alloc.rk`, `stdlib/raw_buffer.rk`, `stdlib/vec.rk`, and `stdlib/hash_map.rk`: centralize checked allocation arithmetic and remove/fix unsafe `Vec` deref.
- Modify `lib/tests/integration.rs`: add integration coverage for user-visible receiver, escape, stdlib, artifact, and cache behavior.
- Add or modify unit tests in nearby Rust modules when a phase-specific invariant can be tested without full integration compilation.

## Execution Rules

- Do not inspect or modify `.sisyphus/`.
- Do not commit unless the user explicitly asks during execution. The commit steps below are markers for approved commit points only.
- Run the smallest relevant test command after each task. Save broad-suite verification for the final task.
- Keep codegen as LLVM lowering only; ownership policy belongs in selection, lowering, MIR construction, and MIR ownership validation.

---

### Task 1: Receiver Adjustment Architecture

**Files:**
- Modify: `lib/src/selection/types.rs`
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`
- Test: `lib/tests/integration.rs`
- Test: `lib/src/selection/service.rs`

- [ ] **Step 1: Write failing integration tests for mutable receiver legality**

Add tests to `lib/tests/integration.rs`:

```rust
#[test]
fn immutable_binding_cannot_call_mut_receiver_method() {
    compile_should_fail(
        r#"
struct Counter
    < value: I64

impl Counter
    ^@inc: Unit
    ^@inc = ->
        self.value = self.value + 1
        return

main = ->
    c = Counter
        value: 0
    c.inc!
    0
"#,
        "mutable receiver",
    );
}

#[test]
fn shared_reference_cannot_call_mut_receiver_method() {
    compile_should_fail(
        r#"
struct Counter
    < value: I64

impl Counter
    ^@inc: Unit
    ^@inc = ->
        self.value = self.value + 1
        return

use_shared: &Counter -> Unit
use_shared = c ->
    c.inc!
    return

main = -> 0
"#,
        "mutable receiver",
    );
}

#[test]
fn mutable_binding_can_call_mut_receiver_method() {
    let output = compile_and_run(
        r#"
struct Counter
    < value: I64

impl Counter
    ^@inc: Unit
    ^@inc = ->
        self.value = self.value + 1
        return

    @get: I64
    @get = -> self.value

main = ->
    mut c = Counter
        value: 1
    c.inc!
    (c.get!).println!
    0
"#,
    );
    assert_eq!(output.trim(), "2");
}
```

- [ ] **Step 2: Run receiver tests and verify red**

Run: `cargo test -p rock-lib --test integration immutable_binding_cannot_call_mut_receiver_method shared_reference_cannot_call_mut_receiver_method mutable_binding_can_call_mut_receiver_method -- --nocapture`

Expected: at least one negative test fails because selection currently accepts the mutable receiver incorrectly, or diagnostics do not mention mutable receiver.

- [ ] **Step 3: Replace receiver adjustment enum with explicit plan names**

In `lib/src/selection/types.rs`, use this enum shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverAdjustment {
    None,
    AutorefShared,
    AutorefMut,
    MutToSharedRef,
    BuiltinDeref,
    TraitDeref,
    ArrayRefToSliceRef,
    ArrayValueToSliceRef,
    ArrayValueToSliceValue,
}
```

Replace old variant names at call sites. Map previous behavior as follows: old `Autoderef` becomes `BuiltinDeref` only when the receiver is a built-in reference deref; trait-based deref uses `TraitDeref`; old array/slice variants keep their specific names.

- [ ] **Step 4: Make receiver candidates carry mutability legality**

In `lib/src/lower/types_helpers/helpers.rs`, add a helper that identifies mutable lvalues:

```rust
pub(crate) fn expr_is_mutable_lvalue(&self, expr: &HirExpr) -> bool {
    match &expr.kind {
        HirExprKind::Var(name) => self.scope.local_is_mutable(name),
        HirExprKind::FieldAccess(base, _, _) => self.expr_is_mutable_lvalue(base),
        HirExprKind::Deref(inner) => matches!(self.engine.resolve(&inner.ty), Type::Reference { mutable: true, .. }),
        _ => false,
    }
}
```

If `scope.local_is_mutable` does not exist, add the narrow equivalent to the scope type that already stores local mutability. Do not infer mutability from type alone for non-reference locals.

- [ ] **Step 5: Enforce mutable receiver matching in selection**

In `lib/src/selection/service.rs`, replace `self_type_matches` with a mode-aware function:

```rust
fn self_type_matches(
    expected_self: &Type,
    receiver_ty: &Type,
    receiver_can_mut_borrow: bool,
    subst: &mut HashMap<GenericParamId, Type>,
) -> Option<ReceiverAdjustment> {
    match expected_self {
        Type::Reference { mutable: true, inner } => {
            if matches!(receiver_ty, Type::Reference { mutable: true, inner: actual } if type_pattern_matches(inner, actual, subst)) {
                return Some(ReceiverAdjustment::None);
            }
            if receiver_can_mut_borrow && type_pattern_matches(inner, receiver_ty, subst) {
                return Some(ReceiverAdjustment::AutorefMut);
            }
            None
        }
        Type::Reference { mutable: false, inner } => {
            if matches!(receiver_ty, Type::Reference { mutable: false, inner: actual } if type_pattern_matches(inner, actual, subst)) {
                return Some(ReceiverAdjustment::None);
            }
            if matches!(receiver_ty, Type::Reference { mutable: true, inner: actual } if type_pattern_matches(inner, actual, subst)) {
                return Some(ReceiverAdjustment::MutToSharedRef);
            }
            if type_pattern_matches(inner, receiver_ty, subst) {
                return Some(ReceiverAdjustment::AutorefShared);
            }
            None
        }
        _ if type_pattern_matches(expected_self, receiver_ty, subst) => Some(ReceiverAdjustment::None),
        _ => None,
    }
}
```

Thread `receiver_can_mut_borrow` from lowering into selection requests. Use `false` for trait-bound generic receivers unless lowering proves a mutable lvalue.

- [ ] **Step 6: Apply adjustment in lowering**

In `lib/src/lower/control_flow/secondary.rs`, before constructing `HirExprKind::MethodCall`, transform `selected.receiver` according to the selected adjustment:

```rust
let adjusted_recv = self.apply_receiver_adjustment(selected.receiver.clone(), selected.receiver_adjustment);
```

Implement `apply_receiver_adjustment` in the lowerer:

```rust
fn apply_receiver_adjustment(&mut self, receiver: HirExpr, adjustment: ReceiverAdjustment) -> HirExpr {
    match adjustment {
        ReceiverAdjustment::None => receiver,
        ReceiverAdjustment::AutorefShared => self.ref_expr(false, receiver),
        ReceiverAdjustment::AutorefMut => self.ref_expr(true, receiver),
        ReceiverAdjustment::MutToSharedRef => self.coerce_mut_ref_to_shared_ref(receiver).unwrap_or_else(|| self.error_expression()),
        ReceiverAdjustment::BuiltinDeref => self.apply_builtin_shared_ref_deref(receiver).unwrap_or_else(|| self.error_expression()),
        ReceiverAdjustment::TraitDeref => self.apply_trait_deref(receiver).unwrap_or_else(|| self.error_expression()),
        ReceiverAdjustment::ArrayRefToSliceRef => self.coerce_array_ref_to_slice_ref(receiver).unwrap_or_else(|| self.error_expression()),
        ReceiverAdjustment::ArrayValueToSliceRef => self.coerce_array_value_to_slice_ref(receiver).unwrap_or_else(|| self.error_expression()),
        ReceiverAdjustment::ArrayValueToSliceValue => self.coerce_array_value_to_slice_value(receiver).unwrap_or_else(|| self.error_expression()),
    }
}
```

If helper names differ, add wrappers with these names rather than duplicating coercion logic.

- [ ] **Step 7: Run receiver tests and verify green**

Run: `cargo test -p rock-lib --test integration immutable_binding_cannot_call_mut_receiver_method shared_reference_cannot_call_mut_receiver_method mutable_binding_can_call_mut_receiver_method -- --nocapture`

Expected: all three tests pass.

- [ ] **Step 8: Run focused selection/lowering tests**

Run: `cargo test -p rock-lib selection lower_method_signature_injects_mut_and_move_self_receiver_types -- --nocapture`

Expected: all matching tests pass. Update expected variant names only where the semantics are unchanged.

---

### Task 2: MIR Ownership Analysis Skeleton And Temporary Metadata

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing temporary escape integration tests**

Add tests to `lib/tests/integration.rs`:

```rust
#[test]
fn returning_reference_to_call_temporary_is_rejected() {
    compile_should_fail(
        r#"
make_value = -> 1

bad: () -> &I64
bad = -> &make_value!

main = -> 0
"#,
        "temporary",
    );
}

#[test]
fn returning_reference_to_local_is_rejected() {
    compile_should_fail(
        r#"
bad: () -> &I64
bad = ->
    value = 1
    &value

main = -> 0
"#,
        "does not live long enough",
    );
}

#[test]
fn returning_input_reference_is_allowed() {
    let output = compile_and_run(
        r#"
id_ref: &I64 -> &I64
id_ref = value -> value

main = ->
    value = 7
    ((*id_ref &value)).println!
    0
"#,
    );
    assert_eq!(output.trim(), "7");
}
```

- [ ] **Step 2: Run temporary escape tests and verify red**

Run: `cargo test -p rock-lib --test integration returning_reference_to_call_temporary_is_rejected returning_reference_to_local_is_rejected returning_input_reference_is_allowed -- --nocapture`

Expected: at least one negative test compiles or fails without the intended diagnostic.

- [ ] **Step 3: Add MIR ownership metadata types**

In `lib/src/mir/mod.rs`, add compact metadata types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceOrigin {
    Param(Local),
    Local(Local),
    Temporary(Local),
    Static,
    UnknownExternal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirOwnershipMetadata {
    pub reference_origins: Vec<(Local, ReferenceOrigin)>,
    pub temporary_locals: Vec<Local>,
}
```

Add `ownership: MirOwnershipMetadata` to `MirFunction` if local to functions, or to `MirBackendMetadata` if current builders can only produce a program-level side table. Prefer function-local metadata.

- [ ] **Step 4: Populate reference origins during MIR construction**

In `lib/src/mir/builder/expr.rs`, when lowering `HirExprKind::Ref`, record the target origin:

```rust
let origin = self.reference_origin_for_place_or_expr(&inner_expr);
self.record_reference_origin(result_local, origin);
```

Use these rules:

- referencing a parameter place records `ReferenceOrigin::Param(local)`.
- referencing a user local records `ReferenceOrigin::Local(local)`.
- referencing a call result or compiler temporary records `ReferenceOrigin::Temporary(local)`.
- string literals record `ReferenceOrigin::Static`.

- [ ] **Step 5: Add ownership validator entry point**

In `lib/src/mir/borrowck/mod.rs`, add an explicit entry point:

```rust
impl MirBorrowChecker {
    pub fn validate_ownership_core(&mut self, function: &MirFunction) -> Result<(), BorrowCheckError> {
        self.validate_moves_and_initialization(function)?;
        self.validate_reference_escapes(function)?;
        self.validate_drop_obligations(function)?;
        Ok(())
    }
}
```

Initially route existing move/init validation through `validate_moves_and_initialization`. Keep behavior unchanged except for the new escape checks.

- [ ] **Step 6: Reject escaping local and temporary references**

Implement `validate_reference_escapes` in `lib/src/mir/borrowck/mod.rs`:

```rust
fn validate_reference_escapes(&self, function: &MirFunction) -> Result<(), BorrowCheckError> {
    for return_operand in self.return_operands(function) {
        if let Some(origin) = self.reference_origin_for_operand(function, return_operand) {
            match origin {
                ReferenceOrigin::Temporary(_) => return Err(BorrowCheckError::new("Cannot return reference to temporary value")),
                ReferenceOrigin::Local(local) if !self.local_is_param(function, local) => {
                    return Err(BorrowCheckError::new("Cannot return reference to local value that does not live long enough"));
                }
                _ => {}
            }
        }
    }
    Ok(())
}
```

Use existing MIR return-place assignment patterns to implement `return_operands`. Do not special-case the test strings; use metadata.

- [ ] **Step 7: Run temporary escape tests and verify green**

Run: `cargo test -p rock-lib --test integration returning_reference_to_call_temporary_is_rejected returning_reference_to_local_is_rejected returning_input_reference_is_allowed -- --nocapture`

Expected: all three tests pass.

---

### Task 3: Drop Obligations And Codegen Enforcement

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/blocks.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/src/codegen/mir_llvm/terminator.rs`
- Test: `lib/src/codegen/mir_llvm/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write/confirm failing missing drop glue unit test**

In `lib/src/codegen/mir_llvm/mod.rs`, ensure there is a unit test with this assertion shape:

```rust
#[test]
fn mir_drop_without_required_glue_is_codegen_error() {
    let err = compile_test_mir_with_direct_drop_but_no_glue();
    assert!(err.to_string().contains("Missing drop glue"));
}
```

If a similar test exists, adjust it to fail when `compile_mir_drop_terminator` silently branches onward.

- [ ] **Step 2: Run missing glue test and verify red**

Run: `cargo test -p rock-lib mir_drop_without_required_glue_is_codegen_error -- --exact --nocapture`

Expected: test fails or must be updated because codegen currently treats absent drop glue as no-op.

- [ ] **Step 3: Add drop obligation kind metadata**

In `lib/src/mir/mod.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropObligationKind {
    Direct,
    Structural,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirDropObligation {
    pub place: Place,
    pub ty: TypeId,
    pub kind: DropObligationKind,
}
```

Attach obligations to `MirFunction` metadata if possible. If obligations are program-level, include `MirFunctionId` in the record.

- [ ] **Step 4: Record direct drop obligations during drop elaboration**

In `lib/src/mir/builder/blocks.rs`, when emitting a direct drop terminator, record:

```rust
self.record_drop_obligation(place.clone(), self.type_id_for(ty), DropObligationKind::Direct);
```

Record structural obligations before recursing into fields/elements/payloads:

```rust
self.record_drop_obligation(place.clone(), self.type_id_for(ty), DropObligationKind::Structural);
```

Do not require codegen to lower structural obligations directly if they are elaborated into child direct drops.

- [ ] **Step 5: Validate direct drop obligations in ownership core**

In `lib/src/mir/borrowck/mod.rs`, implement:

```rust
fn validate_drop_obligations(&self, function: &MirFunction) -> Result<(), BorrowCheckError> {
    for obligation in &function.ownership.drop_obligations {
        if obligation.kind == DropObligationKind::Direct
            && !self.drop_glue_contains(obligation.ty)
        {
            return Err(BorrowCheckError::new("Missing required drop glue for direct Drop obligation"));
        }
    }
    Ok(())
}
```

Wire `drop_glue_contains` to MIR backend metadata or an ownership validator context that can see `MirBackendMetadata.drop_glue`.

- [ ] **Step 6: Make codegen require direct drop glue**

In `lib/src/codegen/mir_llvm/terminator.rs`, replace the optional branch with an error:

```rust
let symbol = self
    .backend_metadata
    .drop_glue_backend_symbol(place_ty)
    .ok_or_else(|| CodegenError::from(format!(
        "Missing drop glue for MIR drop of type {:?} in MIR function '{}'",
        place_ty, function.name
    )))?
    .to_string();
```

Keep the existing return-place skip before this lookup.

- [ ] **Step 7: Run drop/codegen tests and verify green**

Run: `cargo test -p rock-lib mir_drop_without_required_glue_is_codegen_error partial_move_from_direct_drop_type_is_rejected -- --nocapture`

Expected: tests pass.

---

### Task 4: Artifact Specialization Predicate

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/crate_system/context.rs`
- Modify: `lib/src/mono/external.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/crate_artifact/tests.rs`
- Test: `lib/src/mono/external.rs`

- [ ] **Step 1: Write failing product test for generic method in non-generic impl**

In `lib/src/products.rs` tests, add:

```rust
#[test]
fn compiler_products_preserve_non_generic_impl_with_generic_method_body() {
    let hir = resolved_hir_with_non_generic_impl_generic_method();
    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let impl_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(42)));
    assert!(products.bodies.generic_impls.contains_key(&impl_id));
}
```

Build the fixture with one concrete struct `Foo` and `impl Foo` containing method `@id: T -> T` with non-empty `generic_params` and `generic_param_ids`.

- [ ] **Step 2: Run product test and verify red**

Run: `cargo test -p rock-lib compiler_products_preserve_non_generic_impl_with_generic_method_body -- --exact --nocapture`

Expected: test fails because `products.bodies.generic_impls` does not contain the non-generic impl.

- [ ] **Step 3: Add shared specialization predicate**

In `lib/src/products.rs` or a small shared module already visible to product and crate-system code, add:

```rust
pub(crate) fn impl_requires_downstream_specialization(imp: &HirImpl) -> bool {
    !imp.type_generics.is_empty()
        || !imp.trait_generics.is_empty()
        || imp.methods.values().any(|method| {
            !method.generic_params.is_empty() || !method.generic_param_ids.is_empty()
        })
}

pub(crate) fn function_requires_downstream_specialization(function: &HirFunction) -> bool {
    !function.generic_params.is_empty() || !function.generic_param_ids.is_empty()
}
```

- [ ] **Step 4: Use predicate when writing product bodies**

In `lib/src/products.rs`, replace:

```rust
if !imp.type_generics.is_empty() || !imp.trait_generics.is_empty() {
    bodies.generic_impls.insert(id, imp.clone());
}
```

with:

```rust
if impl_requires_downstream_specialization(&imp) {
    bodies.generic_impls.insert(id, imp.clone());
}
```

- [ ] **Step 5: Use predicate in crate-system and mono code**

Replace local duplicate logic in `lib/src/mono/external.rs` and `lib/src/crate_system/extern_store.rs` with the shared predicate. Preserve object-backed concrete impl behavior:

```rust
pub(crate) fn concrete_impl_body_is_object_provided(&self, imp: &HirImpl) -> bool {
    self.is_object_backed()
        && !impl_requires_downstream_specialization(imp)
        && imp.methods.values().all(hir_function_is_codegen_concrete)
}
```

- [ ] **Step 6: Run artifact and mono tests**

Run: `cargo test -p rock-lib compiler_products_preserve_non_generic_impl_with_generic_method_body process_with_crates_does_not_eagerly_emit_dependency_generic_impl_methods -- --nocapture`

Expected: tests pass.

---

### Task 5: Stdlib Allocation Primitives And Vec Deref Policy

**Files:**
- Modify: `stdlib/alloc.rk`
- Modify: `stdlib/raw_buffer.rk`
- Modify: `stdlib/vec.rk`
- Modify: `stdlib/hash_map.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing tests for unsafe deref and allocation overflow hooks**

Add to `lib/tests/integration.rs`:

```rust
#[test]
fn vec_deref_does_not_return_reference_to_temporary_slice() {
    compile_should_fail(
        r#"
> stdlib::vec::Vec

bad: Vec I64 -> &&[I64]
bad = v -> &v.as_slice!

main = -> 0
"#,
        "temporary",
    );
}
```

Add an overflow test only if the current language can express a capacity near `I64` max without excessive allocation. If not expressible without executing a huge allocation, rely on unit-level stdlib helper behavior through small negative arguments.

- [ ] **Step 2: Run deref/temporary test and verify red**

Run: `cargo test -p rock-lib --test integration vec_deref_does_not_return_reference_to_temporary_slice -- --exact --nocapture`

Expected: test fails until Task 2 escape validation is active.

- [ ] **Step 3: Add checked arithmetic helpers**

In `stdlib/alloc.rk`, add helper functions using conservative aborts:

```rock
checked_add_i64: I64 -> I64 -> I64
< checked_add_i64 = a, b ->
    result = a + b
    if b > 0 && result < a
        exit (1 as I32)
    if b < 0 && result > a
        exit (1 as I32)
    result

checked_mul_i64: I64 -> I64 -> I64
< checked_mul_i64 = a, b ->
    if a < 0 || b < 0
        exit (1 as I32)
    if a != 0
        result = a * b
        if (result / a) != b
            exit (1 as I32)
        result
    else
        0

checked_grow_capacity: I64 -> I64
< checked_grow_capacity = old_cap ->
    if old_cap == 0
        4
    else
        checked_mul_i64 old_cap, 2
```

- [ ] **Step 4: Use checked helpers in RawBuffer, Vec, and HashMap**

In `stdlib/raw_buffer.rk`, replace `bytes = elem_size * cap` with:

```rock
bytes = checked_mul_i64 elem_size, cap
```

In `stdlib/vec.rk`, replace length and capacity arithmetic with:

```rock
new_len = checked_add_i64 self.raw_len, 1
new_cap = checked_grow_capacity old_cap
```

In `stdlib/hash_map.rk`, replace `old_cap * 2` with:

```rock
new_cap = if old_cap == 0
    8
else
    checked_mul_i64 old_cap, 2
```

Add imports for the new helpers in each file.

- [ ] **Step 5: Remove unsafe Vec Deref implementation for this branch**

In `stdlib/vec.rk`, remove:

```rock
impl Deref for Vec T
    type Target = &[T]
    @deref = -> &self.as_slice!
```

Update tests/examples that relied on implicit Vec deref to call `as_slice!` explicitly.

- [ ] **Step 6: Run focused stdlib tests**

Run: `cargo test -p rock-lib --test integration test_stdlib_vec test_hash_map test_stdlib_box vec_deref_does_not_return_reference_to_temporary_slice -- --nocapture`

Expected: all focused tests pass.

---

### Task 6: Concurrent-Safe Stdlib Test Artifact Cache

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Write cache helper tests**

Keep `stdlib_cache_key_changes_when_stdlib_source_changes`. Add a partial-cache test:

```rust
#[test]
fn stdlib_cache_rejects_missing_ready_marker() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = test_temp_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("stdlib.rkca"), b"not a real artifact").unwrap();
    std::fs::write(dir.join("stdlib.o"), b"object").unwrap();
    assert!(!stdlib_cache_entry_is_ready(&dir.join("stdlib.rkca"), &dir.join("stdlib.o"), &dir.join("ready")));
}
```

- [ ] **Step 2: Run cache tests and verify red**

Run: `cargo test -p rock-lib --test integration stdlib_cache_ -- --nocapture`

Expected: the new test fails until readiness helper exists.

- [ ] **Step 3: Add lock and ready-marker helpers**

In `lib/tests/integration.rs`, add:

```rust
struct CacheLock {
    path: PathBuf,
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn acquire_cache_lock(path: &Path) -> CacheLock {
    loop {
        match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(_) => return CacheLock { path: path.to_path_buf() },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => std::thread::sleep(std::time::Duration::from_millis(25)),
            Err(err) => panic!("failed to acquire stdlib cache lock {}: {}", path.display(), err),
        }
    }
}

fn stdlib_cache_entry_is_ready(artifact_path: &Path, object_path: &Path, ready_path: &Path) -> bool {
    ready_path.is_file()
        && object_path.is_file()
        && artifact_path.is_file()
        && rock_lib::products::CompilerProducts::read_artifact_from_path(artifact_path).is_ok()
}
```

- [ ] **Step 4: Build in temp dir and publish atomically**

In `stdlib_artifact_path`, use:

```rust
let ready_path = artifact_dir.join("ready");
let lock_path = artifact_dir.with_extension("lock");
if stdlib_cache_entry_is_ready(&artifact_path, &object_path, &ready_path) {
    return artifact_path;
}
let _lock = acquire_cache_lock(&lock_path);
if stdlib_cache_entry_is_ready(&artifact_path, &object_path, &ready_path) {
    return artifact_path;
}
let build_dir = artifact_dir.with_extension(format!("building-{}", std::process::id()));
let _ = std::fs::remove_dir_all(&build_dir);
std::fs::create_dir_all(&build_dir).unwrap();
```

Compile into `build_dir`, write `stdlib.rkca` and `stdlib.o` there, validate the artifact, then publish:

```rust
let _ = std::fs::remove_dir_all(&artifact_dir);
std::fs::rename(&build_dir, &artifact_dir).unwrap();
std::fs::write(&ready_path, b"ready").unwrap();
```

If rename across filesystems fails in practice, replace with copy-then-marker while holding the lock.

- [ ] **Step 5: Run cache tests and reuse timing check**

Run: `cargo test -p rock-lib --test integration stdlib_cache_ -- --nocapture`

Then run twice: `cargo test -p rock-lib --test integration test_hello_world -- --exact`

Expected: cache tests pass; second exact run avoids stdlib rebuild and remains sub-second on a warm cache.

---

### Task 7: Final Integration And Verification

**Files:**
- All touched files

- [ ] **Step 1: Search for forbidden leftovers**

Run: `rg "EqRef|HashRef|Missing drop glue.*return|&self\.as_slice" lib stdlib docs/superpowers/specs docs/superpowers/plans`

Expected: no stale `EqRef`/`HashRef`; no unsafe `&self.as_slice!`; no silent missing-glue branch; no placeholder comments in the plan/spec.

- [ ] **Step 2: Run focused ownership suites**

Run: `cargo test -p rock-lib --test integration immutable_binding_cannot_call_mut_receiver_method shared_reference_cannot_call_mut_receiver_method returning_reference_to_call_temporary_is_rejected returning_reference_to_local_is_rejected vec_deref_does_not_return_reference_to_temporary_slice -- --nocapture`

Expected: all pass.

- [ ] **Step 3: Run artifact and mono focused tests**

Run: `cargo test -p rock-lib compiler_products_preserve_non_generic_impl_with_generic_method_body process_with_crates_does_not_eagerly_emit_dependency_generic_impl_methods process_with_crates_does_not_emit_unused_dependency_drop_glue_for_interned_types -- --nocapture`

Expected: all pass.

- [ ] **Step 4: Run stdlib container focused tests**

Run: `cargo test -p rock-lib --test integration test_stdlib_vec test_hash_map test_stdlib_box -- --nocapture`

Expected: all pass.

- [ ] **Step 5: Run formatting and whitespace checks**

Run: `cargo fmt --all --check`

Run: `git diff --check`

Expected: no output from either command.

- [ ] **Step 6: Run full library tests**

Run: `cargo test -p rock-lib`

Expected: all tests pass.

- [ ] **Step 7: Run artifact CLI tests**

Run: `cargo test -p rock artifact`

Expected: all artifact tests pass.

- [ ] **Step 8: Review final diff against develop**

Run: `git diff --stat develop`

Run: `git diff develop -- lib/src/selection lib/src/lower lib/src/mir lib/src/codegen lib/src/products.rs lib/src/crate_system lib/src/mono stdlib lib/tests/integration.rs`

Expected: diff matches the approved spec and contains no unrelated refactors.

- [ ] **Step 9: Commit only if explicitly approved**

If the user has explicitly requested a commit, run:

```bash
git status --short
git diff --stat
git log --oneline -10
git add docs/superpowers/specs/2026-06-22-compiler-ownership-architecture-rewrite-design.md docs/superpowers/plans/2026-06-22-compiler-ownership-architecture-rewrite.md lib stdlib rock-shared rock
git commit -m "refactor: establish MIR ownership core"
```

Expected: commit succeeds and `git status --short` is clean except for unrelated user changes.

## Self-Review Notes

- Spec coverage: receiver adjustment is Task 1; ownership core and temporary escapes are Task 2; drop obligations are Task 3; artifact predicate is Task 4; stdlib allocation and Vec deref are Task 5; cache safety is Task 6; final verification is Task 7.
- Placeholder scan: this plan intentionally contains no placeholder markers or unspecified implementation steps.
- Type consistency: receiver adjustment variants are defined once in Task 1 and reused by later lowering tasks; ownership metadata is introduced before escape/drop validators consume it.
