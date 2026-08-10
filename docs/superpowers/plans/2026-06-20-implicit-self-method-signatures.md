# Implicit Self Method Signatures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement bead `new_lang2-lue` by making method signatures omit the automatically injected receiver type while the compiler injects the correct `Self` receiver internally.

**Architecture:** Keep `HirFunctionSig.params` as the canonical callable parameter list used by conformance, selection, artifacts, and monomorphization. Change signature lowering so `@foo: A -> B` becomes HIR params `[&Self, A]`, `~@foo: A -> B` becomes `[Self, A]`, and `^@foo: A -> B` becomes `[&mut Self, A]`. Change method header self-param construction to use the same receiver type shape so signatures and actual method definitions agree.

**Tech Stack:** Rust 2021 compiler implementation in `lib/`, Rock stdlib source in `stdlib/`, integration tests in `lib/tests/integration.rs`, artifact CLI tests in `rock/src/tests/artifact.rs`.

---

## Design Choices

Recommended approach: inject receiver types during lowering/collection, not in the parser.

Tradeoffs:
- Parser injection would make the AST lie about source syntax and would complicate diagnostics for explicit receiver mistakes.
- Lowering injection preserves the source AST and keeps downstream HIR consumers unchanged because they already treat method params as including the receiver.
- A temporary compatibility mode that accepts both `@x: Self -> R` and `@x: R` would reduce migration churn, but this repo is in prototype mode and the bead asks for the new convention. Do not add compatibility shims.

Receiver mapping:
- `@method` injects `&Self`.
- `~@method` injects `Self`.
- `^@method` injects `&mut Self`.

Source migration rule:
- Delete the first receiver type from every method signature.
- Examples: `@show: Self -> String` becomes `@show: String`; `@==: Self -> Self -> Bool` becomes `@==: Self -> Bool`; `@eq_ref: &Self -> &Self -> Bool` becomes `@eq_ref: &Self -> Bool`; `^@set: Vec T -> I64 -> T -> Unit` becomes `^@set: I64 -> T -> Unit`; `~@into_inner: Self -> T` becomes `~@into_inner: T`.

## Files

- Modify: `lib/src/lower/function.rs` for legacy lowerer signature lowering and actual self-param type construction.
- Modify: `lib/src/collect/context.rs` for collect-phase self-param type construction.
- Modify: `lib/src/collect/headers.rs` for collect-phase signature lowering and signature-backed method headers.
- Modify: `lib/src/lower/traits/defaults.rs` for stub params built from trait signatures.
- Modify: `lib/src/lower/traits/conformance.rs` only if conformance still assumes old explicit source receiver semantics after signature injection.
- Modify: `lib/src/lower/bodies.rs` for method body concrete self type substitution to preserve `&Self` and `&mut Self` receiver shapes.
- Modify: `lib/src/selection/service.rs` only if current-trait signature stub construction double-injects a receiver after `HirFunctionSig.params` already includes it.
- Modify: `stdlib/*.rk` method signatures that explicitly list receiver types.
- Modify: Rock snippets in `lib/tests/integration.rs`, `rock/src/tests/artifact.rs`, `lib/src/crate_artifact/tests.rs`, `lib/src/lower/program.rs`, and `lib/src/collect/mod.rs`.
- Test: focused unit tests near changed lowering code plus integration tests for stdlib and trait method behavior.

---

### Task 1: Add Receiver Type Helper and Red Unit Tests

**Files:**
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/headers.rs`

- [ ] **Step 1: Add failing lowerer tests for implicit receiver injection**

Add tests in `lib/src/lower/function.rs` under the existing `#[cfg(test)]` module:

```rust
#[test]
fn lower_method_signature_injects_shared_self_receiver_type() {
    let mut lowerer = Lowerer::new();
    let signature_id = def_id(30);
    let sig = FunctionSig {
        name: ident("is_valid"),
        sig: named_type("Bool"),
        where_clauses: vec![],
        self_receiver: Some(SelfReceiverMode::Shared),
        is_unsafe: false,
        exported: false,
    };

    let hir_sig = lowerer.lower_function_sig_with_id(&sig, signature_id);

    let expected_self = Type::Reference {
        mutable: false,
        inner: Box::new(Type::Generic(GenericParamId {
            owner: signature_id,
            index: 0,
        })),
    };
    assert_eq!(hir_sig.params, vec![expected_self]);
    assert_eq!(hir_sig.ret, Type::Bool);
}

#[test]
fn lower_method_signature_injects_mut_and_move_self_receiver_types() {
    let mut lowerer = Lowerer::new();
    let mut_sig = FunctionSig {
        name: ident("set"),
        sig: ParseType::Function(vec![named_type("I64"), named_type("Unit")]),
        where_clauses: vec![],
        self_receiver: Some(SelfReceiverMode::Mut),
        is_unsafe: false,
        exported: false,
    };
    let move_sig = FunctionSig {
        name: ident("consume"),
        sig: named_type("Bool"),
        where_clauses: vec![],
        self_receiver: Some(SelfReceiverMode::Move),
        is_unsafe: false,
        exported: false,
    };

    let mut_hir = lowerer.lower_function_sig_with_id(&mut_sig, def_id(31));
    let move_hir = lowerer.lower_function_sig_with_id(&move_sig, def_id(32));

    assert!(matches!(mut_hir.params[0], Type::Reference { mutable: true, .. }));
    assert_eq!(mut_hir.params[1], Type::I64);
    assert!(matches!(move_hir.params[0], Type::Generic(_)));
    assert_eq!(move_hir.ret, Type::Bool);
}
```

- [ ] **Step 2: Add failing collect header tests for signature-backed methods**

Update `test_build_function_sig_preserves_self_receiver_for_signature_backed_method` in `lib/src/collect/headers.rs` so it expects an injected receiver param before source params:

```rust
assert_eq!(hir_sig.self_receiver, Some(SelfReceiverMode::Mut));
assert!(matches!(hir_sig.params[0], Type::Reference { mutable: true, .. }));
assert_eq!(hir_sig.params[1], Type::I64);
assert_eq!(hir_sig.ret, Type::Bool);
```

- [ ] **Step 3: Run focused tests and verify failure**

Run:

```bash
cargo test -p rock-lib lower_method_signature_injects_shared_self_receiver_type -- --exact
cargo test -p rock-lib lower_method_signature_injects_mut_and_move_self_receiver_types -- --exact
cargo test -p rock-lib test_build_function_sig_preserves_self_receiver_for_signature_backed_method -- --exact
```

Expected: the new tests fail because signature lowering currently only lowers source-written parameter types and does not prepend receiver types.

- [ ] **Step 4: Implement shared receiver type construction**

Add small helpers in both `lib/src/lower/function.rs` and `lib/src/collect/context.rs`. Keep them local for now because the two contexts use different type-var engines.

Lowerer helper shape:

```rust
fn receiver_ty_for_self(&mut self, self_receiver: ast::SelfReceiverMode) -> Type {
    let base = if let Some(index) = self
        .current_generic_params()
        .iter()
        .position(|param| param == "Self")
    {
        self.current_generic_owner()
            .map(|owner| Type::Generic(GenericParamId { owner, index: index as u32 }))
            .unwrap_or_else(|| self.engine.fresh_type_var())
    } else {
        self.engine.fresh_type_var()
    };

    match self_receiver {
        ast::SelfReceiverMode::Shared => Type::Reference {
            mutable: false,
            inner: Box::new(base),
        },
        ast::SelfReceiverMode::Mut => Type::Reference {
            mutable: true,
            inner: Box::new(base),
        },
        ast::SelfReceiverMode::Move => base,
    }
}
```

CollectContext helper has the same match but uses `self.current_generic_params`, `self.current_generic_owner`, and `self.type_vars.fresh_type_var()`.

- [ ] **Step 5: Use the helper in self-param construction**

Change `build_self_param_with_local_id` in `lib/src/lower/function.rs` and `build_self_param` in `lib/src/collect/context.rs` to set `ty` from the helper. Keep `mutable: matches!(self_receiver, ast::SelfReceiverMode::Mut)` so existing mutable-local checks still work.

- [ ] **Step 6: Inject receiver type into method signatures**

In `lower_function_sig_with_id` in `lib/src/lower/function.rs`, after flattening `lowered_type` into `(params, ret)`, prepend the helper result when `sig.self_receiver` is present:

```rust
let (mut params, ret) = flatten_curried_type(&lowered_type);
if let Some(self_receiver) = sig.self_receiver {
    params.insert(0, self.receiver_ty_for_self(self_receiver));
}
```

Make the same change in `build_function_sig_with_id` in `lib/src/collect/headers.rs` using the CollectContext helper.

- [ ] **Step 7: Run focused tests and verify pass**

Run the same commands from Step 3.

Expected: all three pass.

- [ ] **Step 8: Commit Task 1**

```bash
git add lib/src/lower/function.rs lib/src/collect/context.rs lib/src/collect/headers.rs
git commit -m "feat: inject implicit self in method signatures"
```

---

### Task 2: Align Signature-Backed Method Headers and Trait Stubs

**Files:**
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/traits/defaults.rs`
- Modify: `lib/src/selection/service.rs`

- [ ] **Step 1: Add failing tests for new source convention with signature-backed methods**

Add integration tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_method_signature_omits_shared_self_receiver() {
    let output = compile_and_run(
        r#"
struct Counter
    value: I64

impl Counter
    @value: I64
    @value = -> self.value

main = ->
    c = Counter
        value: 7
    (c.value!).println!
    0
"#,
    );

    assert_eq!(output.trim(), "7");
}

#[test]
fn test_method_signature_omits_mut_and_move_self_receiver() {
    let output = compile_and_run(
        r#"
struct Counter
    < value: I64

impl Counter
    ^@set: I64 -> Unit
    ^@set = value ->
        self.value = value
        return

    ~@take: I64
    ~@take = -> self.value

main = ->
    c = Counter
        value: 1
    c.set! 9
    (c.take!).println!
    0
"#,
    );

    assert_eq!(output.trim(), "9");
}
```

- [ ] **Step 2: Run tests and verify failure if Task 1 did not already fix all paths**

Run:

```bash
cargo test -p rock-lib --test integration test_method_signature_omits_shared_self_receiver -- --exact
cargo test -p rock-lib --test integration test_method_signature_omits_mut_and_move_self_receiver -- --exact
```

Expected before this task is complete: failures may occur from body header param indexing, double receiver injection, or `self` type mismatch.

- [ ] **Step 3: Fix signature-backed method header indexing**

In `lower_function_decl_header_with_sig_and_id` and `build_function_header_with_sig`, keep this invariant:

```rust
let param_start = if fd.self_receiver.is_some() { 1 } else { 0 };
```

Because `sig.params[0]` now contains the injected receiver, lambda parameter `i` must read `sig_params[param_start + i]`. Remove special cases that copy `sig_params.first()` into the self param only for non-mut receivers; the self param should already be built with the correct receiver shape and unified against `sig_params[0]` for all receiver modes.

Use this shape:

```rust
if let Some(self_receiver) = fd.self_receiver {
    let local_id = local_ids.fresh();
    let mut self_param = self.build_self_param_with_local_id(self_receiver, local_id);
    if let Some(sig_self_ty) = sig_params.first() {
        let _ = self.engine.unify(&self_param.ty, sig_self_ty);
        self_param.ty = self.engine.resolve(&self_param.ty);
    }
    all_params.push(self_param);
}
```

Apply the equivalent collect-phase logic in `build_function_header_with_sig` without using the inference engine if unavailable; assign `sig_params[0].clone()` to the generated self param after construction.

- [ ] **Step 4: Avoid double injection in trait-signature stubs**

Audit `lib/src/lower/traits/defaults.rs` and `lib/src/selection/service.rs`. If code builds a method stub from a `HirFunctionSig`, it must not push an extra self param and then iterate all `sig.params`. Use one of these patterns consistently:

```rust
// Preferred: HIR signature params are already complete.
for (i, ty) in sig.params.iter().enumerate() {
    params.push(HirParam {
        name: if i == 0 && sig.self_receiver.is_some() { "self".to_string() } else { format!("arg{}", i) },
        local_id: lowerer.fresh_local_id(),
        ty: ty.clone(),
        mutable: i == 0 && matches!(sig.self_receiver, Some(ast::SelfReceiverMode::Mut)),
        is_ref: false,
    });
}
```

- [ ] **Step 5: Fix concrete method body self type substitution**

In `lib/src/lower/bodies.rs`, compute the concrete receiver type from the receiver mode after deriving the concrete owner type:

```rust
let concrete_owner_ty = /* existing struct/enum/slice/array owner type logic */;
let actual_self_ty = match self_receiver {
    ast::SelfReceiverMode::Shared => Type::Reference {
        mutable: false,
        inner: Box::new(concrete_owner_ty),
    },
    ast::SelfReceiverMode::Mut => Type::Reference {
        mutable: true,
        inner: Box::new(concrete_owner_ty),
    },
    ast::SelfReceiverMode::Move => concrete_owner_ty,
};
```

Preserve any existing builtin slice and generic owner logic inside `concrete_owner_ty`; do not change visibility or field-access policy in this task.

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p rock-lib --test integration test_method_signature_omits_shared_self_receiver -- --exact
cargo test -p rock-lib --test integration test_method_signature_omits_mut_and_move_self_receiver -- --exact
cargo test -p rock-lib test_build_function_sig_preserves_self_receiver_for_signature_backed_method -- --exact
```

Expected: all pass.

- [ ] **Step 7: Commit Task 2**

```bash
git add lib/src/lower/function.rs lib/src/collect/headers.rs lib/src/lower/traits/defaults.rs lib/src/selection/service.rs lib/src/lower/bodies.rs lib/tests/integration.rs
git commit -m "fix: align method headers with implicit self signatures"
```

---

### Task 3: Update Trait Conformance and Associated Type Cases

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/tests/integration.rs`
- Modify: `lib/src/lower/collect/traits.rs`

- [ ] **Step 1: Add failing trait conformance tests for implicit receiver signatures**

Add integration tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_trait_method_signature_omits_self_receiver() {
    let output = compile_and_run(
        r#"
trait Value
    @value: I64

struct Boxed
    value: I64

impl Value for Boxed
    @value = -> self.value

main = ->
    b = Boxed
        value: 11
    (b.value!).println!
    0
"#,
    );

    assert_eq!(output.trim(), "11");
}

#[test]
fn test_trait_method_signature_implicit_self_preserves_associated_output() {
    let output = compile_and_run(
        r#"
trait Projector
    type Output
    @project: Self::Output -> Self::Output

struct Id

impl Projector for Id
    type Output = I64
    @project = value -> value

main = ->
    id = Id
    (id.project! 13).println!
    0
"#,
    );

    assert_eq!(output.trim(), "13");
}
```

- [ ] **Step 2: Run tests and verify failure if conformance still offsets incorrectly**

Run:

```bash
cargo test -p rock-lib --test integration test_trait_method_signature_omits_self_receiver -- --exact
cargo test -p rock-lib --test integration test_trait_method_signature_implicit_self_preserves_associated_output -- --exact
```

Expected before fixes: failures may show trait implementation mismatch or incorrect associated output substitution.

- [ ] **Step 3: Keep conformance comparing full HIR parameter lists**

In `lib/src/lower/traits/conformance.rs`, retain full-list comparison between `method.params` and `sig.params`; both should include the injected receiver. If code still assumes comments like `Self -> Self -> Self` came from source, update comments only. Do not skip `sig.params[0]` during conformance.

- [ ] **Step 4: Update collect trait unit tests**

In `lib/src/lower/collect/traits.rs`, update tests that construct method signatures to use the new source convention. For a trait `Mapper T` with `@map: T`, expected HIR params should be `[&Self]` or `[&Self, T]` depending on the source signature used in the test.

- [ ] **Step 5: Run focused conformance tests**

Run:

```bash
cargo test -p rock-lib --test integration test_trait_method_signature_omits_self_receiver -- --exact
cargo test -p rock-lib --test integration test_trait_method_signature_implicit_self_preserves_associated_output -- --exact
cargo test -p rock-lib lower_collects_trait_signature_generics
```

Expected: matching trait-collection tests pass. If the short unit-test name is too broad or does not match, rerun with the fully qualified path printed by Cargo.

- [ ] **Step 6: Commit Task 3**

```bash
git add lib/src/lower/traits/conformance.rs lib/src/lower/collect/traits.rs lib/tests/integration.rs
git commit -m "test: cover implicit self trait signatures"
```

---

### Task 4: Migrate Stdlib Method Signatures

**Files:**
- Modify: `stdlib/bitwise.rk`
- Modify: `stdlib/index.rk`
- Modify: `stdlib/deref.rk`
- Modify: `stdlib/not.rk`
- Modify: `stdlib/neg.rk`
- Modify: `stdlib/clone.rk`
- Modify: `stdlib/num.rk`
- Modify: `stdlib/drop.rk`
- Modify: `stdlib/show.rk`
- Modify: `stdlib/eq.rk`
- Modify: `stdlib/hash.rk`
- Modify: `stdlib/ord.rk`
- Modify any other `stdlib/*.rk` signatures found by the grep command below.

- [ ] **Step 1: Verify current explicit receiver signatures**

Run:

```bash
rg '[\^~]?@[A-Za-z_!+\-*/%<>=&|][^:]*:\s*(&mut Self|&Self|Self)\s*->' stdlib -g '*.rk'
```

Expected before migration: matches in traits such as `Show`, `Eq`, `Hash`, `Drop`, arithmetic traits, and index/deref traits.

- [ ] **Step 2: Apply mechanical stdlib migration**

Use these exact replacements as the model:

```rock
// Before
@show: Self -> String
@==: Self -> Self -> Bool
@eq_ref: &Self -> &Self -> Bool
@index: Self -> Idx -> &Self::Output
@drop: Self -> Unit

// After
@show: String
@==: Self -> Bool
@eq_ref: &Self -> Bool
@index: Idx -> &Self::Output
@drop: Unit
```

For mut and move receivers:

```rock
// Before
^@set: Self -> I64 -> Unit
~@consume: Self -> I64

// After
^@set: I64 -> Unit
~@consume: I64
```

- [ ] **Step 3: Verify stdlib has no explicit receiver signatures**

Run:

```bash
rg '[\^~]?@[A-Za-z_!+\-*/%<>=&|][^:]*:\s*(&mut Self|&Self|Self)\s*->' stdlib -g '*.rk'
```

Expected: remaining matches are manually reviewed. Some remaining matches are valid after migration, for example `@==: Self -> Bool` and `@hash_ref: &Self -> I64` because that `Self` is now the first explicit non-receiver argument. Do not delete valid operands.

- [ ] **Step 4: Run stdlib-focused tests**

Run:

```bash
cargo test -p rock-lib --test integration test_stdlib
cargo test -p rock-lib --test integration test_hash_map
cargo test -p rock-lib --test integration test_unary_neg_dispatches_through_stdlib_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
```

Expected: all pass.

- [ ] **Step 5: Commit Task 4**

```bash
git add stdlib
git commit -m "refactor: omit self receivers in stdlib signatures"
```

---

### Task 5: Migrate Test Fixtures and Artifact Snippets

**Files:**
- Modify: `lib/tests/integration.rs`
- Modify: `rock/src/tests/artifact.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Find explicit receiver signatures in Rust string fixtures**

Run:

```bash
rg '[\^~]?@[A-Za-z_!+\-*/%<>=&|][^:]*:\s*(&mut Self|&Self|Self)\s*->' lib rock -g '*.rs'
```

Expected before migration: matches in integration tests, crate artifact tests, artifact CLI tests, and a few unit-test string literals.

- [ ] **Step 2: Migrate fixture strings using the same rule as stdlib**

Examples:

```rock
@check: Self -> Token -> I64      // becomes @check: Token -> I64
@unwrap: Self -> Box T -> T       // becomes @unwrap: Box T -> T
@value: Self -> Self::Output -> Self::Output
                                  // becomes @value: Self::Output -> Self::Output
@deref: Self -> &Self::Target     // becomes @deref: &Self::Target
```

Do not alter method bodies or call sites unless type changes from `&Self`/`&mut Self` expose a legitimate body typing issue.

- [ ] **Step 3: Add a regression test proving first explicit `Self` operands still work**

Add an integration test that compiles a binary method where the first source-written argument is still `Self` after receiver migration:

```rust
#[test]
fn test_method_signature_allows_explicit_self_operand_after_implicit_receiver() {
    let output = compile_and_run(
        r#"
trait Same
    @same: Self -> Bool

struct Token
    value: I64

impl Same for Token
    @same = other -> self.value == other.value

main = ->
    a = Token
        value: 4
    b = Token
        value: 4
    (a.same! b).println!
    0
"#,
    );

    assert_eq!(output.trim(), "true");
}
```

Do not add a general diagnostic for old-style `@show: Self -> I64`: after this migration, that syntax is indistinguishable from a valid one-argument method whose explicit argument type is `Self`. Migration should be enforced by updating repo sources and tests, not by rejecting all leading `Self` operands.

- [ ] **Step 4: Run fixture-focused tests**

Run:

```bash
cargo test -p rock-lib --test integration test_method_signature_allows_explicit_self_operand_after_implicit_receiver -- --exact
cargo test -p rock-lib --test integration test_trait_method_signature_omits_self_receiver -- --exact
cargo test -p rock-lib --test integration test_trait_default_methods -- --exact
cargo test -p rock-lib crate_artifact
cargo test -p rock --test artifact
```

Expected: all pass. If `crate_artifact` short filter does not match, run `cargo test -p rock-lib crate_artifact` and inspect exact names.

- [ ] **Step 5: Commit Task 5**

```bash
git add lib/tests/integration.rs rock/src/tests/artifact.rs lib/src/crate_artifact/tests.rs lib/src/lower/program.rs lib/src/collect/mod.rs
git commit -m "test: migrate method signature fixtures"
```

---

### Task 6: Remove Temporary HashRef Workaround and Track EqRef Follow-Up

**Files:**
- Modify: `stdlib/hash.rk`
- Modify: `stdlib/hash_map.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add or update test proving HashMap borrowed probing uses normal Hash**

Use existing `test_stdlib_hash_map_get_and_contains_drop_probe_key_once` as the focused test. Confirm it still covers borrowed lookup and key-drop behavior.

- [ ] **Step 2: Remove `HashRef` from stdlib if normal borrowed receiver dispatch supports the same behavior**

Delete the temporary `HashRef` trait and impls from `stdlib/hash.rk`. Update `stdlib/hash_map.rk` bounds and calls from `HashRef` to `Hash` when the receiver is borrowed through the implicit `@` receiver.

Keep `EqRef` for now. `Eq` still takes the RHS by value, while `HashMap` lookup needs borrowed RHS comparison between stored keys and probe keys. Removing `EqRef` requires borrowed non-receiver equality support or an `Eq` redesign; track that as a follow-up bead.

Expected Rock shape:

```rock
impl HashMap K, V where K: Hash, K: EqRef
    @get = key ->
        // use key.hash! through normal borrowed receivers
        // keep stored_key.eq_ref (&key) until Eq supports borrowed RHS
```

- [ ] **Step 3: Run HashMap tests**

Run:

```bash
cargo test -p rock-lib --test integration test_stdlib_hash_map_get_and_contains_drop_probe_key_once -- --exact
cargo test -p rock-lib --test integration test_hash_map_str_keys -- --exact
cargo test -p rock-lib --test integration test_hash_map_handles_collisions_and_growth -- --exact
```

Expected: all pass. If dispatch still cannot select borrowed receivers for `Hash`, keep `HashRef` and file/update a follow-up bead. If `HashRef` removal works but `EqRef` cannot be removed, keep `EqRef` and file/update a follow-up bead for borrowed RHS equality.

- [ ] **Step 4: Commit Task 6**

```bash
git add stdlib/hash.rk stdlib/hash_map.rk lib/tests/integration.rs
git commit -m "refactor: use implicit hash receiver for hash map probes"
```

---

### Task 7: Full Verification and Bead Closure

**Files:**
- Modify: bead `new_lang2-lue` status through `bd` only if all implementation tasks are complete.

- [ ] **Step 1: Run formatting and whitespace checks**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: no output from either command.

- [ ] **Step 2: Run full library test suite**

Run:

```bash
cargo test -p rock-lib
```

Expected: all unit, integration, parser, and doctests pass.

- [ ] **Step 3: Run rock artifact tests if touched**

Run:

```bash
cargo test -p rock --test artifact
```

Expected: all artifact CLI tests pass.

- [ ] **Step 4: Verify no old-style signatures remain**

Run:

```bash
rg '[\^~]?@[A-Za-z_!+\-*/%<>=&|][^:]*:\s*(&mut Self|&Self|Self)\s*->' stdlib lib rock -g '*.rk' -g '*.rs'
```

Expected: remaining matches are valid first explicit operands, such as binary operators or borrowed probe arguments. Manually review each remaining match; there is no fully reliable grep that can distinguish old explicit receiver syntax from a new first argument whose type is `Self`.

- [ ] **Step 5: Close bead**

Run:

```bash
bd close new_lang2-lue --reason "Implemented implicit Self receiver injection for method signatures and migrated stdlib/tests" --json
```

Expected: JSON output shows `status` closed.

- [ ] **Step 6: Commit final verification/bead export if applicable**

```bash
git status --short
git add .beads/issues.jsonl
git commit -m "chore: close implicit self method signature bead"
```

Only commit `.beads/issues.jsonl` if the bd command changed it and repo policy for bead exports expects it in git.

---

## Risks and Review Points

- `@` and `^@` changing from bare `Self` to reference-shaped `Self` can expose field access and method dispatch bugs through references. Fix those at lowering/type-checking boundaries, not by reverting receiver types to bare `Self`.
- Signature-backed methods appear in both the legacy lowerer and the collect/header path. Update both or artifact tests will diverge from direct integration tests.
- Trait default method stubs and current-trait selection may double-inject self after this change. Stub builders should consume `HirFunctionSig.params` as already complete.
- Old-style `@==: Self -> Self -> Bool` is ambiguous after migration because the new valid binary-op signature is `@==: Self -> Bool`. Diagnostics should reject only a source-leading receiver that matches the receiver mode; do not reject legitimate operand `Self` after the implicit receiver is removed.
- Product artifact format may need a version bump only if serialized HIR signature semantics change incompatibly for persisted artifacts. If tests that read/write artifacts fail due schema expectations, bump the product format and shared sysroot contract in the same commit.

## Final Test Plan

Run these before claiming completion:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
cargo test -p rock --test artifact
rg '[\^~]?@[A-Za-z_!+\-*/%<>=&|][^:]*:\s*(&mut Self|&Self|Self)\s*->' stdlib lib rock -g '*.rk' -g '*.rs'
```

Manually classify remaining grep matches as valid explicit operands or missed old receiver signatures.
