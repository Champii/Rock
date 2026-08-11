# Stdlib Deref And Vec Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real stdlib `Deref` trait, make `Vec` expose Rust-like borrowed-slice semantics, route `Vec[i]` through shared autoderef receiver resolution into `[T]` indexing, and add the missing parser/integration coverage while keeping `cargo test -p rock-lib` green.

**Architecture:** Keep operator lowering on the existing trait-style `MethodCall(..., "index", ...)` plus outer `Deref`, but replace the receiver-specific `Deref`/`Index` checks with a shared autoderef candidate builder. In stdlib, make `Vec` store a stable slice view, change `get` to `Option &T`, expose `Deref<Target = [T]>`, and let `Vec[i]` inherit bounds-checked `[T]` indexing instead of adding a direct `Vec: Index` impl.

**Tech Stack:** Rust 2021, Rock stdlib `.rk` modules, Rock parser tests, Rock integration tests, lowering/type-resolution/codegen pipeline, `cargo test -p rock-lib`.

---

## File Map

- Create: `stdlib/deref.rk` — declare the real stdlib `Deref` trait.
- Modify: `stdlib/lib.rk` — export the new `deref` module.
- Modify: `stdlib/prelude.rk` — re-export `Deref` beside `Index` and the other traits.
- Modify: `stdlib/vec.rk` — add a stable slice-view field, change `get` to `Option &T`, and implement `Deref<Target = [T]>`.
- Modify: `stdlib/vec.rk` — update `Show for Vec T` to dereference `Vec.get` results before calling `show`.
- Modify: `lib/src/lower/intrinsics.rs` — make `~MakeArr` infer `[T]` from a `*T` argument instead of always returning `[U8]`.
- Modify: `lib/src/lower/types_helpers/helpers.rs` — add the shared autoderef receiver helper(s).
- Modify: `lib/src/lower/expression.rs` — make unary `*` use the shared autoderef step after built-in ref/pointer fast paths.
- Modify: `lib/src/lower/control_flow/secondary.rs` — make `[]` choose the first autoderef receiver candidate that supports `Index`.
- Modify: `lib/src/codegen/expr/access.rs` — bounds-check all `Type::Array(_)` indexing, not just `[U8]`.
- Modify: `lib/src/crate_artifact/tests.rs` — verify the stdlib artifact exports `Deref` through the interface and prelude export map.
- Modify: `lib/src/parser/items/tests/path/test_type_path.rs` — add explicit parser coverage for `Self::Target` and `Self::Output`.
- Modify: `lib/tests/integration.rs` — add new `Deref`/`Vec` tests, a runtime-status helper for out-of-bounds indexing, and update existing `Vec.get` callers to the reference-returning API.

### Task 1: Add Parser Coverage And The Stdlib `Deref` Surface

**Files:**
- Create: `stdlib/deref.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/parser/items/tests/path/test_type_path.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add the missing parser tests, keep the integration regression, and add a real stdlib export test**

In `lib/src/parser/items/tests/path/test_type_path.rs`, add these two tests below `test_type_path_keeps_enum_variant_segments`:

```rust
#[test]
fn test_type_path_accepts_self_target_projection() {
    let input = "Self::Target";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, path) = type_path.process(ParseCtx::from(&tokens, &config)).unwrap();

    assert_eq!(path.path.len(), 2);
    assert_eq!(rest.len(), 0);
}

#[test]
fn test_type_path_accepts_self_output_projection() {
    let input = "Self::Output";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, path) = type_path.process(ParseCtx::from(&tokens, &config)).unwrap();

    assert_eq!(path.path.len(), 2);
    assert_eq!(rest.len(), 0);
}
```

In `lib/src/crate_artifact/tests.rs`, add this test near `test_build_stdlib_artifact_exports`:

```rust
#[test]
fn test_build_stdlib_artifact_exports_deref_trait() {
    let _guard = artifact_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let stdlib_dir = workspace_root().join("stdlib");
    let mut ctx = CrateContext::new();
    ctx.load_crate_from_dir(stdlib_dir).unwrap();

    let artifact = ctx.build_artifact("stdlib").unwrap();

    assert_eq!(
        artifact.prelude_exports.get("Deref").map(String::as_str),
        Some("stdlib::deref::Deref"),
    );
    assert!(artifact.interface.traits.contains_key("stdlib::deref::Deref"));
}
```

In `lib/tests/integration.rs`, add this new integration test next to the existing `Index` / `Deref` associated-type tests:

```rust
#[test]
fn test_stdlib_deref_trait_is_available_from_prelude() {
    let output = compile_and_run(
        r#"
struct Point
    < x: I64

struct Box T
    < value: T

impl Deref for Box T
    type Target = T
    @deref = -> &@value

main = ->
    boxed = Box
        value: Point
            x: 12
    point: Point = *boxed
    point.x.println!
    0
"#,
    );

    assert_eq!(output.trim(), "12");
}
```

- [ ] **Step 2: Run the focused tests to establish the red/green baseline**

Run:

```bash
cargo test -p rock-lib parser::items::tests::path::test_type_path::test_type_path_accepts_self_target_projection -- --exact
cargo test -p rock-lib parser::items::tests::path::test_type_path::test_type_path_accepts_self_output_projection -- --exact
cargo test -p rock-lib crate_artifact::tests::test_build_stdlib_artifact_exports_deref_trait -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_deref_trait_is_available_from_prelude -- --exact --nocapture
```

Expected:
- the two parser tests should PASS immediately if the parser already accepts the syntax; keep them as explicit regression coverage either way;
- the new artifact test should FAIL because stdlib does not yet export `Deref` from the interface or prelude map;
- the integration test may already PASS because the current compiler accepts `impl Deref for ...` even when no declared `Deref` trait has been loaded, so it is a regression test but not the red gate for this task.

- [ ] **Step 3: Add the real stdlib `Deref` trait and export it**

Create `stdlib/deref.rk` with exactly this content:

```rock
// Deref trait - provides `*value` through an associated target type.

< trait Deref
    type Target
    @deref: Self -> &Self::Target
```

In `stdlib/lib.rk`, insert the new module declaration after `< mod show` and before `< mod index`:

```rock
// Deref trait - dereferencing with an associated target type
< mod deref
```

In `stdlib/prelude.rk`, add the trait re-export alongside the other trait exports:

```rock
< stdlib::deref::*
```

- [ ] **Step 4: Re-run the focused parser and prelude tests**

Run:

```bash
cargo test -p rock-lib parser::items::tests::path::test_type_path::test_type_path_accepts_self_target_projection -- --exact
cargo test -p rock-lib parser::items::tests::path::test_type_path::test_type_path_accepts_self_output_projection -- --exact
cargo test -p rock-lib crate_artifact::tests::test_build_stdlib_artifact_exports_deref_trait -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_deref_trait_is_available_from_prelude -- --exact --nocapture
```

Expected: all four commands PASS.

- [ ] **Step 5: Do not commit unless the user explicitly asks for one**

If the user requests a checkpoint commit for this task, run:

```bash
git add lib/src/crate_artifact/tests.rs lib/src/parser/items/tests/path/test_type_path.rs lib/tests/integration.rs stdlib/deref.rk stdlib/lib.rk stdlib/prelude.rk
git commit -m "add stdlib deref trait surface"
```

### Task 2: Make `Vec` Expose A Stable Borrowed Slice And Reference-Returning `get`

**Files:**
- Modify: `lib/src/lower/intrinsics.rs`
- Modify: `stdlib/vec.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add the failing integration test for `Vec.get -> Option &T`**

In `lib/tests/integration.rs`, add this test near the existing `Vec` tests:

```rust
#[test]
fn test_vec_get_returns_optional_reference() {
    let output = compile_and_run(
        r#"
main = ->
    v = Vec::new!
    v.push 10
    v.push 20

    match (v.get 0)
        Option::Some val => *val .println!
        Option::None => -1 .println!

    match (v.get 5)
        Option::Some val => *val .println!
        Option::None => -1 .println!

    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["10", "-1"]);
}
```

- [ ] **Step 2: Run the new test to verify the current `Vec.get` contract is wrong**

Run: `cargo test -p rock-lib --test integration test_vec_get_returns_optional_reference -- --exact --nocapture`

Expected: FAIL with a type error in the `Option::Some val => *val .println!` branch because `v.get` still returns copied `I64` values instead of references.

- [ ] **Step 3: Make `~MakeArr` infer `[T]` from a `*T` argument**

In `lib/src/lower/intrinsics.rs`, replace the hard-coded `MakeArr` return type branch with this logic:

```rust
"MakeArr" => {
    if let Some(HirExpr {
        ty: Type::Pointer(inner),
        ..
    }) = args.first()
    {
        return Type::Array(inner.clone());
    }
    return Type::Array(Box::new(Type::U8));
}
```

Keep the existing `[U8]` fallback for callers that do not carry a typed pointer argument yet.

- [ ] **Step 4: Rework `Vec` to store a stable slice view, return references from `get`, and expose `Deref<Target = [T]>`**

In `stdlib/vec.rk`, change the struct and impl blocks to this shape:

```rock
< struct Vec T
    raw_ptr: *T
    raw_len: I64
    raw_cap: I64
    raw_view: [T]

impl Vec T
    @len = -> self.raw_len

    @push = elem ->
        elem_size = stdlib::mem::size_of elem
        new_len = self.raw_len + 1
        new_cap = if self.raw_cap == 0
            4
        else if new_len > self.raw_cap
            self.raw_cap * 2
        else
            self.raw_cap
        new_ptr = if new_cap != self.raw_cap
            (realloc (self.raw_ptr as *U8, new_cap * elem_size)) as *T
        else
            self.raw_ptr
        unsafe new_ptr[self.raw_len] = elem
        self.raw_ptr = new_ptr
        self.raw_len = new_len
        self.raw_cap = new_cap
        self.raw_view = unsafe ~MakeArr new_ptr, new_len

    @get = i ->
        if i >= 0 && i < self.raw_len
            Option::Some (&(self.raw_view[i]))
        else
            Option::None

    @set = i, val ->
        unsafe self.raw_ptr[i] = val

    new = ->
        ptr = (malloc 0) as *T
        Vec T
            raw_ptr: ptr
            raw_len: 0
            raw_cap: 0
            raw_view: unsafe ~MakeArr ptr, 0

impl Deref for Vec T
    type Target = [T]
    @deref = -> &@raw_view
```

The important details are:
- `raw_view` is the stable borrowed slice location that `@deref` can safely reference;
- `@push` keeps `raw_view` synchronized whenever `raw_ptr` or `raw_len` changes;
- `@get` returns `Option::Some (&(self.raw_view[i]))`, not a copied element.

- [ ] **Step 5: Update `Show for Vec T` and the existing integration callsites to the reference-returning API**

In `stdlib/vec.rk`, update the existing `impl Show for Vec T` match arm like this:

```rock
elem_str = match (self.get i)
    Option::Some val => (*val).show!
    Option::None => String::from_str ""
```

In `lib/tests/integration.rs`, update the existing `Vec.get` callers so they dereference `Option::Some` values explicitly.

Use these exact replacements in the embedded Rock snippets:

For `test_vec_push`:

```rock
match (v.get 0)
    Option::Some val => *val .println!
    Option::None => 0.println!
match (v.get 1)
    Option::Some val => *val .println!
    Option::None => 0.println!
match (v.get 2)
    Option::Some val => *val .println!
    Option::None => 0.println!
```

For `test_vec_set_get`:

```rock
match (v.get 0)
    Option::Some val => *val .println!
    Option::None => 0.println!
match (v.get 1)
    Option::Some val => *val .println!
    Option::None => 0.println!
match (v.get 2)
    Option::Some val => *val .println!
    Option::None => 0.println!
```

For `test_vec_get_option`:

```rock
match (v.get 0)
    Option::Some val => *val .println!
    Option::None => "None".println!
match (v.get 5)
    Option::Some val => *val .println!
    Option::None => "None".println!
match (v.get 1)
    Option::Some val => *val .println!
    Option::None => 99.println!
match (v.get 99)
    Option::Some val => *val .println!
    Option::None => 99.println!
```

For `test_sieve_of_eratosthenes`, replace the two `unwrap_or` uses with this pattern:

```rock
        is_prime = match (sieve.get i)
            Option::Some val => *val
            Option::None => 0
        if is_prime == 1
```

Use that same `is_prime` block in both loops.

- [ ] **Step 6: Re-run the focused `Vec` tests**

Run:

```bash
cargo test -p rock-lib --test integration test_vec_get_returns_optional_reference -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_push -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_set_get -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_get_option -- --exact --nocapture
cargo test -p rock-lib --test integration test_sieve_of_eratosthenes -- --exact --nocapture
```

Expected: all five tests PASS.

- [ ] **Step 7: Do not commit unless the user explicitly asks for one**

If the user requests a checkpoint commit for this task, run:

```bash
git add lib/src/lower/intrinsics.rs lib/tests/integration.rs stdlib/vec.rk
git commit -m "make vec get return borrowed elements"
```

### Task 3: Add Shared Autoderef Receiver Resolution For Unary `*` And `[]`

**Files:**
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add the failing end-to-end test for `Vec[i]` through `Deref<Target = [T]>`**

In `lib/tests/integration.rs`, add this test near the other associated-type operator tests:

```rust
#[test]
fn test_vec_index_dispatches_through_deref_slice() {
    let output = compile_and_run(
        r#"
main = ->
    v = Vec::new!
    v.push 4
    v.push 7
    v[1].println!
    0
"#,
    );

    assert_eq!(output.trim(), "7");
}
```

- [ ] **Step 2: Run the new `Vec[i]` test and capture the current failure**

Run: `cargo test -p rock-lib --test integration test_vec_index_dispatches_through_deref_slice -- --exact --nocapture`

Expected: FAIL with the current family of lowering errors, typically `No implementation found for operator '[]' on type Vec`, because `[]` only checks the original receiver type.

- [ ] **Step 3: Add shared `Deref` helpers that build autoderef receiver candidates**

In `lib/src/lower/types_helpers/helpers.rs`, change the top import to include `HashSet` and add two helpers near `find_matching_trait_impl`:

```rust
use std::collections::{HashMap, HashSet};
```

```rust
pub(crate) fn apply_trait_deref(&mut self, expr: HirExpr) -> Option<HirExpr> {
    let resolved_ty = self.resolve_projection_type(&self.engine.resolve(&expr.ty));
    let method_func = self
        .find_matching_trait_impl(&resolved_ty, "Deref", &[])
        .and_then(|imp| imp.methods.get("deref"))
        .cloned()?;

    let subst = self.infer_method_substitution(&resolved_ty, &method_func, &[]);
    let return_ty = if subst.is_empty() {
        method_func.ret_type.clone()
    } else {
        method_func.ret_type.substitute_generics(&subst)
    };
    let return_ty = self.resolve_projection_type(&return_ty);
    let Type::Reference { inner, .. } = &return_ty else {
        return None;
    };

    Some(HirExpr {
        ty: (*inner.clone()),
        kind: HirExprKind::Deref(Box::new(HirExpr {
            ty: return_ty,
            kind: HirExprKind::MethodCall(
                Box::new(expr.clone()),
                "deref".to_string(),
                vec![],
                method_func.self_receiver,
            ),
            span: expr.span.clone(),
        })),
        span: expr.span.clone(),
    })
}

pub(crate) fn autoderef_candidates(&mut self, expr: HirExpr) -> Vec<HirExpr> {
    let mut candidates = vec![expr.clone()];
    let mut seen = HashSet::new();
    let mut current = expr;

    seen.insert(self.resolve_projection_type(&self.engine.resolve(&current.ty)));

    for _ in 0..8 {
        let Some(next) = self.apply_trait_deref(current.clone()) else {
            break;
        };

        let resolved_next = self.resolve_projection_type(&self.engine.resolve(&next.ty));
        if !seen.insert(resolved_next) {
            break;
        }

        candidates.push(next.clone());
        current = next;
    }

    candidates
}
```

Keep the recursion bound at `8`; it is a termination guard, not a separate language rule.

- [ ] **Step 4: Reuse the shared helper in unary `*` and `[]` lowering**

In `lib/src/lower/expression.rs`, keep the built-in `&T` and raw `*T` fast paths, then replace the open-coded trait `Deref` lookup with this simpler fallback:

```rust
if let Some(next) = self.autoderef_candidates(inner_hir.clone()).into_iter().nth(1) {
    return next;
}
```

Only keep the existing `Cannot dereference non-pointer type` error when neither the built-in fast paths nor `autoderef_candidates(...).nth(1)` produce a result.

In `lib/src/lower/control_flow/secondary.rs`, keep the existing `TypeVar` / `Generic` constraint path, but for concrete receivers replace the single-type `Index` check with a candidate search:

```rust
let candidates = self.autoderef_candidates(expr.clone());
let chosen = candidates.into_iter().find_map(|candidate| {
    let candidate_ty = self.resolve_projection_type(&self.engine.resolve(&candidate.ty));
    let supports_builtin = candidate_ty.has_builtin_index_impl(&resolved_index_ty);
    let supports_user = self
        .find_matching_trait_impl(
            &candidate_ty,
            "Index",
            std::slice::from_ref(&resolved_index_ty),
        )
        .and_then(|imp| imp.methods.get("index"))
        .is_some();

    if supports_builtin || supports_user {
        Some((candidate, candidate_ty))
    } else {
        None
    }
});
```

Use the chosen receiver expression and chosen receiver type when constructing the lowered method call:

```rust
let (receiver_expr, receiver_ty) = chosen.unwrap();

let method_call = HirExpr {
    ty: Type::Reference {
        mutable: false,
        inner: Box::new(Type::Projection {
            ty: Box::new(receiver_ty),
            trait_name: "Index".to_string(),
            assoc_name: "Output".to_string(),
            trait_args: vec![index.ty.clone()],
        }),
    },
    kind: HirExprKind::MethodCall(
        Box::new(receiver_expr),
        "index".to_string(),
        vec![index],
        Some(crate::ast::SelfReceiverMode::Shared),
    ),
    span: span.clone(),
};
```

Keep the old diagnostic only when there is no matching candidate and the original receiver type is concrete.

- [ ] **Step 5: Re-run the focused autoderef/index tests**

Run:

```bash
cargo test -p rock-lib --test integration test_deref_target_type_drives_unary_deref -- --exact --nocapture
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_index_dispatches_through_deref_slice -- --exact --nocapture
```

Expected: all three commands PASS.

- [ ] **Step 6: Do not commit unless the user explicitly asks for one**

If the user requests a checkpoint commit for this task, run:

```bash
git add lib/src/lower/control_flow/secondary.rs lib/src/lower/expression.rs lib/src/lower/types_helpers/helpers.rs lib/tests/integration.rs
git commit -m "add shared autoderef receiver resolution"
```

### Task 4: Bounds-Check All `[T]` Indexing And Add A Runtime Status Helper

**Files:**
- Modify: `lib/src/codegen/expr/access.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add a runtime-status helper and a failing out-of-bounds integration test**

In `lib/tests/integration.rs`, add this helper next to `compile_and_run`:

```rust
fn compile_and_run_with_status(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tid = std::thread::current().id();
    let dir = std::env::temp_dir().join(format!("rock_test_{:?}_{}", tid, id));
    std::fs::create_dir_all(&dir).unwrap();

    let source_path = dir.join(format!("test_{}.rk", id));
    std::fs::write(&source_path, source).unwrap();

    let config = test_config(source_path.clone(), dir.clone());
    rock_lib::compile(&config).expect("Compilation failed");

    let binary = dir.join(format!("test_{}", id));
    let output = Command::new(&binary)
        .output()
        .expect("Failed to run compiled binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();

    let _ = std::fs::remove_dir_all(&dir);

    (stdout, success)
}
```

Then add this integration test near the new `Vec[i]` test:

```rust
#[test]
fn test_vec_index_out_of_bounds_traps() {
    let (stdout, success) = compile_and_run_with_status(
        r#"
main = ->
    v = Vec::new!
    v.push 10
    v[1].println!
    0
"#,
    );

    assert!(!success);
    assert!(
        stdout.contains("index out of bounds"),
        "stdout was {stdout:?}",
    );
}
```

- [ ] **Step 2: Run the out-of-bounds test to establish the current bounds-check gap**

Run: `cargo test -p rock-lib --test integration test_vec_index_out_of_bounds_traps -- --exact --nocapture`

Expected: FAIL because generic `[T]` indexing still bypasses the structured bounds-check path; `success` may be `true`, or the process may exit without printing the standard `index out of bounds` message.

- [ ] **Step 3: Reuse the existing `[U8]` bounds-check path for all `Type::Array(_)` receivers**

In `lib/src/codegen/expr/access.rs`, replace the `[U8]`-specific bounds check guard with an all-array guard.

Change this:

```rust
if matches!(&resolved_base_ty, Type::Array(inner) if matches!(inner.as_ref(), Type::U8)) {
```

to this:

```rust
if let Type::Array(_) = &resolved_base_ty {
```

Inside that block:
- rename `"str_len"` to `"arr_len"`;
- rename the basic blocks to `"arr_oob"` and `"arr_ok"`;
- keep the same negative-index check, upper-bound check, `puts("index out of bounds")`, `exit(1)`, and `unreachable` sequence.

The body should look like this:

```rust
let len = self
    .builder
    .build_extract_value(arr_struct, 1, "arr_len")
    .map_err(|e| CodegenError::from(format!("Failed to extract array len: {}", e)))?
    .into_int_value();
let zero = self.context.i64_type().const_int(0, false);
let is_neg = self
    .builder
    .build_int_compare(IntPredicate::SLT, idx, zero, "idx_neg")
    .map_err(|e| CodegenError::from(format!("Failed to build neg check: {}", e)))?;
let is_oob = self
    .builder
    .build_int_compare(IntPredicate::SGE, idx, len, "idx_oob")
    .map_err(|e| CodegenError::from(format!("Failed to build oob check: {}", e)))?;
let fail = self
    .builder
    .build_or(is_neg, is_oob, "idx_fail")
    .map_err(|e| CodegenError::from(format!("Failed to build bounds or: {}", e)))?;

let function = self
    .current_function
    .ok_or(CodegenError::from("No current function for bounds check"))?;
let fail_bb = self.context.append_basic_block(function, "arr_oob");
let ok_bb = self.context.append_basic_block(function, "arr_ok");

self.builder
    .build_conditional_branch(fail, fail_bb, ok_bb)
    .map_err(|e| CodegenError::from(format!("Failed to build bounds branch: {}", e)))?;
```

Keep the existing `puts` + `exit` body unchanged apart from the block names and `arr_len` label.

- [ ] **Step 4: Re-run the in-bounds and out-of-bounds indexing tests**

Run:

```bash
cargo test -p rock-lib --test integration test_vec_index_dispatches_through_deref_slice -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_index_out_of_bounds_traps -- --exact --nocapture
```

Expected: both commands PASS.

- [ ] **Step 5: Do not commit unless the user explicitly asks for one**

If the user requests a checkpoint commit for this task, run:

```bash
git add lib/src/codegen/expr/access.rs lib/tests/integration.rs
git commit -m "bounds-check generic slice indexing"
```

### Task 5: Run The Focused Regression Set And The Full `rock-lib` Suite

**Files:**
- No new files. Only revisit earlier files if one of the verification commands exposes a regression.

- [ ] **Step 1: Run the focused regression commands in sequence**

Run:

```bash
cargo test -p rock-lib parser::items::tests::path::test_type_path::test_type_path_accepts_self_target_projection -- --exact
cargo test -p rock-lib parser::items::tests::path::test_type_path::test_type_path_accepts_self_output_projection -- --exact
cargo test -p rock-lib crate_artifact::tests::test_build_stdlib_artifact_exports_deref_trait -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_deref_trait_is_available_from_prelude -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_get_returns_optional_reference -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_push -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_set_get -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_get_option -- --exact --nocapture
cargo test -p rock-lib --test integration test_sieve_of_eratosthenes -- --exact --nocapture
cargo test -p rock-lib --test integration test_deref_target_type_drives_unary_deref -- --exact --nocapture
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_index_dispatches_through_deref_slice -- --exact --nocapture
cargo test -p rock-lib --test integration test_vec_index_out_of_bounds_traps -- --exact --nocapture
```

Expected: every command PASS.

- [ ] **Step 2: Run the full library test suite once**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 3: Do not commit unless the user explicitly asks for one**

If the user requests a final commit after the full suite is green, run:

```bash
git add lib/src/codegen/expr/access.rs lib/src/crate_artifact/tests.rs lib/src/lower/control_flow/secondary.rs lib/src/lower/expression.rs lib/src/lower/intrinsics.rs lib/src/lower/types_helpers/helpers.rs lib/src/parser/items/tests/path/test_type_path.rs lib/tests/integration.rs stdlib/deref.rk stdlib/lib.rk stdlib/prelude.rk stdlib/vec.rk
git commit -m "add stdlib deref and vec slice autoderef indexing"
```
