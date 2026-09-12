# User-Facing Array Name Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove `Array` as a magic user-facing type/owner name while preserving internal `Type::Array(_, N)` for `[T; N]` and Rust-style borrowed slices through `&[T]` / `&mut [T]`.

**Architecture:** Enforce the source rule at lowering/collection boundaries: bare `ParseType::Slice` is valid only as the referent of a reference type, while string literals and stdlib slice APIs use borrowed slice types. Then remove every production lookup/codegen fallback that treats the string `"Array"` as builtin sequence identity, replacing it with type-shape matching and structural mono owner identity.

**Tech Stack:** Rust 2021, `rock-lib`, parser/AST tests, collect/lower, stdlib `.rk` sources, infer, mono, codegen, integration tests, `cargo fmt --all`, `cargo test -p rock-lib`.

---

## Source Spec

Approved spec: `docs/superpowers/specs/2026-05-09-user-facing-array-name-removal-design.md`.

Important rules from the spec:

- `Array` is available as a normal user-defined name.
- Internal `Type::Array(Box<Type>, usize)` remains the representation for fixed-size `[T; N]`.
- Internal `Type::Slice(Box<Type>)` remains the unsized slice shape, but user-facing source should use `&[T]` or `&mut [T]`.
- Bare `[T]` must not be accepted as a standalone source type, function signature type, or impl target.
- Production compiler code must not use the string `"Array"` as builtin sequence identity.
- Intrinsic names such as `ArrayLen` may remain because they are intrinsic names, not type names.

## File Map

- Modify: `lib/src/parser/items/tests/ast_validation.rs`
  - Replace the old `impl_for_array_type` parser validation with tests showing `Array` is plain named syntax and borrowed slice impl syntax parses.
  - Add a parser-level test documenting that bare `[T]` still parses as `ParseType::Slice` internally, but is rejected later by lowering.
- Modify: `lib/tests/integration.rs`
  - Add end-to-end regressions for user-defined `Array`, rejected bare `[T]`, accepted `&[T]`, and preserved fixed arrays.
  - Update old slice impl tests from `impl Trait for [T]` to `impl Trait for &[T]`.
- Modify: `stdlib/show.rk`, `stdlib/eq.rk`, `stdlib/string_type.rk`
  - Replace public bare slice signatures/impls with borrowed slice spellings.
- Modify: `lib/src/collect/context.rs`, `lib/src/lower/types.rs`
  - Add source-context-aware type lowering that rejects bare `ParseType::Slice` except under `ParseType::Reference`.
  - Lower `Str` and string literals to borrowed slices.
- Modify: `lib/src/collect/headers.rs`, `lib/src/lower/collect/traits.rs`
  - Treat `impl Trait for &[T]` and `impl Trait for &mut [T]` as builtin slice impls.
  - Stop producing `type_name: "Array"` for generic slice impls.
- Modify: `lib/src/lower/types_helpers/helpers.rs`, `lib/src/infer/solve.rs`
  - Remove `"Array"` method/trait lookup fallbacks.
  - Add shape-aware borrowed-slice candidates and array-to-slice-reference receiver adjustment.
- Modify: `lib/src/mono/registry.rs`, `lib/src/mono/mod.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/external.rs`
  - Add structural `InstanceImplOwner` for builtin impl method origins.
  - Remove synthetic `Array`/`BuiltinSlice` fake owner `DefId` use.
  - Update direct-HIR tests to use borrowed-slice spelling or structural owners.
- Modify: `lib/src/codegen/mod.rs`, `lib/src/codegen/types.rs`, `lib/src/codegen/intrinsics.rs`
  - Remove `Type::Struct("Array", ...)` builtin storage handling.
  - Make method lookup and intrinsics key on `Type::Slice`, `Type::Array`, and references to them.

## Scope Guard

- Do not rename internal `Type::Array`.
- Do not remove `Type::Slice`.
- Do not change fixed array syntax `[T; N]`.
- Do not redesign semantic `Type` into ID-based `Ty`.
- Do not implement a new `BuiltinFixedArray` variant unless tests reveal that fixed arrays and slices need distinct mono owner keys in this slice.
- Do not apply the old stash `wip canonical identity aborted pre-plan`; implement from clean tree and use it only as a manual reference if absolutely needed.

---

### Task 0: Baseline And Safety Checks

**Files:**
- Verify only.

- [ ] **Step 1: Confirm tracked tree is clean except committed spec**

Run:

```bash
git status --short
```

Expected: no tracked implementation edits. Pre-existing untracked plan docs may remain.

- [ ] **Step 2: Confirm baseline focused tests compile**

Run:

```bash
cargo test -p rock-lib parser::items::tests::ast_validation::impl_for_array_type -- --exact
cargo test -p rock-lib --test integration test_explicit_generic_slice_impl_uses_self_body_and_dispatches -- --exact
```

Expected before implementation: both PASS. These tests document the old behavior that later tasks will replace.

---

### Task 1: Add Red Regressions For The New Source Rules

**Files:**
- Modify: `lib/src/parser/items/tests/ast_validation.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Replace the old parser validation for `Array T`**

In `lib/src/parser/items/tests/ast_validation.rs`, replace the existing `impl_for_array_type` test with these two tests:

```rust
#[test]
fn impl_for_array_named_type_is_plain_named_impl_target() {
    let input = r#"trait Show
    show: () -> String

struct Array T
    < value: T

impl Show for Array T
    show = -> "Array"
"#;
    validate(input);

    let p = parse(input);
    let i = program_find_impl_for(&p, "Show", "Array");
    assert!(i.is_some(), "Should find impl Show for user type Array T");
    let i = i.unwrap();
    let for_type = i.for_.as_ref().expect("impl should have a target type");
    match for_type {
        ParseType::Type(inner) => {
            assert_eq!(inner.name, "Array");
            assert_eq!(inner.generics.len(), 1);
        }
        other => panic!("expected named Array type, got {:?}", other),
    }
}

#[test]
fn impl_for_borrowed_slice_type_parses_reference_to_slice() {
    let input = r#"trait Show
    show: () -> String

impl Show for &[T]
    show = -> "slice"
"#;
    validate(input);

    let p = parse(input);
    let i = program_find_impl_for(&p, "Show", "&[T]")
        .expect("Should find impl Show for borrowed slice");
    let for_type = i.for_.as_ref().expect("impl should have a target type");
    match for_type {
        ParseType::Reference { is_mut, pointee } => {
            assert!(!is_mut);
            assert!(matches!(pointee.as_ref(), ParseType::Slice(_)));
        }
        other => panic!("expected borrowed slice type, got {:?}", other),
    }
}
```

- [ ] **Step 2: Add integration regressions for user-facing behavior**

Append these tests near the existing slice/fixed-array tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_array_is_available_as_user_defined_type_name() {
    let output = compile_and_run(
        r#"
struct Array
    < value: I64

main = ->
    a = Array
        value: 41
    a.value.println!
    0
"#,
    );

    assert_eq!(output.trim(), "41");
}

#[test]
fn test_bare_slice_impl_target_is_rejected() {
    compile_should_fail(
        r#"
trait SliceLen
    @slice_len = -> 0

impl SliceLen for [T]
    @slice_len = -> ~ArrayLen self

main = -> 0
"#,
        "bare slice type [T] must be written behind a reference",
    );
}

#[test]
fn test_borrowed_slice_impl_target_dispatches() {
    let output = compile_and_run(
        r#"
trait SliceLen
    @slice_len = -> 0

impl SliceLen for &[T]
    @slice_len = -> ~ArrayLen self

main = ->
    s = "abc"
    s.slice_len!.println!
    0
"#,
    );

    assert_eq!(output.trim(), "3");
}

#[test]
fn test_user_defined_array_does_not_receive_slice_methods() {
    compile_should_fail(
        r#"
struct Array
    < value: I64

main = ->
    a = Array
        value: 1
    a.show!.println!
    0
"#,
        "Unknown field 'show' on struct 'Array'",
    );
}
```

- [ ] **Step 3: Run the new tests and confirm red state**

Run:

```bash
cargo test -p rock-lib parser::items::tests::ast_validation::impl_for_array_named_type_is_plain_named_impl_target -- --exact
cargo test -p rock-lib parser::items::tests::ast_validation::impl_for_borrowed_slice_type_parses_reference_to_slice -- --exact
cargo test -p rock-lib --test integration test_array_is_available_as_user_defined_type_name -- --exact
cargo test -p rock-lib --test integration test_bare_slice_impl_target_is_rejected -- --exact
cargo test -p rock-lib --test integration test_borrowed_slice_impl_target_dispatches -- --exact
cargo test -p rock-lib --test integration test_user_defined_array_does_not_receive_slice_methods -- --exact
```

Expected: parser tests and `test_array_is_available_as_user_defined_type_name` may already pass. `test_bare_slice_impl_target_is_rejected` and `test_borrowed_slice_impl_target_dispatches` should FAIL before implementation.

- [ ] **Step 4: Commit red tests**

Run:

```bash
git add lib/src/parser/items/tests/ast_validation.rs lib/tests/integration.rs
git commit -m "test: lock Array name and borrowed slice source rules"
```

---

### Task 2: Enforce Borrowed Slice Source Types And Update Stdlib Surface

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `stdlib/show.rk`
- Modify: `stdlib/eq.rk`
- Modify: `stdlib/string_type.rk`
- Test: `lib/src/lower/types.rs`

- [ ] **Step 1: Make `CollectContext` reject bare slices outside references**

In `lib/src/collect/context.rs`, replace `pub(crate) fn lower_parse_type(&mut self, parse_type: &ast::ParseType) -> Type` with a wrapper plus helper:

```rust
pub(crate) fn lower_parse_type(&mut self, parse_type: &ast::ParseType) -> Type {
    self.lower_parse_type_with_slice_context(parse_type, false)
}

fn lower_parse_type_with_slice_context(
    &mut self,
    parse_type: &ast::ParseType,
    allow_bare_slice: bool,
) -> Type {
    match parse_type {
        ast::ParseType::Unit => Type::Unit,
        ast::ParseType::Type(inner) => self.lower_parse_type_inner(inner),
        ast::ParseType::Associated { base, member } => {
            let base_ty = self.lower_parse_type_inner(base);
            let trait_name = if base.name == "Self" {
                self.current_trait
                    .clone()
                    .unwrap_or_else(|| base.name.clone())
            } else {
                base.name.clone()
            };
            let trait_args = self
                .traits
                .get(&trait_name)
                .map(|trait_def| {
                    trait_def
                        .generic_params
                        .iter()
                        .map(|name| Type::Generic(name.clone()))
                        .collect()
                })
                .unwrap_or_else(|| {
                    self.current_trait_generics
                        .iter()
                        .map(|name| Type::Generic(name.clone()))
                        .collect()
                });

            Type::Projection {
                ty: Box::new(base_ty),
                trait_name,
                assoc_name: member.name.clone(),
                trait_args,
            }
        }
        ast::ParseType::Function(types) => {
            if types.is_empty() {
                return Type::Unit;
            }
            if types.len() == 1 {
                return self.lower_parse_type_with_slice_context(&types[0], false);
            }

            let params: Vec<Type> = types[..types.len() - 1]
                .iter()
                .map(|ty| self.lower_parse_type_with_slice_context(ty, false))
                .collect();
            let ret = self.lower_parse_type_with_slice_context(&types[types.len() - 1], false);
            Type::Function(params, Box::new(ret))
        }
        ast::ParseType::Slice(inner) => {
            if !allow_bare_slice {
                self.push_error("bare slice type [T] must be written behind a reference, such as &[T] or &mut [T]".to_string());
                return Type::Error;
            }
            Type::Slice(Box::new(self.lower_parse_type_with_slice_context(inner, false)))
        }
        ast::ParseType::Array { inner, len } => {
            let inner_ty = self.lower_parse_type_with_slice_context(inner, false);
            Type::Array(Box::new(inner_ty), *len)
        }
        ast::ParseType::Tuple(elems) => Type::Tuple(
            elems
                .iter()
                .map(|ty| self.lower_parse_type_with_slice_context(ty, false))
                .collect(),
        ),
        ast::ParseType::Reference { is_mut, pointee } => Type::Reference {
            mutable: *is_mut,
            inner: Box::new(self.lower_parse_type_with_slice_context(pointee, true)),
        },
        ast::ParseType::Pointer(inner) => {
            Type::Pointer(Box::new(self.lower_parse_type_with_slice_context(inner, false)))
        }
    }
}
```

Keep `lower_parse_type_inner(...)` below it, but change the `"Str"` arm from `Type::Slice(Box::new(Type::U8))` to:

```rust
"Str" => Type::Reference {
    mutable: false,
    inner: Box::new(Type::Slice(Box::new(Type::U8))),
},
```

- [ ] **Step 2: Make `Lowerer` reject bare slices outside references**

In `lib/src/lower/types.rs`, replace `pub(crate) fn lower_parse_type(&mut self, parse_type: &ast::ParseType) -> Type` with the same wrapper/helper shape from Step 1, using `self.push_error(...)` for the same diagnostic string. Keep behavior for associated types, function types, arrays, tuples, references, and pointers equivalent to the old function.

Also change `lower_parse_type_inner(...)` so `"Str"` returns:

```rust
"Str" => Type::Reference {
    mutable: false,
    inner: Box::new(Type::Slice(Box::new(Type::U8))),
},
```

- [ ] **Step 3: Update lower type unit tests**

In `lib/src/lower/types.rs`, replace `test_lower_parse_slice_type_lowers_to_slice_type` with:

```rust
#[test]
fn test_lower_parse_bare_slice_type_reports_error() {
    let mut lowerer = Lowerer::new();

    let ty = lowerer.lower_parse_type(&ParseType::Slice(Box::new(named_type("I64"))));

    assert_eq!(ty, Type::Error);
    assert!(lowerer
        .errors
        .iter()
        .any(|err| err.message.contains("bare slice type [T] must be written behind a reference")));
}

#[test]
fn test_lower_parse_borrowed_slice_type_lowers_to_reference_to_slice_type() {
    let mut lowerer = Lowerer::new();

    let ty = lowerer.lower_parse_type(&ParseType::Reference {
        is_mut: false,
        pointee: Box::new(ParseType::Slice(Box::new(named_type("I64")))),
    });

    assert_eq!(
        ty,
        Type::Reference {
            mutable: false,
            inner: Box::new(Type::Slice(Box::new(Type::I64))),
        }
    );
    assert!(lowerer.errors.is_empty());
}
```

- [ ] **Step 4: Lower string literals to borrowed slices**

In `lib/src/lower/expression.rs`, change the string literal arm from:

```rust
ty: Type::Slice(Box::new(Type::U8)),
```

to:

```rust
ty: Type::Reference {
    mutable: false,
    inner: Box::new(Type::Slice(Box::new(Type::U8))),
},
```

- [ ] **Step 5: Update stdlib slice-facing source**

In `stdlib/show.rk`, change:

```text
impl Show for [T] where T: Show
```

to:

```text
impl Show for &[T] where T: Show
```

and change:

```text
impl Show for [U8]
```

to:

```text
impl Show for &[U8]
```

In `stdlib/eq.rk`, change:

```text
impl Eq for [U8]
```

to:

```text
impl Eq for &[U8]
```

In `stdlib/string_type.rk`, change:

```text
from_str: [U8] -> String
```

to:

```text
from_str: &[U8] -> String
```

- [ ] **Step 6: Run focused source-rule tests**

Run:

```bash
cargo test -p rock-lib lower::types::tests::test_lower_parse_bare_slice_type_reports_error -- --exact
cargo test -p rock-lib lower::types::tests::test_lower_parse_borrowed_slice_type_lowers_to_reference_to_slice_type -- --exact
cargo test -p rock-lib --test integration test_bare_slice_impl_target_is_rejected -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 2**

Run:

```bash
git add lib/src/collect/context.rs lib/src/lower/types.rs lib/src/lower/expression.rs stdlib/show.rk stdlib/eq.rk stdlib/string_type.rk
git commit -m "lower: require borrowed slice source types"
```

---

### Task 3: Remove `Array` From Collect/Lower Method Identity

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add borrowed-slice type-name helpers to lower**

In `lib/src/lower/types_helpers/helpers.rs`, add these helper functions near `get_type_names_for_method_lookup(...)`:

```rust
fn borrowed_slice_family_name(mutable: bool) -> String {
    if mutable {
        "&mut [T]".to_string()
    } else {
        "&[T]".to_string()
    }
}

fn reference_slice_name(mutable: bool, inner: &Type) -> String {
    let slice = Type::Slice(Box::new(inner.clone()));
    Type::Reference {
        mutable,
        inner: Box::new(slice),
    }
    .to_string()
}
```

- [ ] **Step 2: Replace lower method lookup candidates**

In `lib/src/lower/types_helpers/helpers.rs`, replace `get_type_names_for_method_lookup(...)` with:

```rust
pub(crate) fn get_type_names_for_method_lookup(ty: &Type) -> Vec<String> {
    match ty {
        Type::Reference { mutable, inner } => match inner.as_ref() {
            Type::Slice(elem) => {
                let concrete = Self::reference_slice_name(*mutable, elem.as_ref());
                let family = Self::borrowed_slice_family_name(*mutable);
                if concrete == family {
                    vec![family]
                } else {
                    vec![concrete, family]
                }
            }
            Type::Array(elem, _) => {
                let mut names = vec![ty.to_string(), inner.to_string()];
                names.push(Self::reference_slice_name(*mutable, elem.as_ref()));
                names.push(Self::borrowed_slice_family_name(*mutable));
                names
            }
            _ => Self::get_type_name_for_method_lookup(ty)
                .into_iter()
                .collect(),
        },
        Type::Slice(inner) => vec![Type::Slice(inner.clone()).to_string()],
        Type::Array(inner, _) => vec![ty.to_string(), Self::borrowed_slice_family_name(false), {
            Self::reference_slice_name(false, inner.as_ref())
        }],
        _ => Self::get_type_name_for_method_lookup(ty)
            .into_iter()
            .collect(),
    }
}
```

Then change `get_type_name_for_method_lookup(...)` arms for slices/references to:

```rust
Type::Slice(_) => Some(ty.to_string()),
Type::Array(_, _) => Some(ty.to_string()),
Type::Reference { .. } => Some(ty.to_string()),
```

There must be no `"Array".to_string()` in these two methods after this step.

- [ ] **Step 3: Add array-value to borrowed-slice-reference candidate**

In `lib/src/lower/types_helpers/helpers.rs`, add this helper after `coerce_array_ref_to_slice_ref_with_mutability(...)`:

```rust
pub(crate) fn coerce_array_value_to_slice_ref(&mut self, expr: HirExpr) -> Option<HirExpr> {
    let span = expr.span.clone();
    let resolved_ty = self.resolve_projection_type(&self.engine.resolve(&expr.ty));
    let Type::Array(_, _) = resolved_ty else {
        return None;
    };

    if !matches!(
        expr.kind,
        HirExprKind::Var(_) | HirExprKind::FieldAccess(_, _)
    ) {
        return None;
    }

    let borrowed_array = HirExpr {
        ty: Type::Reference {
            mutable: false,
            inner: Box::new(expr.ty.clone()),
        },
        kind: HirExprKind::Ref(false, Box::new(expr)),
        span,
    };

    self.coerce_array_ref_to_slice_ref(borrowed_array)
}
```

In `receiver_adjustment_candidates(...)`, before the existing `coerce_array_value_to_slice_value(expr)` block, insert:

```rust
if let Some(slice_ref) = self.coerce_array_value_to_slice_ref(expr.clone()) {
    push_candidate(self, &mut candidates, &mut seen, slice_ref);
}
```

- [ ] **Step 4: Update receiver arg extraction for references**

In `lib/src/lower/types_helpers/helpers.rs`, update each local `receiver_arg_types` match used for method/trait impl matching so references to slices/arrays report the element type:

```rust
let receiver_arg_types = match base_ty {
    Type::Struct(_, args) | Type::Enum(_, args) => args.clone(),
    Type::Slice(inner) | Type::Array(inner, _) => vec![inner.as_ref().clone()],
    Type::Reference { inner, .. } => match inner.as_ref() {
        Type::Slice(elem) | Type::Array(elem, _) => vec![elem.as_ref().clone()],
        _ => vec![],
    },
    _ => vec![],
};
```

Apply the same shape in `concrete_method_candidate(...)` in `lib/src/lower/control_flow/secondary.rs` if it still only handles `Type::Slice` and `Type::Array` directly.

- [ ] **Step 5: Update collect impl type info**

In `lib/src/collect/context.rs::impl_type_info`, replace the `ast::ParseType::Slice(inner)` arm with an error-producing recovery arm:

```rust
ast::ParseType::Slice(_) => {
    self.push_error("bare slice type [T] must be written behind a reference, such as &[T] or &mut [T]".to_string());
    (for_type.type_name(), vec![], vec![])
}
```

Add an arm before `ast::ParseType::Array { .. }` for borrowed slices:

```rust
ast::ParseType::Reference { is_mut, pointee } => {
    if let ast::ParseType::Slice(inner) = pointee.as_ref() {
        let inner_ty = self.lower_parse_type(inner);
        let type_generics = match &inner_ty {
            Type::Generic(name) => vec![name.clone()],
            _ => vec![],
        };
        let type_name = Type::Reference {
            mutable: *is_mut,
            inner: Box::new(Type::Slice(Box::new(inner_ty.clone()))),
        }
        .to_string();
        (type_name, type_generics, vec![inner_ty])
    } else {
        let receiver_arg_types: Vec<Type> = for_type
            .generics()
            .iter()
            .map(|generic| self.lower_parse_type(generic))
            .collect();
        let type_generics = receiver_arg_types
            .iter()
            .filter_map(|ty| match ty {
                Type::Generic(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        (for_type.type_name(), type_generics, receiver_arg_types)
    }
}
```

Keep the `ast::ParseType::Array { inner, .. }` arm returning `(for_type.type_name(), type_generics, vec![inner_ty])`.

- [ ] **Step 6: Update lower impl type info and owner detection**

Apply the same `Slice` rejection and `Reference { pointee: Slice }` handling from Step 5 to `lib/src/lower/collect/traits.rs::impl_type_info`.

In `lib/src/collect/headers.rs::build_impl`, change the `owner` expression to treat borrowed-slice impls as builtin slice owners:

```rust
owner: match imp.for_.as_ref() {
    Some(ast::ParseType::Reference { pointee, .. })
        if matches!(pointee.as_ref(), ast::ParseType::Slice(_)) => HirImplOwner::BuiltinSlice,
    Some(ast::ParseType::Array { .. }) => HirImplOwner::BuiltinSlice,
    _ => HirImplOwner::Named(type_name.clone()),
},
```

In `lib/src/lower/collect/traits.rs::collect_impl`, make the same `owner` change:

```rust
owner: match imp.for_.as_ref() {
    Some(ast::ParseType::Reference { pointee, .. })
        if matches!(pointee.as_ref(), ast::ParseType::Slice(_)) => HirImplOwner::BuiltinSlice,
    Some(ast::ParseType::Array { .. }) => HirImplOwner::BuiltinSlice,
    _ => HirImplOwner::Named(self.canonical_owner_path(&type_name)),
},
```

- [ ] **Step 7: Update trait solver type names**

In `lib/src/infer/solve.rs`, change `type_shape_matches(...)` so the fallback arm no longer special-cases `Array`:

```rust
_ => false,
```

Change `type_base_name(...)` for slices and references to:

```rust
Type::Slice(_) => ty.to_string(),
Type::Reference { .. } => ty.to_string(),
```

Keep `Type::Array(_, _) => ty.to_string()`.

- [ ] **Step 8: Update old integration tests to borrowed-slice impls**

In `lib/tests/integration.rs`, update these existing test sources:

- `test_explicit_generic_slice_impl_uses_self_body_and_dispatches`: change `impl SliceLen for [T]` to `impl SliceLen for &[T]`.
- `test_explicit_generic_slice_impl_method_as_value_falls_back_from_u8_slice`: change `impl SliceLen for [T]` to `impl SliceLen for &[T]`.
- `test_mut_array_borrow_dispatches_custom_shared_slice_method`: change `impl SliceLen for [T]` to `impl SliceLen for &[T]`.

- [ ] **Step 9: Run focused lower/infer tests**

Run:

```bash
cargo test -p rock-lib --test integration test_borrowed_slice_impl_target_dispatches -- --exact
cargo test -p rock-lib --test integration test_explicit_generic_slice_impl_uses_self_body_and_dispatches -- --exact
cargo test -p rock-lib --test integration test_explicit_generic_slice_impl_method_as_value_falls_back_from_u8_slice -- --exact
cargo test -p rock-lib --test integration test_mut_array_borrow_dispatches_custom_shared_slice_method -- --exact
cargo test -p rock-lib --test integration test_fixed_array_show_uses_slice_impl_via_coercion -- --exact
cargo test -p rock-lib --test integration test_fixed_array_trait_bound_solver_finds_concrete_impl -- --exact
```

Expected: PASS.

- [ ] **Step 10: Commit Task 3**

Run:

```bash
git add lib/src/collect/context.rs lib/src/collect/headers.rs lib/src/lower/collect/traits.rs lib/src/lower/types_helpers/helpers.rs lib/src/lower/control_flow/secondary.rs lib/src/infer/solve.rs lib/tests/integration.rs
git commit -m "lower: remove Array fallback from slice method identity"
```

---

### Task 4: Make Mono Builtin Impl Owners Structural And Remove `Array` Fallbacks

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add structural owner enum to mono registry**

In `lib/src/mono/registry.rs`, replace `InstanceOrigin` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstanceImplOwner {
    Named(DefId),
    BuiltinSlice,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstanceOrigin {
    Function(DefId),
    ImplMethod {
        owner: InstanceImplOwner,
        method: String,
    },
}
```

Update `instance_registry_distinguishes_impl_methods_by_name` so every named impl method origin wraps `owner` with `InstanceImplOwner::Named(owner)`.

- [ ] **Step 2: Re-export structural owner and remove synthetic owner helpers**

In `lib/src/mono/mod.rs`, change the registry re-export to:

```rust
pub use registry::{
    InstanceId, InstanceImplOwner, InstanceKey, InstanceOrigin, InstanceRecord, InstanceRegistry,
    MonomorphizedProgram,
};
```

Delete these helpers:

```rust
pub(super) fn builtin_slice_owner_def_id() -> DefId
fn impl_owner_def_id(&self, imp: &HirImpl, crate_name: Option<&str>) -> DefId
fn synthetic_owner_def_id(name: &str) -> DefId
```

Add this helper:

```rust
fn impl_owner_identity(&self, imp: &HirImpl, crate_name: Option<&str>) -> InstanceImplOwner {
    match &imp.owner {
        HirImplOwner::BuiltinSlice => InstanceImplOwner::BuiltinSlice,
        HirImplOwner::Named(owner_path) => {
            let mut candidates = vec![owner_path.clone(), imp.type_name.clone()];
            if let Some(crate_name) = crate_name {
                candidates.push(format!("{}::{}", crate_name, imp.type_name));
            }
            if let Some(trait_name) = &imp.trait_name {
                candidates.push(format!("{}::{}", imp.type_name, trait_name));
                candidates.push(trait_name.clone());
            }
            InstanceImplOwner::Named(self.resolve_def_id(&candidates))
        }
    }
}
```

Update `method_instance_origin(...)` to use `owner: self.impl_owner_identity(imp, crate_name)`.

Remove unused imports `std::collections::hash_map::DefaultHasher` and `std::hash::{Hash, Hasher}` if they become unused.

- [ ] **Step 3: Remove mono `Array` receiver candidates**

In `lib/src/mono/mod.rs`, replace `lookup_type_names_for_receiver(...)` with the same candidate strategy used in lower:

```rust
fn borrowed_slice_family_name(mutable: bool) -> String {
    if mutable {
        "&mut [T]".to_string()
    } else {
        "&[T]".to_string()
    }
}

fn reference_slice_name(mutable: bool, inner: &Type) -> String {
    Type::Reference {
        mutable,
        inner: Box::new(Type::Slice(Box::new(inner.clone()))),
    }
    .to_string()
}

fn lookup_type_names_for_receiver(ty: &Type) -> Vec<String> {
    match ty {
        Type::Reference { mutable, inner } => match inner.as_ref() {
            Type::Slice(elem) => {
                let concrete = Self::reference_slice_name(*mutable, elem.as_ref());
                let family = Self::borrowed_slice_family_name(*mutable);
                if concrete == family {
                    vec![family]
                } else {
                    vec![concrete, family]
                }
            }
            Type::Array(elem, _) => vec![
                ty.to_string(),
                inner.to_string(),
                Self::reference_slice_name(*mutable, elem.as_ref()),
                Self::borrowed_slice_family_name(*mutable),
            ],
            _ => Self::get_type_name_for_method(ty).into_iter().collect(),
        },
        Type::Slice(_) => vec![ty.to_string()],
        Type::Array(inner, _) => vec![
            ty.to_string(),
            Self::reference_slice_name(false, inner.as_ref()),
            Self::borrowed_slice_family_name(false),
        ],
        _ => Self::get_type_name_for_method(ty).into_iter().collect(),
    }
}
```

Change `get_type_name_for_method(...)` slice/reference arms to:

```rust
Type::Slice(_) => Some(ty.to_string()),
Type::Array(_, _) => Some(ty.to_string()),
Type::Reference { .. } => Some(ty.to_string()),
```

- [ ] **Step 4: Update mono receiver arg extraction for references**

In `lib/src/mono/mod.rs::receiver_type_matches`, update `receiver_arg_types` extraction to include references:

```rust
let receiver_arg_types = match recv_ty {
    Type::Struct(_, generics) | Type::Enum(_, generics) => Some(generics.as_slice()),
    Type::Slice(inner) | Type::Array(inner, _) => Some(std::slice::from_ref(inner.as_ref())),
    Type::Reference { inner, .. } => match inner.as_ref() {
        Type::Slice(elem) | Type::Array(elem, _) => Some(std::slice::from_ref(elem.as_ref())),
        _ => None,
    },
    _ => None,
};
```

- [ ] **Step 5: Update mono direct-HIR tests**

In `lib/src/mono/methods.rs`, update `test_monomorphize_trait_method_call_prefers_concrete_slice_impl_over_generic_builtin_slice_impl`:

- remove `seed_owner(&mut mono, "Array")`;
- replace the generic impl `type_name: "Array".to_string()` with `type_name: "&[T]".to_string()`;
- make the generic impl method receiver type a borrowed slice if the helper currently uses `Type::Slice`: use `Type::Reference { mutable: false, inner: Box::new(Type::Slice(Box::new(Type::Generic("T".to_string())))) }`.

In `lib/src/mono/external.rs::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls`, replace `type_name: "Array".to_string()` with `type_name: "&[T]".to_string()` and make the generic method receiver type a borrowed slice reference.

- [ ] **Step 6: Replace old mono builtin owner test**

In `lib/src/mono/mod.rs`, replace `builtin_slice_owner_identity_does_not_depend_on_type_name` with:

```rust
#[test]
fn builtin_slice_owner_identity_is_structural() {
    let mono = Monomorphizer::new();
    let imp = HirImpl {
        id: DefId::new(CrateId(0), LocalDefId(0)),
        owner: HirImplOwner::BuiltinSlice,
        type_name: "&[T]".to_string(),
        type_generics: vec!["T".to_string()],
        receiver_arg_types: vec![Type::Generic("T".to_string())],
        trait_name: Some("Show".to_string()),
        trait_generics: vec![],
        trait_arg_types: vec![],
        associated_types: vec![],
        bounds: vec![],
        methods: HashMap::new(),
    };

    assert_eq!(
        mono.method_instance_origin(&imp, None, "println"),
        InstanceOrigin::ImplMethod {
            owner: InstanceImplOwner::BuiltinSlice,
            method: "println".to_string(),
        }
    );
}
```

Update the test imports to include `InstanceImplOwner` and `InstanceOrigin` from `super`.

- [ ] **Step 7: Run focused mono tests**

Run:

```bash
cargo test -p rock-lib mono::registry::tests::instance_registry_distinguishes_impl_methods_by_name -- --exact
cargo test -p rock-lib mono::tests::builtin_slice_owner_identity_is_structural -- --exact
cargo test -p rock-lib mono::methods::tests::test_monomorphize_trait_method_call_prefers_concrete_slice_impl_over_generic_builtin_slice_impl -- --exact
cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact
cargo test -p rock-lib mono::external::tests::process_with_crates_records_object_backed_instances_without_re_emitting -- --exact
```

Expected: PASS.

- [ ] **Step 8: Commit Task 4**

Run:

```bash
git add lib/src/mono/registry.rs lib/src/mono/mod.rs lib/src/mono/methods.rs lib/src/mono/external.rs
git commit -m "mono: use structural builtin slice owner identity"
```

---

### Task 5: Remove `Array` Magic From Codegen

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/codegen/intrinsics.rs`

- [ ] **Step 1: Remove codegen method lookup `Array` candidates**

In `lib/src/codegen/mod.rs`, replace `get_type_names_for_method(...)` with the same shape as mono/lower:

```rust
fn borrowed_slice_family_name(mutable: bool) -> String {
    if mutable {
        "&mut [T]".to_string()
    } else {
        "&[T]".to_string()
    }
}

fn reference_slice_name(mutable: bool, inner: &Type) -> String {
    Type::Reference {
        mutable,
        inner: Box::new(Type::Slice(Box::new(inner.clone()))),
    }
    .to_string()
}

fn get_type_names_for_method(recv_ty: &Type) -> Vec<String> {
    match recv_ty {
        Type::Reference { mutable, inner } => match inner.as_ref() {
            Type::Slice(elem) => {
                let concrete = Self::reference_slice_name(*mutable, elem.as_ref());
                let family = Self::borrowed_slice_family_name(*mutable);
                if concrete == family {
                    vec![family]
                } else {
                    vec![concrete, family]
                }
            }
            Type::Array(elem, _) => vec![
                recv_ty.to_string(),
                inner.to_string(),
                Self::reference_slice_name(*mutable, elem.as_ref()),
                Self::borrowed_slice_family_name(*mutable),
            ],
            _ => vec![Self::get_type_name_for_method(recv_ty)],
        },
        Type::Slice(_) => vec![recv_ty.to_string()],
        Type::Array(inner, _) => vec![
            recv_ty.to_string(),
            Self::reference_slice_name(false, inner.as_ref()),
            Self::borrowed_slice_family_name(false),
        ],
        _ => vec![Self::get_type_name_for_method(recv_ty)],
    }
}
```

Update `find_trait_impl(...)` receiver argument extraction to include references to slices/arrays:

```rust
let receiver_arg_types = match recv_ty {
    Type::Struct(_, args) | Type::Enum(_, args) => args.clone(),
    Type::Slice(inner) | Type::Array(inner, _) => vec![inner.as_ref().clone()],
    Type::Reference { inner, .. } => match inner.as_ref() {
        Type::Slice(elem) | Type::Array(elem, _) => vec![elem.as_ref().clone()],
        _ => vec![],
    },
    _ => vec![],
};
```

- [ ] **Step 2: Update codegen type name helper**

In `lib/src/codegen/types.rs::get_type_name_for_method`, replace slice/reference arms with:

```rust
Type::Slice(_) => ty.to_string(),
Type::Array(_, _) => ty.to_string(),
Type::Reference { .. } => ty.to_string(),
```

There must be no `Type::Slice(_) => "Array".to_string()` after this step.

- [ ] **Step 3: Remove `Type::Struct("Array")` from `ArrayLen` intrinsic**

In `lib/src/codegen/intrinsics.rs`, in the `"ArrayLen"` match, delete these branches:

```rust
Type::Struct(name, _) if name == "Array" => { ... }
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Struct(name, _) if name == "Array") => { ... }
```

Keep the `Type::Slice(_)`, `Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_))`, `Type::Array(_, len)`, and `Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Array(_, _))` branches.

- [ ] **Step 4: Remove `Type::Struct("Array")` from `ArrPtr` intrinsic and add borrowed slice support**

In `lib/src/codegen/intrinsics.rs`, in the `"ArrPtr"` match, delete:

```rust
Type::Struct(name, _) if name == "Array" => { ... }
```

Add this branch after `Type::Slice(_)`:

```rust
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_)) => {
    let arr = compiled_args[0].into_struct_value();
    let ptr = self
        .builder
        .build_extract_value(arr, 0, "arr_raw_ptr")
        .map_err(|e| CodegenError::from(format!("Failed ArrPtr: {}", e)))?;
    Ok(Some(ptr))
}
```

Keep `Type::Array(_, _)` support.

- [ ] **Step 5: Run focused codegen/integration tests**

Run:

```bash
cargo test -p rock-lib --test integration test_array_is_available_as_user_defined_type_name -- --exact
cargo test -p rock-lib --test integration test_user_defined_array_does_not_receive_slice_methods -- --exact
cargo test -p rock-lib --test integration test_borrowed_slice_impl_target_dispatches -- --exact
cargo test -p rock-lib --test integration test_fixed_array_show_uses_slice_impl_via_coercion -- --exact
cargo test -p rock-lib --test integration test_mut_array_borrow_directly_dispatches_shared_slice_method -- --exact
cargo test -p rock-lib --test integration test_mut_array_borrow_dispatches_custom_shared_slice_method -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit Task 5**

Run:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/types.rs lib/src/codegen/intrinsics.rs
git commit -m "codegen: remove Array magic from slice handling"
```

---

### Task 6: Production Grep Cleanup And Full Verification

**Files:**
- Modify only if grep finds remaining production magic: files under `lib/src`, `stdlib`.
- Verify: full package.

- [ ] **Step 1: Search for remaining magic `"Array"` string usage in production compiler code**

Run:

```bash
rg '"Array"' lib/src stdlib
```

Allowed remaining matches:

- `lib/src/ast/debug.rs` label for array literals.
- intrinsic names such as `ArrayLen` in strings or comments.
- test source strings defining user type `Array`.

Not allowed:

- `Type::Struct(name, _) if name == "Array"`.
- method lookup candidates containing `"Array"`.
- `type_name: "Array"` in direct-HIR tests.
- resolver/owner seeding for `"Array"`.

- [ ] **Step 2: If production magic remains, remove it and run focused tests**

For each disallowed match, replace it with type-shape matching already introduced in Tasks 3-5. Then rerun:

```bash
cargo test -p rock-lib --test integration test_array_is_available_as_user_defined_type_name -- --exact
cargo test -p rock-lib --test integration test_borrowed_slice_impl_target_dispatches -- --exact
cargo test -p rock-lib mono::tests::builtin_slice_owner_identity_is_structural -- --exact
```

Expected: PASS.

- [ ] **Step 3: Run formatting**

Run:

```bash
cargo fmt --all
```

Expected: exits successfully.

- [ ] **Step 4: Run full package tests**

Run:

```bash
cargo test -p rock-lib
```

Expected: PASS.

If Cargo reports stale incremental artifacts, run:

```bash
cargo clean -p rock-lib
cargo test -p rock-lib
```

Expected after retry: PASS.

- [ ] **Step 5: Commit cleanup if needed**

If Step 2 or formatting changed files not committed by earlier tasks, run:

```bash
git add lib/src stdlib lib/tests/integration.rs
git commit -m "cleanup: remove remaining Array type-name magic"
```

Skip this commit if there are no uncommitted tracked changes.

- [ ] **Step 6: Inspect final status**

Run:

```bash
git status --short
```

Expected: no tracked implementation edits. Unrelated pre-existing untracked plan docs may remain.

---

## Self-Review

- Spec coverage: Tasks cover user-defined `Array`, rejected bare `[T]`, borrowed slice source forms, stdlib migration, lower/collect type rules, method/infer/mono/codegen removal of `"Array"`, structural mono builtin owner identity, and final grep/full tests.
- Placeholder scan: The plan contains no `TBD`, `TODO`, or unspecified tests. Each code-changing step names exact files and includes concrete code or exact replacements.
- Type consistency: `InstanceImplOwner` is introduced in `mono/registry.rs`, re-exported in `mono/mod.rs`, and used by `InstanceOrigin::ImplMethod`. Borrowed slice spelling consistently uses `&[T]` / `&mut [T]`; internal slice shape remains `Type::Slice`.
