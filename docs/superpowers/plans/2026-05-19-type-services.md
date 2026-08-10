# Type Services Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move semantic type facts, projection normalization, display formatting, and backend-facing type-shape queries behind explicit services while keeping current compiler phase boundaries on structural `Type`.

**Architecture:** Add a new `type_services` module with focused `facts`, `display`, `projection`, and `layout` services. Existing `Type` wrappers may remain, but they delegate to services; lowering and codegen use provider adapters for shared projection normalization; LLVM type construction stays in codegen.

**Tech Stack:** Rust 2021, existing `rock-lib` HIR/type modules, `cargo test -p rock-lib`, `cargo fmt --all`, `git diff --check`.

---

## File Structure

- Create: `lib/src/type_services/mod.rs`
  - Registers the type service submodules.
- Create: `lib/src/type_services/facts.rs`
  - Owns pure structural facts over `Type`: numeric categories, concreteness, builtin index output, copyability, and reference containment.
- Create: `lib/src/type_services/display.rs`
  - Owns diagnostic/compatibility formatting for `Type`.
- Create: `lib/src/type_services/projection.rs`
  - Owns shared projection normalization over structural `Type` through a provider trait.
- Create: `lib/src/type_services/layout.rs`
  - Owns projection-normalized backend shape predicates such as slice and fat-pointer shape.
- Modify: `lib/src/lib.rs`
  - Registers `type_services` as a crate module.
- Modify: `lib/src/types/mod.rs`
  - Delegates ad hoc fact methods and `Display for Type` to services while keeping compatibility APIs.
- Modify: `lib/src/infer/engine.rs`, `lib/src/infer/solve.rs`, `lib/src/mir/builder/mod.rs`, `lib/src/mir/builder/expr.rs`, `lib/src/mir/borrowck/liveness.rs`, `lib/src/mono/methods.rs`, `lib/src/lower/control_flow/secondary.rs`, `lib/src/codegen/mod.rs`
  - Migrates low-risk direct fact call sites to `TypeFacts`.
- Modify: `lib/src/lower/types_helpers/helpers.rs`, `lib/src/codegen/types.rs`
  - Delegates projection normalization to `ProjectionNormalizer` provider adapters.
- Modify: `lib/src/codegen/expr/mod.rs`, `lib/src/codegen/stmt.rs`, `lib/src/codegen/types.rs`
  - Delegates backend-facing shape checks to `TypeLayout` while leaving LLVM lowering in codegen.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Marks Roadmap Task 10 complete and keeps Task 11 as the `TypeId` phase-boundary migration.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Updates the Type Context And Semantic Types audit track with type service evidence and remaining gaps.

## Task 1: Add Pure Type Facts Service

**Files:**
- Create: `lib/src/type_services/mod.rs`
- Create: `lib/src/type_services/facts.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/types/mod.rs`

- [ ] **Step 1: Add the module declaration and failing facts tests**

In `lib/src/lib.rs`, add this module declaration near `type_context` and `type_lowering`:

```rust
pub mod type_services;
```

Create `lib/src/type_services/mod.rs`:

```rust
pub mod facts;
```

Create `lib/src/type_services/facts.rs` with tests that describe the desired service API before implementation:

```rust
use crate::types::Type;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_facts_classify_numeric_and_concrete_types() {
        assert!(TypeFacts::is_integer(&Type::I64));
        assert!(TypeFacts::is_unsigned_integer(&Type::U8));
        assert!(TypeFacts::is_signed_integer(&Type::I32));
        assert!(TypeFacts::is_float(&Type::F64));
        assert!(TypeFacts::is_numeric(&Type::F32));
        assert!(!TypeFacts::is_numeric(&Type::Bool));
        assert!(TypeFacts::is_concrete(&Type::Tuple(vec![Type::I64, Type::Bool])));
        assert!(!TypeFacts::is_concrete(&Type::Generic(crate::types::GenericParamId {
            owner: crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(1)),
            index: 0,
        })));
    }

    #[test]
    fn type_facts_match_copy_and_reference_behavior() {
        assert!(TypeFacts::is_copy(&Type::Tuple(vec![Type::I64, Type::Bool])));
        assert!(!TypeFacts::is_copy(&Type::Array(Box::new(Type::I64), 4)));
        assert!(TypeFacts::contains_reference(&Type::Tuple(vec![Type::Reference {
            mutable: false,
            inner: Box::new(Type::I64),
        }])));
        assert!(!TypeFacts::contains_reference(&Type::Tuple(vec![Type::I64, Type::Bool])));
    }

    #[test]
    fn type_facts_report_builtin_index_output() {
        let index = Type::I64;

        assert_eq!(
            TypeFacts::builtin_index_output(&Type::Slice(Box::new(Type::U8)), &index),
            Some(Type::U8)
        );
        assert_eq!(
            TypeFacts::builtin_index_output(&Type::Array(Box::new(Type::Bool), 3), &index),
            Some(Type::Bool)
        );
        assert_eq!(
            TypeFacts::builtin_index_output(&Type::Pointer(Box::new(Type::Slice(Box::new(Type::I64)))), &index),
            Some(Type::I64)
        );
        assert_eq!(TypeFacts::builtin_index_output(&Type::Str, &index), None);
        assert!(TypeFacts::has_builtin_index_impl(
            &Type::Array(Box::new(Type::I64), 2),
            &index
        ));
    }
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test -p rock-lib type_facts_`

Expected: FAIL with unresolved `TypeFacts` in `lib/src/type_services/facts.rs`.

- [ ] **Step 3: Implement `TypeFacts`**

Insert this implementation above the `#[cfg(test)]` module in `lib/src/type_services/facts.rs`:

```rust
pub struct TypeFacts;

impl TypeFacts {
    pub fn is_integer(ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
        )
    }

    pub fn is_signed_integer(ty: &Type) -> bool {
        matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }

    pub fn is_unsigned_integer(ty: &Type) -> bool {
        matches!(ty, Type::U8 | Type::U16 | Type::U32 | Type::U64)
    }

    pub fn is_float(ty: &Type) -> bool {
        matches!(ty, Type::F32 | Type::F64)
    }

    pub fn is_numeric(ty: &Type) -> bool {
        Self::is_integer(ty) || Self::is_float(ty)
    }

    pub fn is_type_var(ty: &Type) -> bool {
        matches!(ty, Type::TypeVar(_))
    }

    pub fn is_concrete(ty: &Type) -> bool {
        match ty {
            Type::TypeVar(_) | Type::Generic(_) => false,
            Type::Slice(inner) | Type::Pointer(inner) => Self::is_concrete(inner),
            Type::Array(inner, _) => Self::is_concrete(inner),
            Type::Reference { inner, .. } => Self::is_concrete(inner),
            Type::Projection { ty, trait_args, .. } => {
                Self::is_concrete(ty) && trait_args.iter().all(Self::is_concrete)
            }
            Type::Tuple(elems) => elems.iter().all(Self::is_concrete),
            Type::Function(args, ret) => {
                args.iter().all(Self::is_concrete) && Self::is_concrete(ret)
            }
            Type::Struct { args, .. } | Type::Enum { args, .. } => {
                args.iter().all(Self::is_concrete)
            }
            _ => true,
        }
    }

    pub fn builtin_index_output(receiver: &Type, index: &Type) -> Option<Type> {
        match (receiver, index) {
            (Type::Slice(inner), Type::I64) | (Type::Array(inner, _), Type::I64) => {
                Some((**inner).clone())
            }
            (Type::Pointer(inner), Type::I64) => match inner.as_ref() {
                Type::Slice(elem) => Some((**elem).clone()),
                _ => Some((**inner).clone()),
            },
            _ => None,
        }
    }

    pub fn has_builtin_index_impl(receiver: &Type, index: &Type) -> bool {
        Self::builtin_index_output(receiver, index).is_some()
    }

    pub fn is_copy(ty: &Type) -> bool {
        match ty {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 => true,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => true,
            Type::F32 | Type::F64 => true,
            Type::Bool | Type::Char | Type::Unit => true,
            Type::Reference { mutable: false, .. } => true,
            Type::Reference { mutable: true, .. } => false,
            Type::Pointer(_) => true,
            Type::Function(_, _) => true,
            Type::Tuple(elems) => elems.iter().all(Self::is_copy),
            Type::Str => true,
            Type::Slice(_) => true,
            Type::Array(_, _) => false,
            Type::Projection { .. } => false,
            Type::Struct { .. } => false,
            Type::Enum { .. } => false,
            Type::TypeVar(_) | Type::Generic(_) | Type::Error | Type::Never => false,
        }
    }

    pub fn contains_reference(ty: &Type) -> bool {
        match ty {
            Type::Reference { .. } => true,
            Type::Array(inner, _) | Type::Slice(inner) => Self::contains_reference(inner),
            Type::Tuple(elems)
            | Type::Struct { args: elems, .. }
            | Type::Enum { args: elems, .. } => elems.iter().any(Self::contains_reference),
            Type::Function(args, ret) => {
                args.iter().any(Self::contains_reference) || Self::contains_reference(ret)
            }
            Type::Projection { ty, trait_args, .. } => {
                Self::contains_reference(ty) || trait_args.iter().any(Self::contains_reference)
            }
            _ => false,
        }
    }
}
```

- [ ] **Step 4: Delegate existing `Type` fact wrappers**

In `lib/src/types/mod.rs`, add this import with the other crate imports:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace the existing fact methods in `impl Type` with these delegating wrappers:

```rust
    pub fn is_integer(&self) -> bool {
        TypeFacts::is_integer(self)
    }

    pub fn is_signed_integer(&self) -> bool {
        TypeFacts::is_signed_integer(self)
    }

    pub fn is_unsigned_integer(&self) -> bool {
        TypeFacts::is_unsigned_integer(self)
    }

    pub fn is_float(&self) -> bool {
        TypeFacts::is_float(self)
    }

    pub fn is_numeric(&self) -> bool {
        TypeFacts::is_numeric(self)
    }

    pub fn is_type_var(&self) -> bool {
        TypeFacts::is_type_var(self)
    }

    pub fn is_concrete(&self) -> bool {
        TypeFacts::is_concrete(self)
    }

    pub fn builtin_index_output(&self, idx: &Type) -> Option<Type> {
        TypeFacts::builtin_index_output(self, idx)
    }

    pub fn has_builtin_index_impl(&self, idx: &Type) -> bool {
        TypeFacts::has_builtin_index_impl(self, idx)
    }

    pub fn is_copy(&self) -> bool {
        TypeFacts::is_copy(self)
    }

    pub fn contains_reference(&self) -> bool {
        TypeFacts::contains_reference(self)
    }
```

- [ ] **Step 5: Run facts and wrapper compatibility tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_facts_
cargo test -p rock-lib test_builtin_index_output_supports_slice_and_fixed_array
cargo test -p rock-lib test_u8_slice_and_array_index_output_is_u8
cargo test -p rock-lib test_str_has_no_builtin_index_output
```

Expected: PASS for each command.

- [ ] **Step 6: Commit the facts service**

```bash
git add lib/src/lib.rs lib/src/type_services/mod.rs lib/src/type_services/facts.rs lib/src/types/mod.rs
git commit -m "add type facts service"
```

## Task 2: Migrate Low-Risk Fact Consumers

**Files:**
- Modify: `lib/src/infer/engine.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/borrowck/liveness.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Add a failing direct-service coverage test**

In `lib/src/type_services/facts.rs`, add this test inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn type_wrappers_match_direct_type_facts() {
        let receiver = Type::Pointer(Box::new(Type::Slice(Box::new(Type::U8))));
        let index = Type::I64;
        let composite = Type::Function(
            vec![Type::Reference {
                mutable: false,
                inner: Box::new(Type::I64),
            }],
            Box::new(Type::Bool),
        );

        assert_eq!(receiver.has_builtin_index_impl(&index), TypeFacts::has_builtin_index_impl(&receiver, &index));
        assert_eq!(receiver.builtin_index_output(&index), TypeFacts::builtin_index_output(&receiver, &index));
        assert_eq!(composite.contains_reference(), TypeFacts::contains_reference(&composite));
        assert_eq!(composite.is_copy(), TypeFacts::is_copy(&composite));
        assert_eq!(Type::I64.is_integer(), TypeFacts::is_integer(&Type::I64));
        assert_eq!(Type::F64.is_float(), TypeFacts::is_float(&Type::F64));
        assert_eq!(Type::Bool.is_concrete(), TypeFacts::is_concrete(&Type::Bool));
    }
```

- [ ] **Step 2: Run the direct-service test**

Run: `cargo test -p rock-lib type_wrappers_match_direct_type_facts`

Expected: PASS. This test locks the compatibility wrapper behavior before migrating call sites.

- [ ] **Step 3: Replace direct numeric fact calls in inference**

In `lib/src/infer/engine.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
(a_ty, b_ty) if a_ty.is_integer() && b_ty.is_integer() => {
```

with:

```rust
(a_ty, b_ty) if TypeFacts::is_integer(a_ty) && TypeFacts::is_integer(b_ty) => {
```

In `lib/src/infer/solve.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
ty if ty.is_integer() => {}
```

with:

```rust
ty if TypeFacts::is_integer(ty) => {}
```

Replace:

```rust
ty if ty.is_float() => {}
```

with:

```rust
ty if TypeFacts::is_float(ty) => {}
```

- [ ] **Step 4: Replace MIR fact calls**

In `lib/src/mir/builder/mod.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
!ty.is_copy()
```

with:

```rust
!TypeFacts::is_copy(ty)
```

Replace:

```rust
if !recv.ty.has_builtin_index_impl(&args[0].ty) {
```

with:

```rust
if !TypeFacts::has_builtin_index_impl(&recv.ty, &args[0].ty) {
```

In `lib/src/mir/builder/expr.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
if expr.ty.contains_reference() {
```

with:

```rust
if TypeFacts::contains_reference(&expr.ty) {
```

In `lib/src/mir/borrowck/liveness.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
ty.contains_reference() || matches!(ty, Type::Function(_, _) | Type::Pointer(_))
```

with:

```rust
TypeFacts::contains_reference(ty) || matches!(ty, Type::Function(_, _) | Type::Pointer(_))
```

- [ ] **Step 5: Replace builtin index/concreteness fact calls in mono, lower, and codegen**

In `lib/src/mono/methods.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
recv_ty.has_builtin_index_impl(&args[1].ty)
```

with:

```rust
TypeFacts::has_builtin_index_impl(recv_ty, &args[1].ty)
```

In `lib/src/lower/control_flow/secondary.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace these expressions:

```rust
.has_builtin_index_impl(&Type::I64)
```

with:

```rust
|ty| TypeFacts::has_builtin_index_impl(&ty, &Type::I64)
```

When applying this replacement in the iterator closure, keep the resolved type in a local:

```rust
let builtin_index_receiver = autoderef_candidates.iter().any(|candidate| {
    let candidate_ty = self.resolve_projection_type(&self.engine.resolve(&candidate.ty));
    TypeFacts::has_builtin_index_impl(&candidate_ty, &Type::I64)
});
```

Replace:

```rust
let builtin_output = candidate_ty.builtin_index_output(&resolved_index_ty);
```

with:

```rust
let builtin_output = TypeFacts::builtin_index_output(&candidate_ty, &resolved_index_ty);
```

Replace:

```rust
_ if resolved_expr_ty.is_concrete() => {
```

with:

```rust
_ if TypeFacts::is_concrete(&resolved_expr_ty) => {
```

In `lib/src/codegen/mod.rs`, add:

```rust
use crate::type_services::facts::TypeFacts;
```

Replace:

```rust
resolved_recv_ty.has_builtin_index_impl(&resolved_index_ty)
```

with:

```rust
TypeFacts::has_builtin_index_impl(&resolved_recv_ty, &resolved_index_ty)
```

- [ ] **Step 6: Run focused behavior tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_wrappers_match_direct_type_facts
cargo test -p rock-lib substitute_uses_typed_type_var_id_keys
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact
cargo test -p rock-lib --test integration test_borrow_reference_copy_keeps_original_alias_live -- --exact
```

Expected: PASS for each command.

- [ ] **Step 7: Commit fact consumer migration**

```bash
git add lib/src/type_services/facts.rs lib/src/infer/engine.rs lib/src/infer/solve.rs lib/src/mir/builder/mod.rs lib/src/mir/builder/expr.rs lib/src/mir/borrowck/liveness.rs lib/src/mono/methods.rs lib/src/lower/control_flow/secondary.rs lib/src/codegen/mod.rs
git commit -m "route type fact consumers through service"
```

## Task 3: Add Display Service

**Files:**
- Create: `lib/src/type_services/display.rs`
- Modify: `lib/src/type_services/mod.rs`
- Modify: `lib/src/types/mod.rs`

- [ ] **Step 1: Add the display module and failing display tests**

In `lib/src/type_services/mod.rs`, add:

```rust
pub mod display;
```

Create `lib/src/type_services/display.rs`:

```rust
use std::fmt;

use crate::ids::Idx;
use crate::types::Type;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId, TypeVarId};
    use crate::types::{AssociatedTypeKey, GenericParamId};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn type_display_service_preserves_existing_output() {
        let generic = GenericParamId {
            owner: def_id(4),
            index: 1,
        };
        let projection = Type::Projection {
            ty: Box::new(Type::Generic(generic)),
            trait_id: def_id(5),
            assoc_type: AssociatedTypeKey {
                owner: def_id(5),
                assoc_type_id: AssocTypeId(2),
            },
            trait_args: vec![Type::I64, Type::Bool],
        };

        let cases = vec![
            (Type::I8, "I8"),
            (Type::U64, "U64"),
            (Type::F32, "F32"),
            (Type::Bool, "Bool"),
            (Type::Str, "Str"),
            (Type::Char, "Char"),
            (Type::Unit, "()"),
            (Type::Never, "!"),
            (Type::Slice(Box::new(Type::I64)), "[I64]"),
            (Type::Array(Box::new(Type::Bool), 4), "[Bool; 4]"),
            (Type::Tuple(vec![Type::I64, Type::Bool]), "(I64, Bool)"),
            (Type::Function(vec![Type::I64, Type::Bool], Box::new(Type::Unit)), "I64 -> Bool -> ()"),
            (Type::Struct { id: def_id(1), args: vec![Type::I64] }, "struct#0::1<I64>"),
            (Type::Enum { id: def_id(2), args: vec![Type::Bool] }, "enum#0::2<Bool>"),
            (Type::Reference { mutable: false, inner: Box::new(Type::Str) }, "&Str"),
            (Type::Reference { mutable: true, inner: Box::new(Type::I64) }, "&mut I64"),
            (Type::Pointer(Box::new(Type::I64)), "*I64"),
            (Type::TypeVar(TypeVarId(7)), "?T7"),
            (Type::Generic(generic), "generic#0::4.1"),
            (projection, "<generic#0::4.1 as trait#0::5<I64, Bool>>::assoc#0::5.2"),
            (Type::Error, "<error>"),
        ];

        for (ty, expected) in cases {
            assert_eq!(display_type(&ty).to_string(), expected);
            assert_eq!(ty.to_string(), expected);
        }
    }
}
```

- [ ] **Step 2: Run the display test to verify it fails**

Run: `cargo test -p rock-lib type_display_service_preserves_existing_output`

Expected: FAIL with unresolved `display_type`.

- [ ] **Step 3: Implement display formatting service**

Insert this implementation above the `#[cfg(test)]` module in `lib/src/type_services/display.rs`:

```rust
pub struct TypeDisplay<'a> {
    ty: &'a Type,
}

pub fn display_type(ty: &Type) -> TypeDisplay<'_> {
    TypeDisplay { ty }
}

pub fn write_type(ty: &Type, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match ty {
        Type::I8 => write!(f, "I8"),
        Type::I16 => write!(f, "I16"),
        Type::I32 => write!(f, "I32"),
        Type::I64 => write!(f, "I64"),
        Type::U8 => write!(f, "U8"),
        Type::U16 => write!(f, "U16"),
        Type::U32 => write!(f, "U32"),
        Type::U64 => write!(f, "U64"),
        Type::F32 => write!(f, "F32"),
        Type::F64 => write!(f, "F64"),
        Type::Bool => write!(f, "Bool"),
        Type::Str => write!(f, "Str"),
        Type::Char => write!(f, "Char"),
        Type::Unit => write!(f, "()"),
        Type::Never => write!(f, "!"),
        Type::Slice(inner) => write!(f, "[{}]", display_type(inner)),
        Type::Array(inner, len) => write!(f, "[{}; {}]", display_type(inner), len),
        Type::Tuple(elems) => {
            write!(f, "(")?;
            for (index, elem) in elems.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", display_type(elem))?;
            }
            write!(f, ")")
        }
        Type::Function(args, ret) => {
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    write!(f, " -> ")?;
                }
                write!(f, "{}", display_type(arg))?;
            }
            if !args.is_empty() {
                write!(f, " -> ")?;
            }
            write!(f, "{}", display_type(ret))
        }
        Type::Struct { id, args } => {
            write!(f, "struct#{}::{}", id.crate_id.0, id.local.0)?;
            write_type_args(args, f)
        }
        Type::Enum { id, args } => {
            write!(f, "enum#{}::{}", id.crate_id.0, id.local.0)?;
            write_type_args(args, f)
        }
        Type::Reference { mutable, inner } => {
            if *mutable {
                write!(f, "&mut {}", display_type(inner))
            } else {
                write!(f, "&{}", display_type(inner))
            }
        }
        Type::Pointer(inner) => write!(f, "*{}", display_type(inner)),
        Type::TypeVar(id) => write!(f, "?T{}", id.raw()),
        Type::Generic(param) => write!(
            f,
            "generic#{}::{}.{}",
            param.owner.crate_id.0, param.owner.local.0, param.index
        ),
        Type::Projection {
            ty,
            trait_id,
            assoc_type,
            trait_args,
        } => {
            write!(
                f,
                "<{} as trait#{}::{}",
                display_type(ty),
                trait_id.crate_id.0,
                trait_id.local.0
            )?;
            write_type_args(trait_args, f)?;
            write!(
                f,
                ">::assoc#{}::{}.{}",
                assoc_type.owner.crate_id.0,
                assoc_type.owner.local.0,
                assoc_type.assoc_type_id.0
            )
        }
        Type::Error => write!(f, "<error>"),
    }
}

fn write_type_args(args: &[Type], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if args.is_empty() {
        return Ok(());
    }

    write!(f, "<")?;
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}", display_type(arg))?;
    }
    write!(f, ">")
}

impl fmt::Display for TypeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_type(self.ty, f)
    }
}
```

- [ ] **Step 4: Delegate `Display for Type` to the service**

In `lib/src/types/mod.rs`, replace the body of `impl fmt::Display for Type` with:

```rust
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::type_services::display::write_type(self, f)
    }
}
```

Keep `use std::fmt;` because the `Display` impl still uses it.

- [ ] **Step 5: Run display compatibility tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_display_service_preserves_existing_output
cargo test -p rock-lib test_display_formats_slice_and_fixed_array_differently
cargo test -p rock-lib test_display_formats_str_with_rock_spelling
```

Expected: PASS for each command.

- [ ] **Step 6: Commit display service extraction**

```bash
git add lib/src/type_services/mod.rs lib/src/type_services/display.rs lib/src/types/mod.rs
git commit -m "move type display formatting into service"
```

## Task 4: Add Shared Projection Normalization Service

**Files:**
- Create: `lib/src/type_services/projection.rs`
- Modify: `lib/src/type_services/mod.rs`
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/codegen/types.rs`

- [ ] **Step 1: Add projection service module and failing tests**

In `lib/src/type_services/mod.rs`, add:

```rust
pub mod projection;
```

Create `lib/src/type_services/projection.rs`:

```rust
use std::collections::HashMap;

use crate::hir::{HirAssociatedTypeDef, HirImpl, HirImplOwner};
use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId};
use crate::types::{AssociatedTypeKey, GenericParamId, Type};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestProjectionProvider {
        impls: Vec<HirImpl>,
        builtin_index_trait: Option<DefId>,
        trait_subst_owner: Option<DefId>,
    }

    impl ProjectionProvider for TestProjectionProvider {
        fn find_projection_impl(
            &self,
            _base_ty: &Type,
            trait_id: DefId,
            _trait_args: &[Type],
        ) -> Option<HirImpl> {
            self.impls
                .iter()
                .find(|imp| imp.trait_id == Some(trait_id))
                .cloned()
        }

        fn projection_substitution(
            &self,
            imp: &HirImpl,
            base_ty: &Type,
            _trait_id: DefId,
            trait_args: &[Type],
        ) -> HashMap<GenericParamId, Type> {
            let mut subst = HashMap::new();
            if let Type::Struct { args, .. } | Type::Enum { args, .. } = base_ty {
                for (index, arg) in args.iter().enumerate().take(imp.type_generics.len()) {
                    subst.insert(
                        GenericParamId {
                            owner: imp.id,
                            index: index as u32,
                        },
                        arg.clone(),
                    );
                }
            }
            if let Some(owner) = self.trait_subst_owner {
                for (index, arg) in trait_args.iter().enumerate() {
                    subst.insert(
                        GenericParamId {
                            owner,
                            index: index as u32,
                        },
                        arg.clone(),
                    );
                }
            }
            subst
        }

        fn is_builtin_index_trait(&self, trait_id: DefId) -> bool {
            self.builtin_index_trait == Some(trait_id)
        }
    }

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    fn assoc(owner: DefId, index: u32) -> AssociatedTypeKey {
        AssociatedTypeKey {
            owner,
            assoc_type_id: AssocTypeId(index),
        }
    }

    fn test_impl(id: DefId, trait_id: DefId, assoc_ty: Type) -> HirImpl {
        HirImpl {
            id,
            owner: HirImplOwner::Named("Box".to_string()),
            type_name: "Box".to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(GenericParamId { owner: id, index: 0 })],
            trait_name: Some("Deref".to_string()),
            trait_id: Some(trait_id),
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: vec![HirAssociatedTypeDef {
                id: AssocTypeId(0),
                name: "Target".to_string(),
                ty: assoc_ty,
            }],
            bounds: Vec::new(),
            methods: HashMap::new(),
        }
    }

    #[test]
    fn projection_normalizer_resolves_impl_associated_type() {
        let impl_id = def_id(30);
        let trait_id = def_id(20);
        let provider = TestProjectionProvider {
            impls: vec![test_impl(
                impl_id,
                trait_id,
                Type::Generic(GenericParamId {
                    owner: impl_id,
                    index: 0,
                }),
            )],
            builtin_index_trait: None,
            trait_subst_owner: None,
        };
        let projection = Type::Projection {
            ty: Box::new(Type::Struct {
                id: def_id(10),
                args: vec![Type::I64],
            }),
            trait_id,
            assoc_type: assoc(trait_id, 0),
            trait_args: Vec::new(),
        };

        assert_eq!(ProjectionNormalizer::normalize(&provider, &projection), Type::I64);
    }

    #[test]
    fn projection_normalizer_uses_builtin_index_output() {
        let index_trait = def_id(40);
        let provider = TestProjectionProvider {
            impls: Vec::new(),
            builtin_index_trait: Some(index_trait),
            trait_subst_owner: None,
        };
        let projection = Type::Projection {
            ty: Box::new(Type::Array(Box::new(Type::U8), 4)),
            trait_id: index_trait,
            assoc_type: assoc(index_trait, 0),
            trait_args: vec![Type::I64],
        };

        assert_eq!(ProjectionNormalizer::normalize(&provider, &projection), Type::U8);
    }

    #[test]
    fn projection_normalizer_preserves_unresolved_projection_identity() {
        let trait_id = def_id(50);
        let provider = TestProjectionProvider::default();
        let projection = Type::Projection {
            ty: Box::new(Type::Generic(GenericParamId {
                owner: def_id(60),
                index: 0,
            })),
            trait_id,
            assoc_type: assoc(trait_id, 1),
            trait_args: vec![Type::Bool],
        };

        assert_eq!(ProjectionNormalizer::normalize(&provider, &projection), projection);
    }
}
```

- [ ] **Step 2: Run projection tests to verify they fail**

Run: `cargo test -p rock-lib projection_normalizer_`

Expected: FAIL with unresolved `ProjectionProvider` and `ProjectionNormalizer`.

- [ ] **Step 3: Implement the projection service**

Insert this implementation above the `#[cfg(test)]` module in `lib/src/type_services/projection.rs`:

```rust
use crate::type_services::facts::TypeFacts;

pub trait ProjectionProvider {
    fn find_projection_impl(
        &self,
        base_ty: &Type,
        trait_id: DefId,
        trait_args: &[Type],
    ) -> Option<HirImpl>;

    fn projection_substitution(
        &self,
        imp: &HirImpl,
        base_ty: &Type,
        trait_id: DefId,
        trait_args: &[Type],
    ) -> HashMap<GenericParamId, Type>;

    fn is_builtin_index_trait(&self, trait_id: DefId) -> bool;
}

pub struct ProjectionNormalizer;

impl ProjectionNormalizer {
    pub fn normalize<P: ProjectionProvider + ?Sized>(provider: &P, ty: &Type) -> Type {
        match ty {
            Type::Projection {
                ty: base_ty,
                trait_id,
                assoc_type,
                trait_args,
            } => {
                let resolved_base = Self::normalize(provider, base_ty);
                let resolved_trait_args = trait_args
                    .iter()
                    .map(|arg| Self::normalize(provider, arg))
                    .collect::<Vec<_>>();

                if assoc_type.owner != *trait_id {
                    return Type::Projection {
                        ty: Box::new(resolved_base),
                        trait_id: *trait_id,
                        assoc_type: *assoc_type,
                        trait_args: resolved_trait_args,
                    };
                }

                if let Some(imp) =
                    provider.find_projection_impl(&resolved_base, *trait_id, &resolved_trait_args)
                {
                    if let Some(assoc) = imp
                        .associated_types
                        .iter()
                        .find(|assoc| assoc.id == assoc_type.assoc_type_id)
                    {
                        let subst = provider.projection_substitution(
                            &imp,
                            &resolved_base,
                            *trait_id,
                            &resolved_trait_args,
                        );
                        return Self::normalize(provider, &assoc.ty.substitute_generics(&subst));
                    }
                }

                if provider.is_builtin_index_trait(*trait_id) && resolved_trait_args.len() == 1 {
                    if let Some(output) =
                        TypeFacts::builtin_index_output(&resolved_base, &resolved_trait_args[0])
                    {
                        return Self::normalize(provider, &output);
                    }
                }

                Type::Projection {
                    ty: Box::new(resolved_base),
                    trait_id: *trait_id,
                    assoc_type: *assoc_type,
                    trait_args: resolved_trait_args,
                }
            }
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(Self::normalize(provider, inner)),
            },
            Type::Pointer(inner) => Type::Pointer(Box::new(Self::normalize(provider, inner))),
            Type::Slice(inner) => Type::Slice(Box::new(Self::normalize(provider, inner))),
            Type::Array(inner, len) => Type::Array(Box::new(Self::normalize(provider, inner)), *len),
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|elem| Self::normalize(provider, elem)).collect())
            }
            Type::Function(args, ret) => Type::Function(
                args.iter().map(|arg| Self::normalize(provider, arg)).collect(),
                Box::new(Self::normalize(provider, ret)),
            ),
            Type::Struct { id, args } => Type::Struct {
                id: *id,
                args: args.iter().map(|arg| Self::normalize(provider, arg)).collect(),
            },
            Type::Enum { id, args } => Type::Enum {
                id: *id,
                args: args.iter().map(|arg| Self::normalize(provider, arg)).collect(),
            },
            _ => ty.clone(),
        }
    }
}
```

- [ ] **Step 4: Delegate `Lowerer::resolve_projection_type` to the shared service**

In `lib/src/lower/types_helpers/helpers.rs`, add:

```rust
use crate::type_services::projection::{ProjectionNormalizer, ProjectionProvider};
```

Replace the body of `pub(crate) fn resolve_projection_type(&self, ty: &Type) -> Type` with:

```rust
    pub(crate) fn resolve_projection_type(&self, ty: &Type) -> Type {
        ProjectionNormalizer::normalize(self, ty)
    }
```

Add this impl after the `impl Lowerer` block in `lib/src/lower/types_helpers/helpers.rs`:

```rust
impl ProjectionProvider for Lowerer {
    fn find_projection_impl(
        &self,
        base_ty: &Type,
        trait_id: crate::ids::DefId,
        trait_args: &[Type],
    ) -> Option<crate::hir::HirImpl> {
        self.find_matching_trait_impl_by_id(base_ty, trait_id, trait_args)
            .cloned()
    }

    fn projection_substitution(
        &self,
        imp: &crate::hir::HirImpl,
        base_ty: &Type,
        trait_id: crate::ids::DefId,
        trait_args: &[Type],
    ) -> HashMap<GenericParamId, Type> {
        let mut subst = HashMap::new();

        match base_ty {
            Type::Struct { args, .. } | Type::Enum { args, .. } => {
                for (index, concrete) in args.iter().enumerate().take(imp.type_generics.len()) {
                    subst.insert(
                        GenericParamId {
                            owner: imp.id,
                            index: index as u32,
                        },
                        concrete.clone(),
                    );
                }
            }
            Type::Slice(inner) | Type::Array(inner, _) => {
                if !imp.type_generics.is_empty() {
                    subst.insert(
                        GenericParamId {
                            owner: imp.id,
                            index: 0,
                        },
                        inner.as_ref().clone(),
                    );
                }
            }
            _ => {}
        }

        if let Some(trait_def) = self.traits.values().find(|trait_def| trait_def.id == trait_id) {
            for (index, concrete) in trait_args
                .iter()
                .enumerate()
                .take(trait_def.generic_params.len())
            {
                subst.insert(
                    GenericParamId {
                        owner: trait_def.id,
                        index: index as u32,
                    },
                    concrete.clone(),
                );
            }
        }

        subst
    }

    fn is_builtin_index_trait(&self, trait_id: crate::ids::DefId) -> bool {
        self.traits
            .get("Index")
            .is_some_and(|trait_def| trait_def.id == trait_id)
    }
}
```

- [ ] **Step 5: Delegate `CodeGen::resolve_projection_type` to the shared service**

In `lib/src/codegen/types.rs`, add:

```rust
use crate::type_services::projection::{ProjectionNormalizer, ProjectionProvider};
```

Replace the body of `pub(crate) fn resolve_projection_type(&self, ty: &Type) -> Type` with:

```rust
    pub(crate) fn resolve_projection_type(&self, ty: &Type) -> Type {
        ProjectionNormalizer::normalize(self, ty)
    }
```

Add this impl at the end of `lib/src/codegen/types.rs`:

```rust
impl<'ctx> ProjectionProvider for CodeGen<'ctx> {
    fn find_projection_impl(
        &self,
        base_ty: &Type,
        trait_id: crate::ids::DefId,
        trait_args: &[Type],
    ) -> Option<crate::hir::HirImpl> {
        self.find_trait_impl(base_ty, trait_id, trait_args).cloned()
    }

    fn projection_substitution(
        &self,
        imp: &crate::hir::HirImpl,
        base_ty: &Type,
        _trait_id: crate::ids::DefId,
        trait_args: &[Type],
    ) -> HashMap<GenericParamId, Type> {
        let mut subst = HashMap::new();

        match base_ty {
            Type::Struct { args, .. } | Type::Enum { args, .. } => {
                for (index, concrete) in args.iter().enumerate() {
                    subst.insert(
                        GenericParamId {
                            owner: imp.id,
                            index: index as u32,
                        },
                        concrete.clone(),
                    );
                }
            }
            Type::Slice(inner) | Type::Array(inner, _) => {
                if !imp.type_generics.is_empty() {
                    subst.insert(
                        GenericParamId {
                            owner: imp.id,
                            index: 0,
                        },
                        inner.as_ref().clone(),
                    );
                }
            }
            _ => {}
        }

        let type_generic_count = imp.type_generics.len();
        for (index, concrete) in trait_args.iter().enumerate() {
            subst.insert(
                GenericParamId {
                    owner: imp.id,
                    index: (type_generic_count + index) as u32,
                },
                concrete.clone(),
            );
        }

        subst
    }

    fn is_builtin_index_trait(&self, trait_id: crate::ids::DefId) -> bool {
        self.builtin_index_trait_ids.contains(&trait_id)
    }
}
```

- [ ] **Step 6: Run projection and associated type tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib projection_normalizer_
cargo test -p rock-lib type_lowerer_lowers_associated_projection_with_trait_identity
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_self_output_projection -- --exact
```

Expected: PASS for each command.

- [ ] **Step 7: Commit projection normalization service**

```bash
git add lib/src/type_services/mod.rs lib/src/type_services/projection.rs lib/src/lower/types_helpers/helpers.rs lib/src/codegen/types.rs
git commit -m "share projection normalization service"
```

## Task 5: Add Layout-Facing Shape Service

**Files:**
- Create: `lib/src/type_services/layout.rs`
- Modify: `lib/src/type_services/mod.rs`
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/stmt.rs`

- [ ] **Step 1: Add layout module and failing tests**

In `lib/src/type_services/mod.rs`, add:

```rust
pub mod layout;
```

Create `lib/src/type_services/layout.rs`:

```rust
use std::collections::HashMap;

use crate::hir::HirImpl;
use crate::ids::DefId;
use crate::type_services::projection::{ProjectionNormalizer, ProjectionProvider};
use crate::types::{GenericParamId, Type};

#[cfg(test)]
mod tests {
    use super::*;

    struct NoProjectionProvider;

    impl ProjectionProvider for NoProjectionProvider {
        fn find_projection_impl(
            &self,
            _base_ty: &Type,
            _trait_id: DefId,
            _trait_args: &[Type],
        ) -> Option<HirImpl> {
            None
        }

        fn projection_substitution(
            &self,
            _imp: &HirImpl,
            _base_ty: &Type,
            _trait_id: DefId,
            _trait_args: &[Type],
        ) -> HashMap<GenericParamId, Type> {
            HashMap::new()
        }

        fn is_builtin_index_trait(&self, _trait_id: DefId) -> bool {
            false
        }
    }

    #[test]
    fn type_layout_identifies_slice_and_fat_pointer_shapes() {
        let provider = NoProjectionProvider;
        let slice = Type::Slice(Box::new(Type::U8));
        let str_ref = Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        };
        let slice_ptr = Type::Pointer(Box::new(Type::Slice(Box::new(Type::I64))));

        assert!(TypeLayout::is_slice_shape(&slice));
        assert!(TypeLayout::is_slice_type(&provider, &slice));
        assert!(TypeLayout::is_fat_pointer_shape(&str_ref));
        assert!(TypeLayout::is_fat_pointer_shape(&slice_ptr));
        assert!(TypeLayout::is_fat_pointer_type(&provider, &str_ref));
        assert!(!TypeLayout::is_fat_pointer_type(&provider, &Type::Pointer(Box::new(Type::I64))));
    }
}
```

- [ ] **Step 2: Run layout tests to verify they fail**

Run: `cargo test -p rock-lib type_layout_`

Expected: FAIL with unresolved `TypeLayout`.

- [ ] **Step 3: Implement layout shape facts**

Insert this implementation above the `#[cfg(test)]` module in `lib/src/type_services/layout.rs`:

```rust
pub struct TypeLayout;

impl TypeLayout {
    pub fn is_slice_shape(ty: &Type) -> bool {
        matches!(ty, Type::Slice(_))
    }

    pub fn is_str_shape(ty: &Type) -> bool {
        matches!(ty, Type::Str)
    }

    pub fn is_fat_pointer_shape(ty: &Type) -> bool {
        match ty {
            Type::Reference { inner, .. } | Type::Pointer(inner) => {
                matches!(inner.as_ref(), Type::Slice(_) | Type::Str)
            }
            _ => false,
        }
    }

    pub fn is_slice_type<P: ProjectionProvider + ?Sized>(provider: &P, ty: &Type) -> bool {
        matches!(ProjectionNormalizer::normalize(provider, ty), Type::Slice(_))
    }

    pub fn is_fat_pointer_type<P: ProjectionProvider + ?Sized>(provider: &P, ty: &Type) -> bool {
        Self::is_fat_pointer_shape(&ProjectionNormalizer::normalize(provider, ty))
    }
}
```

- [ ] **Step 4: Delegate codegen shape helpers to layout service**

In `lib/src/codegen/types.rs`, add:

```rust
use crate::type_services::layout::TypeLayout;
```

Replace `CodeGen::is_slice_type` with:

```rust
    pub(crate) fn is_slice_type(&self, ty: &Type) -> bool {
        TypeLayout::is_slice_type(self, ty)
    }
```

Replace `CodeGen::is_fat_pointer_type` with:

```rust
    pub(crate) fn is_fat_pointer_type(&self, ty: &Type) -> bool {
        TypeLayout::is_fat_pointer_type(self, ty)
    }
```

In `lib/src/codegen/expr/mod.rs`, add:

```rust
use crate::type_services::layout::TypeLayout;
```

Replace shape checks that directly inspect `matches!(self.resolve_projection_type(&expr.ty), Type::Str)` with:

```rust
TypeLayout::is_str_shape(&self.resolve_projection_type(&expr.ty))
```

Keep calls to `self.is_slice_type` and `self.is_fat_pointer_type`; those now delegate to `TypeLayout`.

In `lib/src/codegen/stmt.rs`, keep existing `self.is_slice_type(ty)` and `self.is_fat_pointer_type(ty)` calls because the codegen helper methods now delegate to `TypeLayout`.

- [ ] **Step 5: Run layout and codegen shape tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_layout_
cargo test -p rock-lib --test integration test_fixed_array_show_uses_slice_impl_via_coercion -- --exact
cargo test -p rock-lib --test integration test_str_trait_impl_uses_borrowed_str_not_u8_slice -- --exact
```

Expected: PASS for each command.

- [ ] **Step 6: Commit layout service extraction**

```bash
git add lib/src/type_services/mod.rs lib/src/type_services/layout.rs lib/src/codegen/types.rs lib/src/codegen/expr/mod.rs lib/src/codegen/stmt.rs
git commit -m "move type layout shape facts into service"
```

## Task 6: Update Roadmap And Audit Trackers

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Run service and regression tests before tracker edits**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_facts_
cargo test -p rock-lib type_display_service_preserves_existing_output
cargo test -p rock-lib projection_normalizer_
cargo test -p rock-lib type_layout_
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 2: Update Roadmap Task 10 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, insert this baseline bullet after the existing Task 9 bullet:

```markdown
- Roadmap Task 10 has landed for type service extraction: pure facts, display formatting, projection normalization, and layout-facing shape checks now live behind explicit services while structural `Type` remains the compatibility boundary for Task 11.
```

In the Task 10 section, add this status line immediately after the heading:

```markdown
**Status:** Complete for type fact, display, projection normalization, and layout-shape service extraction in `docs/superpowers/plans/2026-05-19-type-services.md`.
```

In the focused-plan list near the bottom, add this bullet after `Type Context Scaffold`:

```markdown
6. `Type Services`: complete in `docs/superpowers/plans/2026-05-19-type-services.md`; covers Task 10 by moving type facts, display formatting, projection normalization, and layout-facing shape checks behind explicit services while deferring `TypeId` phase-boundary migration to Task 11.
```

Replace:

```markdown
The first five focused implementation plans are complete. Later work can continue with type facts, `TypeId` phase-boundary migration, selection-service, instance, MIR, or codegen boundaries.
```

with:

```markdown
The first six focused implementation plans are complete. Later work can continue with `TypeId` phase-boundary migration, selection-service, instance, MIR, or codegen boundaries.
```

- [ ] **Step 3: Update audit tracker evidence and remaining work**

In `docs/superpowers/plans/master-audit-checklist.md`, replace the `Type Context And Semantic Types` summary row with:

```markdown
| Type Context And Semantic Types | In progress | `lib/src/type_context/mod.rs`, `lib/src/type_services/*`, `lib/src/types/mod.rs`, `lib/src/type_lowering.rs`, `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md` | Interned `Ty` / `TypeId` scaffolding and explicit type services are in place, but phase-boundary `TypeId` migration remains |
```

In the `## 3. Type Context And Semantic Types` evidence list, add this bullet after the `type_context` evidence bullet:

```markdown
- `lib/src/type_services/*` owns explicit services for pure type facts, display formatting, projection normalization, and layout-facing shape checks that previously lived on `Type` or duplicated phase-local helpers.
```

In the `Done:` list, add:

```markdown
- [x] Moved type facts and semantic queries such as builtin index behavior, copy semantics, layout-facing facts, display formatting, and projection normalization out of ad hoc `Type` helpers into dedicated services.
```

In the `Still to do:` list, remove this line:

```markdown
- [ ] Move type facts and semantic queries such as builtin index behavior, copy semantics, layout-facing facts, and display/symbol formatting out of ad hoc `Type` helpers into dedicated services or context tables.
```

Keep this line under `Still to do:`:

```markdown
- [ ] Migrate selected HIR, inference, and artifact-facing phase boundaries from structural `Type` to context-owned `TypeId` after type fact services are stable.
```

- [ ] **Step 4: Run documentation diff checks**

Run: `git diff --check`

Expected: PASS with no whitespace errors.

- [ ] **Step 5: Commit tracker updates**

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for type services"
```

## Task 7: Final Verification

**Files:**
- Verify: whole workspace formatting and `rock-lib` tests

- [ ] **Step 1: Format Rust changes**

Run: `cargo fmt --all`

Expected: command exits successfully.

- [ ] **Step 2: Verify formatting is stable**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 3: Run focused type service tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_facts_
cargo test -p rock-lib type_display_service_preserves_existing_output
cargo test -p rock-lib projection_normalizer_
cargo test -p rock-lib type_layout_
```

Expected: PASS for each command.

- [ ] **Step 4: Run identity, indexing, projection, display, and borrow regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids
cargo test -p rock-lib test_display_formats_slice_and_fixed_array_differently
cargo test -p rock-lib test_builtin_index_output_supports_slice_and_fixed_array
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_fixed_array_show_uses_slice_impl_via_coercion -- --exact
cargo test -p rock-lib --test integration test_borrow_reference_copy_keeps_original_alias_live -- --exact
```

Expected: PASS for each command.

- [ ] **Step 5: Run the full documented library suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 6: Verify the final diff has no whitespace errors**

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 7: Verify worktree state**

Run: `git status --short`

Expected: clean.

## Self-Review Notes

- Spec coverage: Tasks 1-2 cover pure facts and consumers; Task 3 covers display; Task 4 covers projection normalization; Task 5 covers layout-facing shape facts; Task 6 updates roadmap/audit separation from Task 11; Task 7 verifies focused and full behavior.
- Placeholder scan: this plan contains concrete implementation steps and references only service APIs defined earlier in the plan.
- Type consistency: all services consume structural `Type`; `ProjectionProvider` returns cloned `HirImpl` values to avoid lifetime coupling; `TypeId` phase-boundary migration remains outside this plan.
