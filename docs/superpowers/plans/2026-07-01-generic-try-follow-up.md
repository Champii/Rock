# Generic Try Follow-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the temporary `Result`/`Option` `?` lowering with fully generic `Try` and `FromResidual` dispatch, and make `From`/`Into` conversions fully usable.

**Architecture:** `?` becomes a first-class HIR expression carrying selected `Try::branch` and `FromResidual::from_residual` impl targets plus `ControlFlow` variant metadata. MIR lowering evaluates the carrier, calls selected `branch`, switches on `ControlFlow`, returns through selected `from_residual` on break, and continues with the output payload. `From` remains the canonical conversion trait; `Into` is implemented as a blanket receiver trait where `U: From T`.

**Tech Stack:** Rust compiler implementation in `lib/`, Rock stdlib sources in `stdlib/`, integration tests in `lib/tests/integration.rs`, verified with focused `cargo test -p rock-lib --test integration ... -- --exact` and full `cargo test -p rock-lib`.

---

## File Structure

- Modify `stdlib/convert.rk`: add `Into T` with blanket `impl Into U for T where U: From T`.
- Modify `lib/src/hir/mod.rs`: add `HirExprKind::Try` with carrier expression, selected `branch` method target, selected `from_residual` call target, output/residual/return types, and `ControlFlow` variant locations.
- Modify `lib/src/lower/control_flow/secondary.rs`: replace enum-name special-casing in `lower_try_interogation` with trait-based selection for `Try` and `FromResidual`.
- Modify `lib/src/selection/service.rs`: add helper(s) to select a static trait function from an impl by receiver/trait args, or expose enough impl/method metadata for lowering to build the call target.
- Modify HIR traversal/serialization files as required by `cargo check`: `lib/src/hir/type_ids.rs`, `lib/src/products.rs`, `lib/src/lower/traits/conformance.rs`, `lib/src/codegen/metadata.rs`, and any other exhaustive `HirExprKind` matches.
- Modify `lib/src/mir/builder/expr.rs`: lower `HirExprKind::Try` directly to MIR branch/return control flow.
- Modify `lib/tests/integration.rs`: add red-green tests for `Into`, `Result` error conversion through `From`, custom carrier `Try`, custom `FromResidual`, and diagnostics.

---

### Task 1: From/Into Conversion Traits

**Files:**
- Modify: `stdlib/convert.rk`
- Test: `lib/tests/integration.rs`

- [x] **Step 1: Add failing `Into` integration test**

Add this test near the current try/conversion tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_stdlib_into_uses_from_blanket_impl() {
    let output = compile_and_run(
        r#"
struct Small
    value: I64

struct Big
    value: I64

impl From Small for Big
    from = small -> Big { value: small.value + 1 }

main = ->
    small = Small { value: 41 }
    big: Big = small.into!
    big.value.println!
    0
"#,
    );
    assert_eq!(output.trim(), "42");
}
```

- [x] **Step 2: Verify it fails**

Run: `cargo test -p rock-lib --test integration test_stdlib_into_uses_from_blanket_impl -- --exact`

Expected: FAIL because `Into` is not defined.

- [x] **Step 3: Add `Into` to `stdlib/convert.rk`**

Add:

```rock
< trait Into T
    ~@into: T

impl Into U for T where U: From T
    ~@into = -> U::from self
```

Use receiver syntax so `small.into!` works. Keep `From` as the only explicit conversion users normally implement.

- [x] **Step 4: Verify `Into` test passes**

Run: `cargo test -p rock-lib --test integration test_stdlib_into_uses_from_blanket_impl -- --exact`

Expected: PASS.

---

### Task 2: Generic Try HIR Shape

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: traversal files found by `cargo check`
- Test: `cargo check -p rock-lib`

- [x] **Step 1: Add `HirExprKind::Try`**

Add this HIR variant:

```rust
Try {
    expr: Box<HirExpr>,
    branch_method: Option<HirMethodCallTarget>,
    branch_self_receiver: Option<SelfReceiverMode>,
    from_residual_target: Option<HirCallTarget>,
    output_ty: Type,
    residual_ty: Type,
    return_ty: Type,
    control_flow_enum: DefId,
    break_variant: HirVariantLocation,
    continue_variant: HirVariantLocation,
}
```

The surrounding `HirExpr.ty` is `output_ty`.

- [x] **Step 2: Run check for exhaustive-match failures**

Run: `cargo check -p rock-lib`

Expected: FAIL with non-exhaustive matches.

- [x] **Step 3: Handle `HirExprKind::Try` in walkers**

For type collection, metadata, products, and conformance traversals, visit `expr` and treat the selected target fields as metadata. Do not generate behavior in these passes.

- [x] **Step 4: Verify check progresses**

Run: `cargo check -p rock-lib`

Expected: PASS or fail only in lowering/MIR code not yet updated in later tasks.

---

### Task 3: Trait-Based Lowering For `?`

**Files:**
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/selection/service.rs`
- Test: `lib/tests/integration.rs`

- [x] **Step 1: Add failing custom carrier test**

Add:

```rust
#[test]
fn test_try_custom_carrier_uses_try_and_from_residual() {
    let output = compile_and_run(
        r#"
enum MyFlow T
    Value T
    Stop I64

enum MyResidual
    Stop I64

impl Try for MyFlow T
    type Output = T
    type Residual = MyResidual
    ~@branch = ->
        match self
            MyFlow::Value value => ControlFlow::Continue value
            MyFlow::Stop code => ControlFlow::Break (MyResidual::Stop code)

impl FromResidual MyResidual for MyFlow T
    from_residual = residual ->
        match residual
            MyResidual::Stop code => MyFlow::Stop code

next: Bool -> MyFlow I64
next = ok ->
    if ok
        MyFlow::Value 41
    else
        MyFlow::Stop 7

compute: Bool -> MyFlow I64
compute = ok ->
    value = next ok?
    MyFlow::Value (value + 1)

main = ->
    success = compute true
    failure = compute false
    match success
        MyFlow::Value value => value.println!
        MyFlow::Stop code => code.println!
    match failure
        MyFlow::Value value => value.println!
        MyFlow::Stop code => code.println!
    0
"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["42", "7"]);
}
```

- [x] **Step 2: Verify it fails**

Run: `cargo test -p rock-lib --test integration test_try_custom_carrier_uses_try_and_from_residual -- --exact`

Expected: FAIL because current lowering special-cases only `Result` and `Option`.

- [x] **Step 3: Select `Try::branch` generically**

In `lower_try_interogation`, resolve the `Try` trait by canonical item name, select required method `branch` for the carrier using `select_required_trait_method`, record pending impl bounds, and read `Output`/`Residual` associated type projections from the selected impl. Build `HirExprKind::Try` instead of a HIR `Match`.

- [x] **Step 4: Select `FromResidual::from_residual` generically**

Resolve `FromResidual`, select the impl for the enclosing return type with trait arg `[residual_ty]`, find the `from_residual` function in the selected impl, and store its `HirCallTarget` in `HirExprKind::Try`. If selection fails, emit a diagnostic mentioning `FromResidual`, the residual type, and the return type.

- [x] **Step 5: Resolve `ControlFlow` metadata**

Resolve `ControlFlow` enum and store `Break`/`Continue` `HirVariantLocation`s in the HIR try node. If unavailable, emit a structured diagnostic.

---

### Task 4: MIR Lowering For Generic Try

**Files:**
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/tests/integration.rs`

- [x] **Step 1: Lower `HirExprKind::Try`**

MIR lowering should:

1. Lower carrier expression to temp.
2. Call selected `branch` method into a `ControlFlow residual, output` temp.
3. Switch on the `ControlFlow` discriminant using stored variant IDs.
4. On `Continue`, move/extract payload into `dest` and jump to merge.
5. On `Break`, extract residual, call selected `from_residual` into return place local `0`, and terminate through normal cleanup return.

- [x] **Step 2: Verify custom carrier test passes**

Run: `cargo test -p rock-lib --test integration test_try_custom_carrier_uses_try_and_from_residual -- --exact`

Expected: PASS.

---

### Task 5: Result Error Conversion Through From

**Files:**
- Modify: `lib/tests/integration.rs`
- Modify lowering/selection only if test exposes a bug.

- [x] **Step 1: Add failing error conversion test**

Add:

```rust
#[test]
fn test_try_result_error_conversion_uses_from() {
    let output = compile_and_run(
        r#"
enum SmallError
    Bad I64

enum BigError
    Wrapped I64

impl From SmallError for BigError
    from = err ->
        match err
            SmallError::Bad code => BigError::Wrapped (code + 1)

fail: -> Result I64, SmallError
fail = -> Result::Err (SmallError::Bad 6)

compute: -> Result I64, BigError
compute = ->
    value = fail!?
    Result::Ok value

main = ->
    result = compute!
    match result
        Result::Ok value => value.println!
        Result::Err err =>
            match err
                BigError::Wrapped code => code.println!
    0
"#,
    );
    assert_eq!(output.trim(), "7");
}
```

- [x] **Step 2: Verify it fails before generic `FromResidual` selection**

Run: `cargo test -p rock-lib --test integration test_try_result_error_conversion_uses_from -- --exact`

Expected: FAIL before Task 3/4, PASS after generic `FromResidual` and `From` constraints work.

- [x] **Step 3: Verify it passes**

Run: `cargo test -p rock-lib --test integration test_try_result_error_conversion_uses_from -- --exact`

Expected: PASS.

---

### Task 6: Diagnostics And Regression Coverage

**Files:**
- Modify: `lib/tests/integration.rs`
- Modify diagnostics in `lib/src/lower/control_flow/secondary.rs`

- [x] **Step 1: Add incompatible return carrier diagnostic test**

Add:

```rust
#[test]
fn test_try_option_to_result_without_from_residual_reports_diagnostic() {
    compile_should_fail(
        r#"
maybe: -> Option I64
maybe = -> Option::None

main: -> Result I64, I64
main = ->
    value = maybe!?
    Result::Ok value
"#,
        "FromResidual",
    );
}
```

- [x] **Step 2: Add regression runs for existing Result/Option tests**

Run:

```bash
cargo test -p rock-lib --test integration test_try_result_success_path_unwraps -- --exact
cargo test -p rock-lib --test integration test_try_result_error_path_returns_early -- --exact
cargo test -p rock-lib --test integration test_try_option_some_path_unwraps -- --exact
cargo test -p rock-lib --test integration test_try_option_none_path_returns_early -- --exact
cargo test -p rock-lib --test integration test_try_non_carrier_reports_diagnostic -- --exact
```

Expected: PASS.

---

### Task 7: Full Verification And Commit

**Files:**
- All modified files.

- [x] **Step 1: Format and whitespace checks**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: both pass.

- [x] **Step 2: Full package tests**

Run: `cargo test -p rock-lib > /tmp/rock-lib-generic-try-follow-up-tests.log 2>&1`

Expected: all tests pass.

- [ ] **Step 3: Close follow-up bead**

Run: `bd close new_lang2-68p --reason "Implemented generic Try/FromResidual dispatch and From/Into conversions" --json`

Expected: bead closes successfully.

- [ ] **Step 4: Commit and push**

Run:

```bash
git status --short
git add docs/superpowers/specs/2026-07-01-generic-try-short-circuit-design.md docs/superpowers/plans/2026-07-01-generic-try-follow-up.md stdlib lib
git commit -m "feat: make try short-circuit fully generic"
git push
```

Expected: branch `generic-try-short-circuit` is pushed and clean.

---

## Self-Review Notes

- Spec coverage: covers generic carrier dispatch, `FromResidual`, `From`/blanket `Into`, `Result` error conversion, custom carrier behavior, and diagnostics.
- Placeholder scan: no TBD/TODO placeholders remain.
- Type consistency: plan consistently uses `Try::Output`, `Try::Residual`, `FromResidual Residual`, `From Source for Target`, and blanket `Into Target for Source where Target: From Source`.
