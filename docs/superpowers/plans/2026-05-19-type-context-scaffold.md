# Type Context Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first interned semantic `Ty` / `TypeContext` scaffold while keeping existing compiler phase boundaries on structural `Type`.

**Architecture:** Introduce a new `lib/src/type_context/` module with interned `Ty` nodes keyed by `TypeId`. `Ty` stores child types by `TypeId`, while `TypeContext::intern_type` and `TypeContext::type_for` bridge the existing structural `Type` representation without migrating HIR, inference, artifacts, mono, MIR, or codegen.

**Tech Stack:** Rust 2021, existing `rock-lib` typed IDs, `HashMap` interning, `cargo test -p rock-lib`, `cargo fmt --all`, `git diff --check`.

---

## File Structure

- Create: `lib/src/type_context/mod.rs`
  - Owns `Ty`, `TypeContext`, `intern_ty`, `ty`, `intern_type`, `type_for`, and focused unit tests for canonical interning and structural compatibility.
- Modify: `lib/src/lib.rs`
  - Registers `type_context` as a public compiler-library module for future compiler phases.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Marks Roadmap Task 9 complete for the scaffold slice and keeps Task 11 as the phase-boundary migration.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Updates the Type Context And Semantic Types audit track with `type_context` evidence and remaining gaps.

## Task 1: Add Primitive Interning Scaffold

**Files:**
- Create: `lib/src/type_context/mod.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add the module declaration and failing primitive test**

In `lib/src/lib.rs`, add this module declaration near the existing compiler modules:

```rust
pub mod type_context;
```

Create `lib/src/type_context/mod.rs` with this failing test-first skeleton:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_context_interns_same_primitive_once() {
        let mut context = TypeContext::new();

        let first = context.intern_ty(Ty::I64);
        let second = context.intern_ty(Ty::I64);
        let different = context.intern_ty(Ty::Bool);

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert_eq!(context.ty(first), &Ty::I64);
    }
}
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `cargo test -p rock-lib type_context_interns_same_primitive_once`

Expected: FAIL with unresolved `TypeContext` and `Ty` names in `lib/src/type_context/mod.rs`.

- [ ] **Step 3: Implement primitive `Ty` interning**

Replace `lib/src/type_context/mod.rs` with:

```rust
use std::collections::HashMap;

use crate::ids::{IdGen, Idx, TypeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Str,
    Char,
    Unit,
    Never,
    Error,
}

#[derive(Debug, Default)]
pub struct TypeContext {
    tys: Vec<Ty>,
    interned: HashMap<Ty, TypeId>,
    ids: IdGen<TypeId>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern_ty(&mut self, ty: Ty) -> TypeId {
        if let Some(id) = self.interned.get(&ty) {
            return *id;
        }

        let id = self.ids.fresh();
        debug_assert_eq!(id.index(), self.tys.len());
        self.tys.push(ty.clone());
        self.interned.insert(ty, id);
        id
    }

    pub fn ty(&self, id: TypeId) -> &Ty {
        &self.tys[id.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_context_interns_same_primitive_once() {
        let mut context = TypeContext::new();

        let first = context.intern_ty(Ty::I64);
        let second = context.intern_ty(Ty::I64);
        let different = context.intern_ty(Ty::Bool);

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert_eq!(context.ty(first), &Ty::I64);
    }
}
```

- [ ] **Step 4: Run the focused test to verify it passes**

Run: `cargo test -p rock-lib type_context_interns_same_primitive_once`

Expected: PASS.

- [ ] **Step 5: Commit the primitive scaffold**

```bash
git add lib/src/lib.rs lib/src/type_context/mod.rs
git commit -m "add primitive type context interning"
```

## Task 2: Add Full `Ty` Identity Shapes

**Files:**
- Modify: `lib/src/type_context/mod.rs`

- [ ] **Step 1: Add failing composite, nominal, generic, projection, and type-var tests**

Update the imports at the top of `lib/src/type_context/mod.rs` to the final identity imports:

```rust
use crate::ids::{DefId, IdGen, Idx, TypeId, TypeVarId};
use crate::types::{AssociatedTypeKey, GenericParamId};
```

Add these helpers and tests inside the existing `#[cfg(test)] mod tests` block:

```rust
use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId, TypeVarId};
use crate::types::{AssociatedTypeKey, GenericParamId};

fn def_id(index: u32) -> DefId {
    DefId::new(CrateId(0), LocalDefId(index))
}

fn generic(owner: DefId, index: u32) -> GenericParamId {
    GenericParamId { owner, index }
}

fn assoc_type(owner: DefId, index: u32) -> AssociatedTypeKey {
    AssociatedTypeKey {
        owner,
        assoc_type_id: AssocTypeId(index),
    }
}

#[test]
fn type_context_interns_equal_composite_tys() {
    let mut context = TypeContext::new();
    let i64_ty = context.intern_ty(Ty::I64);
    let bool_ty = context.intern_ty(Ty::Bool);

    assert_eq!(
        context.intern_ty(Ty::Slice(i64_ty)),
        context.intern_ty(Ty::Slice(i64_ty))
    );
    assert_eq!(
        context.intern_ty(Ty::Array {
            inner: i64_ty,
            len: 4,
        }),
        context.intern_ty(Ty::Array {
            inner: i64_ty,
            len: 4,
        })
    );
    assert_ne!(
        context.intern_ty(Ty::Array {
            inner: i64_ty,
            len: 4,
        }),
        context.intern_ty(Ty::Array {
            inner: i64_ty,
            len: 8,
        })
    );
    assert_eq!(
        context.intern_ty(Ty::Tuple(vec![i64_ty, bool_ty])),
        context.intern_ty(Ty::Tuple(vec![i64_ty, bool_ty]))
    );
    assert_eq!(
        context.intern_ty(Ty::Function {
            params: vec![i64_ty],
            ret: bool_ty,
        }),
        context.intern_ty(Ty::Function {
            params: vec![i64_ty],
            ret: bool_ty,
        })
    );
    assert_eq!(
        context.intern_ty(Ty::Reference {
            mutable: false,
            inner: i64_ty,
        }),
        context.intern_ty(Ty::Reference {
            mutable: false,
            inner: i64_ty,
        })
    );
    assert_ne!(
        context.intern_ty(Ty::Reference {
            mutable: false,
            inner: i64_ty,
        }),
        context.intern_ty(Ty::Reference {
            mutable: true,
            inner: i64_ty,
        })
    );
    assert_eq!(
        context.intern_ty(Ty::Pointer(i64_ty)),
        context.intern_ty(Ty::Pointer(i64_ty))
    );
}

#[test]
fn type_context_uses_identities_for_nominal_generic_projection_and_type_vars() {
    let mut context = TypeContext::new();
    let owner = def_id(1);
    let other_owner = def_id(2);

    let first_generic = context.intern_ty(Ty::Generic(generic(owner, 0)));
    let same_generic = context.intern_ty(Ty::Generic(generic(owner, 0)));
    let different_owner_generic = context.intern_ty(Ty::Generic(generic(other_owner, 0)));
    let different_index_generic = context.intern_ty(Ty::Generic(generic(owner, 1)));

    assert_eq!(first_generic, same_generic);
    assert_ne!(first_generic, different_owner_generic);
    assert_ne!(first_generic, different_index_generic);

    let type_var = context.intern_ty(Ty::TypeVar(TypeVarId(7)));
    let same_type_var = context.intern_ty(Ty::TypeVar(TypeVarId(7)));
    let different_type_var = context.intern_ty(Ty::TypeVar(TypeVarId(8)));

    assert_eq!(type_var, same_type_var);
    assert_ne!(type_var, different_type_var);

    assert_ne!(
        context.intern_ty(Ty::Struct {
            id: def_id(10),
            args: vec![first_generic],
        }),
        context.intern_ty(Ty::Struct {
            id: def_id(11),
            args: vec![first_generic],
        })
    );
    assert_ne!(
        context.intern_ty(Ty::Enum {
            id: def_id(12),
            args: vec![first_generic],
        }),
        context.intern_ty(Ty::Enum {
            id: def_id(13),
            args: vec![first_generic],
        })
    );

    let trait_id = def_id(20);
    let projection = context.intern_ty(Ty::Projection {
        ty: first_generic,
        trait_id,
        assoc_type: assoc_type(trait_id, 0),
        trait_args: vec![first_generic],
    });

    assert_eq!(
        projection,
        context.intern_ty(Ty::Projection {
            ty: first_generic,
            trait_id,
            assoc_type: assoc_type(trait_id, 0),
            trait_args: vec![first_generic],
        })
    );
    assert_ne!(
        projection,
        context.intern_ty(Ty::Projection {
            ty: different_owner_generic,
            trait_id,
            assoc_type: assoc_type(trait_id, 0),
            trait_args: vec![first_generic],
        })
    );
    assert_ne!(
        projection,
        context.intern_ty(Ty::Projection {
            ty: first_generic,
            trait_id: def_id(21),
            assoc_type: assoc_type(def_id(21), 0),
            trait_args: vec![first_generic],
        })
    );
    assert_ne!(
        projection,
        context.intern_ty(Ty::Projection {
            ty: first_generic,
            trait_id,
            assoc_type: assoc_type(trait_id, 1),
            trait_args: vec![first_generic],
        })
    );
    assert_ne!(
        projection,
        context.intern_ty(Ty::Projection {
            ty: first_generic,
            trait_id,
            assoc_type: assoc_type(trait_id, 0),
            trait_args: vec![different_index_generic],
        })
    );
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test -p rock-lib type_context_uses_identities_for_nominal_generic_projection_and_type_vars`

Expected: FAIL with missing `Ty` variants such as `Generic`, `TypeVar`, `Struct`, `Enum`, and `Projection`.

- [ ] **Step 3: Extend `Ty` to cover the semantic type shapes**

Replace the `Ty` enum in `lib/src/type_context/mod.rs` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Str,
    Char,
    Unit,
    Never,
    Slice(TypeId),
    Array {
        inner: TypeId,
        len: usize,
    },
    Tuple(Vec<TypeId>),
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Struct {
        id: DefId,
        args: Vec<TypeId>,
    },
    Enum {
        id: DefId,
        args: Vec<TypeId>,
    },
    Reference {
        mutable: bool,
        inner: TypeId,
    },
    Pointer(TypeId),
    TypeVar(TypeVarId),
    Generic(GenericParamId),
    Projection {
        ty: TypeId,
        trait_id: DefId,
        assoc_type: AssociatedTypeKey,
        trait_args: Vec<TypeId>,
    },
    Error,
}
```

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cargo test -p rock-lib type_context_`

Expected: PASS for all `type_context_*` tests currently present.

- [ ] **Step 5: Commit the semantic identity shapes**

```bash
git add lib/src/type_context/mod.rs
git commit -m "complete type context identity shapes"
```

## Task 3: Add Structural `Type` Compatibility Conversions

**Files:**
- Modify: `lib/src/type_context/mod.rs`

- [ ] **Step 1: Add failing structural roundtrip tests**

Add `Type` to the top-level imports in `lib/src/type_context/mod.rs`:

```rust
use crate::types::{AssociatedTypeKey, GenericParamId, Type};
```

Add these tests inside the existing `#[cfg(test)] mod tests` block:

```rust
use crate::types::Type;

#[test]
fn type_context_roundtrips_structural_types() {
    let mut context = TypeContext::new();
    let owner = def_id(30);
    let generic_param = generic(owner, 0);
    let projection_trait = def_id(31);

    let cases = vec![
        Type::I8,
        Type::I16,
        Type::I32,
        Type::I64,
        Type::U8,
        Type::U16,
        Type::U32,
        Type::U64,
        Type::F32,
        Type::F64,
        Type::Bool,
        Type::Str,
        Type::Char,
        Type::Unit,
        Type::Never,
        Type::Error,
        Type::Slice(Box::new(Type::I64)),
        Type::Array(Box::new(Type::Bool), 4),
        Type::Tuple(vec![Type::I64, Type::Bool]),
        Type::Function(vec![Type::I64, Type::Bool], Box::new(Type::Unit)),
        Type::Struct {
            id: def_id(32),
            args: vec![Type::Generic(generic_param)],
        },
        Type::Enum {
            id: def_id(33),
            args: vec![Type::I64],
        },
        Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        },
        Type::Reference {
            mutable: true,
            inner: Box::new(Type::I64),
        },
        Type::Pointer(Box::new(Type::I64)),
        Type::TypeVar(TypeVarId(7)),
        Type::Generic(generic_param),
        Type::Projection {
            ty: Box::new(Type::Struct {
                id: def_id(34),
                args: vec![Type::Generic(generic_param)],
            }),
            trait_id: projection_trait,
            assoc_type: assoc_type(projection_trait, 0),
            trait_args: vec![Type::Reference {
                mutable: false,
                inner: Box::new(Type::Generic(generic_param)),
            }],
        },
    ];

    for ty in cases {
        let id = context.intern_type(&ty);
        let roundtrip = context.type_for(id);

        assert_eq!(roundtrip, ty);
        assert_eq!(context.intern_type(&roundtrip), id);
    }
}

#[test]
fn type_context_intern_type_reuses_equal_structural_types() {
    let mut context = TypeContext::new();
    let first = Type::Function(vec![Type::I64], Box::new(Type::Bool));
    let second = Type::Function(vec![Type::I64], Box::new(Type::Bool));
    let different = Type::Function(vec![Type::Bool], Box::new(Type::Bool));

    let first_id = context.intern_type(&first);
    let second_id = context.intern_type(&second);
    let different_id = context.intern_type(&different);

    assert_eq!(first_id, second_id);
    assert_ne!(first_id, different_id);
}

#[test]
fn type_context_keeps_type_var_ids_distinct_from_type_ids() {
    let mut context = TypeContext::new();

    let first = context.intern_type(&Type::TypeVar(TypeVarId(0)));
    let same = context.intern_type(&Type::TypeVar(TypeVarId(0)));
    let different = context.intern_type(&Type::TypeVar(TypeVarId(1)));

    assert_eq!(first, same);
    assert_ne!(first, different);
    assert_eq!(context.type_for(first), Type::TypeVar(TypeVarId(0)));
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test -p rock-lib type_context_roundtrips_structural_types`

Expected: FAIL with missing `TypeContext::intern_type` and `TypeContext::type_for` methods.

- [ ] **Step 3: Implement `Type -> TypeId -> Type` conversions**

Add these methods inside the existing `impl TypeContext` block, after `ty`:

```rust
    pub fn intern_type(&mut self, ty: &Type) -> TypeId {
        match ty {
            Type::I8 => self.intern_ty(Ty::I8),
            Type::I16 => self.intern_ty(Ty::I16),
            Type::I32 => self.intern_ty(Ty::I32),
            Type::I64 => self.intern_ty(Ty::I64),
            Type::U8 => self.intern_ty(Ty::U8),
            Type::U16 => self.intern_ty(Ty::U16),
            Type::U32 => self.intern_ty(Ty::U32),
            Type::U64 => self.intern_ty(Ty::U64),
            Type::F32 => self.intern_ty(Ty::F32),
            Type::F64 => self.intern_ty(Ty::F64),
            Type::Bool => self.intern_ty(Ty::Bool),
            Type::Str => self.intern_ty(Ty::Str),
            Type::Char => self.intern_ty(Ty::Char),
            Type::Unit => self.intern_ty(Ty::Unit),
            Type::Never => self.intern_ty(Ty::Never),
            Type::Slice(inner) => {
                let inner = self.intern_type(inner);
                self.intern_ty(Ty::Slice(inner))
            }
            Type::Array(inner, len) => {
                let inner = self.intern_type(inner);
                self.intern_ty(Ty::Array { inner, len: *len })
            }
            Type::Tuple(elems) => {
                let elems = elems.iter().map(|elem| self.intern_type(elem)).collect();
                self.intern_ty(Ty::Tuple(elems))
            }
            Type::Function(args, ret) => {
                let params = args.iter().map(|arg| self.intern_type(arg)).collect();
                let ret = self.intern_type(ret);
                self.intern_ty(Ty::Function { params, ret })
            }
            Type::Struct { id, args } => {
                let args = args.iter().map(|arg| self.intern_type(arg)).collect();
                self.intern_ty(Ty::Struct { id: *id, args })
            }
            Type::Enum { id, args } => {
                let args = args.iter().map(|arg| self.intern_type(arg)).collect();
                self.intern_ty(Ty::Enum { id: *id, args })
            }
            Type::Reference { mutable, inner } => {
                let inner = self.intern_type(inner);
                self.intern_ty(Ty::Reference {
                    mutable: *mutable,
                    inner,
                })
            }
            Type::Pointer(inner) => {
                let inner = self.intern_type(inner);
                self.intern_ty(Ty::Pointer(inner))
            }
            Type::TypeVar(id) => self.intern_ty(Ty::TypeVar(*id)),
            Type::Generic(param) => self.intern_ty(Ty::Generic(*param)),
            Type::Projection {
                ty,
                trait_id,
                assoc_type,
                trait_args,
            } => {
                let ty = self.intern_type(ty);
                let trait_args = trait_args.iter().map(|arg| self.intern_type(arg)).collect();
                self.intern_ty(Ty::Projection {
                    ty,
                    trait_id: *trait_id,
                    assoc_type: *assoc_type,
                    trait_args,
                })
            }
            Type::Error => self.intern_ty(Ty::Error),
        }
    }

    pub fn type_for(&self, id: TypeId) -> Type {
        match self.ty(id) {
            Ty::I8 => Type::I8,
            Ty::I16 => Type::I16,
            Ty::I32 => Type::I32,
            Ty::I64 => Type::I64,
            Ty::U8 => Type::U8,
            Ty::U16 => Type::U16,
            Ty::U32 => Type::U32,
            Ty::U64 => Type::U64,
            Ty::F32 => Type::F32,
            Ty::F64 => Type::F64,
            Ty::Bool => Type::Bool,
            Ty::Str => Type::Str,
            Ty::Char => Type::Char,
            Ty::Unit => Type::Unit,
            Ty::Never => Type::Never,
            Ty::Slice(inner) => Type::Slice(Box::new(self.type_for(*inner))),
            Ty::Array { inner, len } => Type::Array(Box::new(self.type_for(*inner)), *len),
            Ty::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|elem| self.type_for(*elem)).collect())
            }
            Ty::Function { params, ret } => Type::Function(
                params.iter().map(|param| self.type_for(*param)).collect(),
                Box::new(self.type_for(*ret)),
            ),
            Ty::Struct { id, args } => Type::Struct {
                id: *id,
                args: args.iter().map(|arg| self.type_for(*arg)).collect(),
            },
            Ty::Enum { id, args } => Type::Enum {
                id: *id,
                args: args.iter().map(|arg| self.type_for(*arg)).collect(),
            },
            Ty::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.type_for(*inner)),
            },
            Ty::Pointer(inner) => Type::Pointer(Box::new(self.type_for(*inner))),
            Ty::TypeVar(id) => Type::TypeVar(*id),
            Ty::Generic(param) => Type::Generic(*param),
            Ty::Projection {
                ty,
                trait_id,
                assoc_type,
                trait_args,
            } => Type::Projection {
                ty: Box::new(self.type_for(*ty)),
                trait_id: *trait_id,
                assoc_type: *assoc_type,
                trait_args: trait_args.iter().map(|arg| self.type_for(*arg)).collect(),
            },
            Ty::Error => Type::Error,
        }
    }
```

- [ ] **Step 4: Run conversion and identity tests**

Run: `cargo test -p rock-lib type_context_`

Expected: PASS for all `type_context_*` tests.

- [ ] **Step 5: Run existing structural type identity and substitution tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib generic_param_identity_is_owner_and_index
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids
cargo test -p rock-lib substitute_uses_typed_type_var_id_keys
```

Expected: PASS for each command. These commands prove the scaffold did not weaken existing structural `Type` identity or inference substitution helpers.

- [ ] **Step 6: Commit compatibility conversions**

```bash
git add lib/src/type_context/mod.rs
git commit -m "add type context structural conversions"
```

## Task 4: Verify Artifact Boundary And Update Trackers

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Run artifact structural-type guard tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib test_product_artifact_preserves_associated_types
cargo test -p rock-lib product_artifact_remaps_projection_type_ids
```

Expected: PASS. These existing tests prove product artifacts still carry structural `Type` projection metadata and remap structural `Type` IDs rather than serialized `TypeId` values.

- [ ] **Step 2: Update the Roadmap Task 9 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, replace this line in the baseline bullets:

```markdown
- Roadmap Task 9 will use the long-term interned `Ty` / `TypeId` direction, but only as a scaffold slice first: structural `Type` remains the compatibility/serialization boundary until a later explicit phase-boundary migration task.
```

with:

```markdown
- Roadmap Task 9 has landed for the interned `Ty` / `TypeId` scaffold: `TypeContext` now owns canonical semantic type nodes, while structural `Type` remains the compatibility/serialization boundary until Task 11.
```

In the Task 9 section, replace this status line:

```markdown
**Status:** Designed for interned `Ty` / `TypeId` scaffolding in `docs/superpowers/specs/2026-05-18-type-context-scaffold-design.md`; implementation plan pending.
```

with:

```markdown
**Status:** Complete for interned `Ty` / `TypeId` scaffolding in `docs/superpowers/plans/2026-05-19-type-context-scaffold.md`.
```

In the focused-plan list, replace this bullet:

```markdown
5. `Type Context Scaffold`: design written in `docs/superpowers/specs/2026-05-18-type-context-scaffold-design.md`; covers the Task 9 decision to use interned `Ty` / `TypeId` as the long-term direction while deferring phase-boundary migration to Task 11.
```

with:

```markdown
5. `Type Context Scaffold`: complete in `docs/superpowers/plans/2026-05-19-type-context-scaffold.md`; covers Task 9 by adding interned `Ty` / `TypeId` identity and structural `Type` compatibility conversions while deferring phase-boundary migration to Task 11.
```

Replace this paragraph:

```markdown
The first four focused implementation plans are complete. The type-context scaffold design is now written, and later work can continue with type facts, `TypeId` phase-boundary migration, selection-service, instance, MIR, or codegen boundaries.
```

with:

```markdown
The first five focused implementation plans are complete. Later work can continue with type facts, `TypeId` phase-boundary migration, selection-service, instance, MIR, or codegen boundaries.
```

- [ ] **Step 3: Update the audit summary row**

In `docs/superpowers/plans/master-audit-checklist.md`, replace the `Type Context And Semantic Types` summary row with:

```markdown
| Type Context And Semantic Types | In progress | `lib/src/type_context/mod.rs`, `lib/src/types/mod.rs`, `lib/src/type_lowering.rs`, `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md` | Interned `Ty` / `TypeId` scaffolding and parsed type lowering are in place, but phase-boundary `TypeId` migration and dedicated type fact services remain |
```

- [ ] **Step 4: Update the audit track evidence and checkboxes**

In the `## 3. Type Context And Semantic Types` section of `docs/superpowers/plans/master-audit-checklist.md`, add this evidence bullet after the existing `TypeId` / `TypeVarId` bullet:

```markdown
- `lib/src/type_context/mod.rs` defines interned `Ty` nodes, a `TypeContext` arena keyed by `TypeId`, and explicit `Type -> TypeId -> Type` compatibility conversions.
```

In the same section, replace these unchecked items:

```markdown
- [ ] Decide whether to intern types and make `TypeId` authoritative.
- [ ] Introduce a semantic type context / `Ty` layer so structural `Type` cloning is not the only phase-wide type representation.
- [ ] Decide whether the shared parsed-type lowering boundary should become part of the future `Ty` / type-context service once Task 9 chooses the type representation model.
```

with these checked items:

```markdown
- [x] Decided to intern semantic `Ty` nodes and make `TypeId` authoritative inside `TypeContext`.
- [x] Introduced a semantic type context / `Ty` scaffold with explicit structural `Type` compatibility conversions.
- [x] Kept the shared parsed-type lowering boundary on structural `Type` for this scaffold slice so phase boundaries do not claim `TypeId` authority before Task 11.
```

Keep this unchecked item in the section:

```markdown
- [ ] Move type facts and semantic queries such as builtin index behavior, copy semantics, layout-facing facts, and display/symbol formatting out of ad hoc `Type` helpers into dedicated services or context tables.
```

Add this unchecked item immediately before the type-facts item:

```markdown
- [ ] Migrate selected HIR, inference, and artifact-facing phase boundaries from structural `Type` to context-owned `TypeId` after type fact services are stable.
```

- [ ] **Step 5: Run documentation diff checks**

Run: `git diff --check`

Expected: PASS with no whitespace errors.

- [ ] **Step 6: Commit tracker updates**

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for type context scaffold"
```

## Task 5: Final Verification

**Files:**
- Verify: whole workspace formatting and `rock-lib` tests

- [ ] **Step 1: Format the Rust changes**

Run: `cargo fmt --all`

Expected: command exits successfully.

- [ ] **Step 2: Verify formatting is stable**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 3: Verify the new type-context tests**

Run: `cargo test -p rock-lib type_context_`

Expected: PASS.

- [ ] **Step 4: Verify existing structural type identity tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib generic_param_identity_is_owner_and_index
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids
cargo test -p rock-lib substitute_uses_typed_type_var_id_keys
```

Expected: PASS for each command.

- [ ] **Step 5: Verify artifact structural-type tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib test_product_artifact_preserves_associated_types
cargo test -p rock-lib product_artifact_remaps_projection_type_ids
```

Expected: PASS for each command.

- [ ] **Step 6: Run the full documented library suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 7: Verify the final diff has no whitespace errors**

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 8: Record final state**

Run: `git status --short`

Expected: either clean, or only user-owned unrelated files such as `.sisyphus/` remain untracked.

## Self-Review Notes

- Spec coverage: Tasks 1-3 cover interned `Ty`, `TypeContext`, `intern_ty`, `ty`, `intern_type`, `type_for`, primitive/composite/nominal/generic/projection/type-var identity, and structural roundtrips. Task 4 covers roadmap/audit separation between scaffold and Task 11 migration. Task 5 covers focused and full verification.
- Non-goals preserved: no task changes HIR, inference, product artifact serialization, mono, MIR, codegen, copyability, builtin index behavior, projection normalization, display formatting, or layout-facing facts.
- Type consistency: all child type structure inside `Ty` uses `TypeId`, while compatibility methods consume and produce the existing structural `Type`.
