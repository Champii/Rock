# Assoc Types Ops Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the `assoc-types-ops` worktree to a green `cargo test -p rock-lib` baseline by fixing the parser panic, correcting the invalid artifact fixture, and making `Index`/`Deref` operator dispatch work through associated-type-aware trait impls.

**Architecture:** Keep `Deref` strict and keep operators on the normal trait-method pipeline. Fix the parser panic locally, correct the invalid `Deref` test fixture, and then repair the shared lowering/conformance/monomorphization/codegen path so trait impls with associated types and trait generic arguments become concrete enough to dispatch and compile.

**Tech Stack:** Rust 2021, cargo test, Rock compiler lowering/HIR/monomorphization/codegen pipeline.

---

### Task 1: Restore `()` Parsing And Stop The Shorthand Panic

**Files:**
- Modify: `lib/src/parser/items/function_decl.rs`
- Modify: `lib/src/parser/items/path.rs`
- Test: `lib/src/parser/items/tests/ast_validation.rs`
- Test: `lib/src/parser/items/tests/path/test_type_path.rs`

- [ ] **Step 1: Re-run the failing parser test**

Run: `cargo test -p rock-lib parser::items::tests::ast_validation::empty_tuple -- --exact`
Expected: FAIL with a panic from `lib/src/parser/items/function_decl.rs:139` caused by `inner_tokens.last().unwrap()`.

- [ ] **Step 2: Guard the empty-token shorthand case**

In `lib/src/parser/items/function_decl.rs`, replace the unconditional unwrap in `suffix_function_shorthand` with an early parse failure when `inner_tokens` is empty.

Use this shape:

```rust
pub fn suffix_function_shorthand(stream: Input) -> IResult<LambdaDecl> {
    let (stream, (span, inner_tokens)) =
        (get_span, consume_tokens_until(TokenType::CloseParen)).process(stream)?;

    let Some(operator) = inner_tokens.last().cloned() else {
        return Err(ParseError::UnexpectedToken(
            TokenType::Operator("".to_string())
                .discriminant()
                .to_string(),
            Token {
                token_type: TokenType::OpenParen,
                span,
            },
        ));
    };

    // existing operator validation and shorthand rewrite continue here
```

Keep the rest of the shorthand rewrite logic unchanged.

- [ ] **Step 3: Restore full type parsing in `ident_or_type`**

In `lib/src/parser/items/path.rs`, undo the branch regression that replaced `parse_type` with `plain_type`.

Change the file back to this shape:

```rust
use super::{ident, parse_type};

pub fn ident_or_type(stream: Input) -> IResult<IdentOrType> {
    ident
        .map(IdentOrType::Ident)
        .or(parse_type.map(IdentOrType::Type))
        .process(stream)
}
```

Remove the `plain_type` helper entirely if nothing else uses it.

This step is required because `()` currently fails after the panic fix: the associated-type path change in this branch narrowed `ident_or_type` too far, which prevents unit syntax from being recognized as a type-path-backed instance/operand.

- [ ] **Step 4: Run the focused parser test**

Run: `cargo test -p rock-lib parser::items::tests::ast_validation::empty_tuple -- --exact`
Expected: PASS.

- [ ] **Step 5: Run parser regression checks**

Run:

```bash
cargo test -p rock-lib test_parse_function_shorthand -- --exact
cargo test -p rock-lib test_type_path_keeps_enum_variant_segments -- --exact
```

Expected:
- the shorthand command should not regress existing shorthand behavior;
- `test_type_path_keeps_enum_variant_segments` should still PASS after restoring `parse_type`.

### Task 2: Keep `Deref` Strict And Fix The Invalid Artifact Fixture

**Files:**
- Modify: `lib/src/crate_artifact/tests.rs`
- Test: `lib/src/crate_artifact/tests.rs:643-699`
- Test: `lib/tests/integration.rs:3470-3487`

- [ ] **Step 1: Re-run the artifact failure**

Run: `cargo test -p rock-lib crate_artifact::tests::test_artifact_roundtrip_preserves_associated_types -- --exact`
Expected: FAIL with `In method 'Box.deref': return type mismatch: Type mismatch: &T vs T`.

- [ ] **Step 2: Fix the test fixture to match its declared signature**

In `lib/src/crate_artifact/tests.rs`, update the embedded Rock source in `test_artifact_roundtrip_preserves_associated_types` so the impl body returns a reference explicitly:

```rock
impl Deref for Box T
    type Target = T
    @deref = -> &@value
```

Do not change the trait signature or relax compiler typing rules.

- [ ] **Step 3: Re-run the focused artifact test**

Run: `cargo test -p rock-lib crate_artifact::tests::test_artifact_roundtrip_preserves_associated_types -- --exact`
Expected: PASS.

- [ ] **Step 4: Re-run the strictness regression test**

Run: `cargo test -p rock-lib --test integration test_trait_impl_body_must_match_associated_type_signature -- --exact`
Expected: PASS, proving `@deref = -> @value` is still rejected when the signature is `-> &Self::Target`.

### Task 3: Make `Index` And `Deref` Dispatch Through Concrete Trait Impl Methods

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/types.rs`
- Test: `lib/tests/integration.rs:3542-3597`
- Test: `lib/src/mono/methods.rs` and `lib/src/mono/process.rs` nearby tests if new monomorphization unit coverage is needed

- [ ] **Step 1: Reproduce the two focused operator failures**

Run:

```bash
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_deref_target_type_drives_unary_deref -- --exact
```

Expected:
- the `Index` test fails with `Unknown method: Boxed_index`;
- the `Deref` test fails with `Cannot dereference non-pointer type: Struct("Box", [Struct("Point", [])])`.

- [ ] **Step 2: Make trait conformance substitute trait generic arguments as well as `Self`**

In `lib/src/lower/traits/conformance.rs`, extend the local `substitute_self` helper so it also substitutes trait generic names from `imp.trait_generics` with the concrete types from the impl header.

Change the helper signature to accept trait substitutions:

```rust
fn substitute_self(
    ty: &Type,
    impl_type: &Type,
    impl_trait_name: Option<&str>,
    impl_associated_types: &[HirAssociatedTypeDef],
    trait_subst: &HashMap<String, Type>,
) -> Type
```

Add an early generic replacement branch:

```rust
Type::Generic(name) if name == "Self" => impl_type.clone(),
Type::Generic(name) => trait_subst
    .get(name)
    .cloned()
    .unwrap_or_else(|| ty.clone()),
```

When checking each impl, build the substitution map from `imp.trait_generics` to the concrete trait argument types taken from the trait impl key. For `impl Index I64 for Boxed`, that map should contain `Idx -> I64`.

Pass that map into every `substitute_self(...)` call used to unify signature params and returns.

This step is required because otherwise `Index` impl methods keep a non-concrete parameter type like `Generic("Idx")`, which makes codegen treat the impl as non-concrete and skip declaring `Boxed_index`.

- [ ] **Step 3: Keep projection resolution aligned with the concrete impl data**

In `lib/src/lower/types_helpers/helpers.rs` and `lib/src/codegen/types.rs`, keep the existing `resolve_projection_type` structure, but make sure the trait-impl lookup still matches after Step 2.

The critical lookup should continue to use:

```rust
imp.type_name == type_name
    && imp.trait_name.as_deref() == Some(trait_name.as_str())
    && imp.trait_generics.len() == resolved_trait_args.len()
```

Do not add fallback coercions or alternate naming. Only adjust this code if Step 2 reveals a concrete mismatch in the resolved trait arguments.

- [ ] **Step 4: Teach unary `*` to dispatch through `Deref` for user types**

In `lib/src/lower/expression.rs`, replace the current hard error branch for `op.value == "*"` with a three-way path:

1. built-in `&T` stays a plain dereference;
2. built-in raw `*T` stays a plain dereference with the existing unsafe check;
3. other types attempt `Deref` trait dispatch before erroring.

Use the same pattern already used for unary `-` and `!`:

```rust
let resolved_ty = self.engine.resolve(&inner_hir.ty);

if let Some(type_name) = Self::get_type_name_for_method_lookup(&resolved_ty) {
    let found_method = self
        .methods
        .get(&(type_name.clone(), "deref".to_string()))
        .cloned()
        .or_else(|| {
            self.impls
                .iter()
                .find(|imp| {
                    imp.type_name == type_name && imp.methods.contains_key("deref")
                })
                .and_then(|imp| imp.methods.get("deref").cloned())
        });

    if let Some(method_func) = found_method {
        let subst = self.infer_method_substitution(&resolved_ty, &method_func, &[]);
        let result_ref_ty = if subst.is_empty() {
            method_func.ret_type.clone()
        } else {
            method_func.ret_type.substitute_generics(&subst)
        };
        let result_ref_ty = self.resolve_projection_type(&result_ref_ty);
        let result_ty = match result_ref_ty {
            Type::Reference { inner, .. } => *inner,
            other => other,
        };

        return HirExpr {
            ty: result_ty,
            kind: HirExprKind::Deref(Box::new(HirExpr {
                ty: result_ref_ty,
                kind: HirExprKind::MethodCall(
                    Box::new(inner_hir),
                    "deref".to_string(),
                    vec![],
                    method_func.self_receiver,
                ),
                span: self.current_span.clone().unwrap_or_default(),
            })),
            span: self.current_span.clone().unwrap_or_default(),
        };
    }
}
```

Only keep the old `Cannot dereference non-pointer type` error when neither the built-in pointer path nor `Deref` trait dispatch applies.

- [ ] **Step 5: Preserve `Index` lowering but make the generated trait method call concrete**

Keep the existing desugaring in `lib/src/lower/control_flow/secondary.rs`:

```rust
let method_call = HirExpr {
    ty: Type::Reference {
        mutable: false,
        inner: Box::new(Type::Projection {
            ty: Box::new(resolved_expr_ty),
            trait_name: "Index".to_string(),
            assoc_name: "Output".to_string(),
            trait_args: vec![index.ty.clone()],
        }),
    },
    kind: HirExprKind::MethodCall(
        Box::new(expr),
        "index".to_string(),
        vec![index],
        Some(crate::ast::SelfReceiverMode::Shared),
    ),
    span: span.clone(),
};
```

Do not revert this to `HirExprKind::Index`. The point of the fix is to make this trait-method path actually work end-to-end.

- [ ] **Step 6: Make monomorphization preserve the specialized return type on trait method calls**

In `lib/src/mono/methods.rs`, when `monomorphize_trait_method_call` creates the specialized function, set the callee expression type to the function type and update the overall expression type from the specialized function’s concrete return type instead of leaving it as the old projected type.

Use the same pattern as `monomorphize_standalone_method_call`:

```rust
let concrete_ret_ty = self
    .new_functions
    .get(&specialized_name)
    .map(|func| func.ret_type.clone())
    .unwrap_or_else(|| expr.ty.clone());

expr.kind = HirExprKind::Call(
    Box::new(HirExpr {
        kind: HirExprKind::Var(specialized_name),
        ty: concrete_ret_ty.clone(),
        span: expr.span.clone(),
    }),
    call_args,
);
expr.ty = concrete_ret_ty;
```

If the callee type needs to be a full function type rather than the return type for consistency with `compile_call`, use the concrete function’s parameter list and return type to construct `Type::Function(...)` there.

- [ ] **Step 7: Keep codegen declaration and lookup on the normal trait method path**

In `lib/src/codegen/mod.rs` and `lib/src/codegen/expr/mod.rs`, do not add operator-specific special cases. After Steps 2 and 6, `impl_is_concrete` should be able to declare the concrete trait impl method, and the existing lookup should succeed via:

```rust
let trait_mangled = format!("{}_{}_{}", type_name, trait_name, method);
```

If `impl_is_concrete` is still rejecting valid trait impls only because of unresolved projections that already have concrete associated type definitions, make the smallest possible adjustment there instead of adding a new declaration path.

- [ ] **Step 8: Run the focused green tests**

Run:

```bash
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_deref_target_type_drives_unary_deref -- --exact
```

Expected: both PASS.

- [ ] **Step 9: Run the strictness regression and any new unit tests**

Run:

```bash
cargo test -p rock-lib --test integration test_trait_impl_body_must_match_associated_type_signature -- --exact
```

If you added monomorphization unit tests, run the exact new test names here as well.

Expected: PASS.

- [ ] **Step 10: Run the full library suite**

Run: `cargo test -p rock-lib`
Expected: PASS.
