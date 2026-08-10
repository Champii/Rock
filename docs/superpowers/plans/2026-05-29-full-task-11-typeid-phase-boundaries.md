# Full Task 11 TypeId Phase Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Roadmap Task 11 by making context-owned `TypeId` the semantic type carrier across mono, MIR, borrowck, and codegen while keeping product artifacts as an explicit structural compatibility boundary.

**Architecture:** Add a narrow type-view API over `TypeContext`, complete required HIR TypeId sidecar coverage, then migrate mono instance identity, MIR type-bearing fields, borrowck/type queries, and codegen layout/ABI APIs to `TypeId`. Structural `Type` remains allowed only for parser/lowering construction, diagnostics/display, and product artifact write/read conversion.

**Tech Stack:** Rust 2021, `rock-lib`, `TypeContext`, `TypeId`, HIR/MIR/mono/codegen/product modules, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Design Spec

- `docs/superpowers/specs/2026-05-29-full-task-11-typeid-phase-boundaries-design.md`

## File Structure

- Create: `lib/src/type_context/view.rs`
  - Read-only and mutable views over `TypeContext` for downstream phases.
  - Shape helpers over `Ty` / `TypeId` so MIR, borrowck, and codegen do not pattern-match structural `Type` for semantic identity.
- Modify: `lib/src/type_context/mod.rs`
  - Export the view module.
  - Derive `Clone` for `TypeContext`.
  - Add immutable lookup helpers for already-interned structural compatibility types.
  - Add interned substitution helpers that rewrite `Ty` and return `TypeId`.
- Modify: `lib/src/hir/type_ids.rs`
  - Add required sidecar locations for cast targets, method-call trait args, and struct-pattern type args.
  - Add required lookup validation helpers.
- Modify: `lib/src/infer/mod.rs`
  - Finalization should produce a TypeId-complete resolved HIR and expose required lookup APIs.
- Modify: `lib/src/mono/registry.rs`, `lib/src/mono/mod.rs`, `lib/src/mono/specialize.rs`, `lib/src/mono/substitute.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/process.rs`, `lib/src/mono/external.rs`
  - Move mono instance identity and substitution from `Vec<Type>` to `Vec<TypeId>` / `HashMap<GenericParamId, TypeId>`.
  - Keep structural HIR rewriting only as a compatibility adapter fed by TypeId substitutions.
- Modify: `lib/src/mir/mod.rs`, `lib/src/mir/identity.rs`, `lib/src/mir/builder/**`, `lib/src/mir/agreement.rs`, `lib/src/mir/borrowck/**`
  - Store `TypeId` in MIR type-bearing fields.
  - Query type shape through the type view.
- Modify: `lib/src/codegen/mod.rs`, `lib/src/codegen/types.rs`, `lib/src/codegen/mir/**`, `lib/src/codegen/expr/**`, `lib/src/codegen/closures.rs`, `lib/src/codegen/stmt.rs`, `lib/src/codegen/control_flow.rs`, `lib/src/codegen/intrinsics.rs`, `lib/src/codegen/operators.rs`
  - Accept and use TypeId-aware type/layout/ABI APIs.
  - Keep structural reconstruction only in named compatibility helpers.
- Modify: `lib/src/products.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/crate_system/**`
  - Preserve structural product artifact payloads.
  - Re-intern loaded structural types into the consumer `TypeContext` before internal downstream use.
- Modify: `lib/src/lib.rs`
  - Thread the context-owned type data through mono, MIR, borrowck, agreement, and codegen.
- Modify docs after final review only:
  - `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - `docs/superpowers/plans/master-audit-checklist.md`

---

## Task 0: Baseline And Audit

**Files:**
- Read: `docs/superpowers/specs/2026-05-29-full-task-11-typeid-phase-boundaries-design.md`
- Read: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Read: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Capture clean starting status**

Run:

```bash
git status --short
git log --oneline -10
```

Expected: working tree clean; latest commits include `design full task 11 typeid migration`.

- [ ] **Step 2: Run baseline focused tests**

Run:

```bash
cargo test -p rock-lib type_context
cargo test -p rock-lib hir_type_ids
cargo test -p rock-lib type_id
cargo test -p rock-lib mono
cargo test -p rock-lib mir::builder
cargo test -p rock-lib codegen
cargo test -p rock-lib product_artifact
```

Expected: PASS. If a baseline fails, stop and diagnose before editing.

- [ ] **Step 3: Run baseline structural Type audit**

Run:

```bash
rg "Vec<Type>|HashMap<.*Type|: Type|Type\)|&Type|Type::" lib/src/mono lib/src/mir lib/src/codegen --glob '*.rs'
```

Expected: many matches. Save the output path or paste the output into implementation notes for later comparison.

- [ ] **Step 4: Commit baseline notes only if a notes file was created**

If no notes file was created, do not commit.

If a notes file was created, run:

```bash
git add docs/superpowers/plans/2026-05-29-full-task-11-typeid-phase-boundaries.md
git commit -m "record task 11 baseline audit"
```

Expected: commit succeeds only for intentionally changed plan notes.

---

## Task 1: Add TypeContext View And TypeId Helpers

**Files:**
- Create: `lib/src/type_context/view.rs`
- Modify: `lib/src/type_context/mod.rs`
- Test: `lib/src/type_context/mod.rs`

- [ ] **Step 1: Write failing TypeView tests**

Add this test module content near the existing `#[cfg(test)] mod tests` in `lib/src/type_context/mod.rs`:

```rust
#[test]
fn type_view_reports_shape_without_structural_type_storage() {
    let mut context = TypeContext::new();
    let elem = context.intern_type(&Type::U8);
    let slice = context.intern_ty(Ty::Slice(elem));
    let shared_ref = context.intern_ty(Ty::Reference {
        mutable: false,
        inner: slice,
    });
    let view = TypeView::new(&context);

    assert!(view.is_slice_shape(slice));
    assert!(view.is_fat_pointer_shape(shared_ref));
    assert!(view.is_copy(shared_ref));
    assert_eq!(view.builtin_index_output(slice, context.intern_type(&Type::I64)), Some(elem));
}

#[test]
fn type_context_finds_previously_interned_structural_type_without_mutation() {
    let mut context = TypeContext::new();
    let ty = Type::Function(vec![Type::I64], Box::new(Type::Bool));
    let id = context.intern_type(&ty);

    assert_eq!(context.id_for_type(&ty), Some(id));
    assert_eq!(context.type_for(id), ty);
}

#[test]
fn type_interner_substitutes_generics_to_type_ids() {
    let owner = def_id(70);
    let generic_param = GenericParamId { owner, index: 0 };
    let mut context = TypeContext::new();
    let generic = context.intern_ty(Ty::Generic(generic_param));
    let replacement = context.intern_type(&Type::U64);
    let tuple = context.intern_ty(Ty::Tuple(vec![generic]));
    let mut subst = HashMap::new();
    subst.insert(generic_param, replacement);

    let substituted = context.substitute_generics(tuple, &subst);

    assert_eq!(context.type_for(substituted), Type::Tuple(vec![Type::U64]));
}
```

Also add this import inside the test module:

```rust
use crate::type_context::TypeView;
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p rock-lib type_view_reports_shape_without_structural_type_storage
cargo test -p rock-lib type_context_finds_previously_interned_structural_type_without_mutation
cargo test -p rock-lib type_interner_substitutes_generics_to_type_ids
```

Expected: FAIL with missing `TypeView`, `id_for_type`, and `substitute_generics` APIs.

- [ ] **Step 3: Create `lib/src/type_context/view.rs`**

Create the file with:

```rust
use crate::ids::TypeId;
use crate::type_context::{Ty, TypeContext};

#[derive(Clone, Copy)]
pub struct TypeView<'a> {
    context: &'a TypeContext,
}

impl<'a> TypeView<'a> {
    pub fn new(context: &'a TypeContext) -> Self {
        Self { context }
    }

    pub fn ty(&self, id: TypeId) -> &'a Ty {
        self.context.ty(id)
    }

    pub fn is_slice_shape(&self, id: TypeId) -> bool {
        matches!(self.ty(id), Ty::Slice(_))
    }

    pub fn is_str_shape(&self, id: TypeId) -> bool {
        matches!(self.ty(id), Ty::Str)
    }

    pub fn is_fat_pointer_shape(&self, id: TypeId) -> bool {
        match self.ty(id) {
            Ty::Reference { inner, .. } | Ty::Pointer(inner) => {
                matches!(self.ty(*inner), Ty::Slice(_) | Ty::Str)
            }
            _ => false,
        }
    }

    pub fn is_copy(&self, id: TypeId) -> bool {
        match self.ty(id) {
            Ty::I8
            | Ty::I16
            | Ty::I32
            | Ty::I64
            | Ty::U8
            | Ty::U16
            | Ty::U32
            | Ty::U64
            | Ty::F32
            | Ty::F64
            | Ty::Bool
            | Ty::Char
            | Ty::Unit
            | Ty::Str
            | Ty::Slice(_)
            | Ty::Pointer(_)
            | Ty::Function { .. } => true,
            Ty::Reference { mutable: false, .. } => true,
            Ty::Tuple(elems) => elems.iter().all(|elem| self.is_copy(*elem)),
            Ty::Reference { mutable: true, .. }
            | Ty::Array { .. }
            | Ty::Projection { .. }
            | Ty::Struct { .. }
            | Ty::Enum { .. }
            | Ty::TypeVar(_)
            | Ty::Generic(_)
            | Ty::Error
            | Ty::Never => false,
        }
    }

    pub fn builtin_index_output(&self, receiver: TypeId, index: TypeId) -> Option<TypeId> {
        if !matches!(self.ty(index), Ty::I64) {
            return None;
        }

        match self.ty(receiver) {
            Ty::Slice(inner) | Ty::Array { inner, .. } => Some(*inner),
            Ty::Pointer(inner) => match self.ty(*inner) {
                Ty::Slice(elem) => Some(*elem),
                _ => Some(*inner),
            },
            _ => None,
        }
    }
}
```

- [ ] **Step 4: Update `lib/src/type_context/mod.rs` exports and helpers**

At the top of `lib/src/type_context/mod.rs`, add:

```rust
mod view;

pub use view::TypeView;
```

Change `TypeContext` derive to:

```rust
#[derive(Debug, Default, Clone)]
pub struct TypeContext {
    tys: Vec<Ty>,
    interned: HashMap<Ty, TypeId>,
    ids: IdGen<TypeId>,
}
```

Add these methods to `impl TypeContext` below `type_for`:

```rust
    pub fn id_for_type(&self, ty: &Type) -> Option<TypeId> {
        let ty = self.ty_for_existing_type(ty)?;
        self.interned.get(&ty).copied()
    }

    fn ty_for_existing_type(&self, ty: &Type) -> Option<Ty> {
        Some(match ty {
            Type::I8 => Ty::I8,
            Type::I16 => Ty::I16,
            Type::I32 => Ty::I32,
            Type::I64 => Ty::I64,
            Type::U8 => Ty::U8,
            Type::U16 => Ty::U16,
            Type::U32 => Ty::U32,
            Type::U64 => Ty::U64,
            Type::F32 => Ty::F32,
            Type::F64 => Ty::F64,
            Type::Bool => Ty::Bool,
            Type::Str => Ty::Str,
            Type::Char => Ty::Char,
            Type::Unit => Ty::Unit,
            Type::Never => Ty::Never,
            Type::Slice(inner) => Ty::Slice(self.id_for_type(inner)?),
            Type::Array(inner, len) => Ty::Array {
                inner: self.id_for_type(inner)?,
                len: *len,
            },
            Type::Tuple(elems) => Ty::Tuple(
                elems
                    .iter()
                    .map(|elem| self.id_for_type(elem))
                    .collect::<Option<Vec<_>>>()?,
            ),
            Type::Function(args, ret) => Ty::Function {
                params: args
                    .iter()
                    .map(|arg| self.id_for_type(arg))
                    .collect::<Option<Vec<_>>>()?,
                ret: self.id_for_type(ret)?,
            },
            Type::Struct { id, args } => Ty::Struct {
                id: *id,
                args: args
                    .iter()
                    .map(|arg| self.id_for_type(arg))
                    .collect::<Option<Vec<_>>>()?,
            },
            Type::Enum { id, args } => Ty::Enum {
                id: *id,
                args: args
                    .iter()
                    .map(|arg| self.id_for_type(arg))
                    .collect::<Option<Vec<_>>>()?,
            },
            Type::Reference { mutable, inner } => Ty::Reference {
                mutable: *mutable,
                inner: self.id_for_type(inner)?,
            },
            Type::Pointer(inner) => Ty::Pointer(self.id_for_type(inner)?),
            Type::TypeVar(id) => Ty::TypeVar(*id),
            Type::Generic(param) => Ty::Generic(*param),
            Type::Projection {
                ty,
                trait_id,
                assoc_type,
                trait_args,
            } => Ty::Projection {
                ty: self.id_for_type(ty)?,
                trait_id: *trait_id,
                assoc_type: *assoc_type,
                trait_args: trait_args
                    .iter()
                    .map(|arg| self.id_for_type(arg))
                    .collect::<Option<Vec<_>>>()?,
            },
            Type::Error => Ty::Error,
        })
    }

    pub fn substitute_generics(
        &mut self,
        id: TypeId,
        subst: &HashMap<GenericParamId, TypeId>,
    ) -> TypeId {
        match self.ty(id).clone() {
            Ty::Generic(param) => subst.get(&param).copied().unwrap_or(id),
            Ty::Slice(inner) => {
                let inner = self.substitute_generics(inner, subst);
                self.intern_ty(Ty::Slice(inner))
            }
            Ty::Array { inner, len } => {
                let inner = self.substitute_generics(inner, subst);
                self.intern_ty(Ty::Array { inner, len })
            }
            Ty::Tuple(elems) => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.substitute_generics(elem, subst))
                    .collect();
                self.intern_ty(Ty::Tuple(elems))
            }
            Ty::Function { params, ret } => {
                let params = params
                    .into_iter()
                    .map(|param| self.substitute_generics(param, subst))
                    .collect();
                let ret = self.substitute_generics(ret, subst);
                self.intern_ty(Ty::Function { params, ret })
            }
            Ty::Struct { id: owner, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| self.substitute_generics(arg, subst))
                    .collect();
                self.intern_ty(Ty::Struct { id: owner, args })
            }
            Ty::Enum { id: owner, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| self.substitute_generics(arg, subst))
                    .collect();
                self.intern_ty(Ty::Enum { id: owner, args })
            }
            Ty::Reference { mutable, inner } => {
                let inner = self.substitute_generics(inner, subst);
                self.intern_ty(Ty::Reference { mutable, inner })
            }
            Ty::Pointer(inner) => {
                let inner = self.substitute_generics(inner, subst);
                self.intern_ty(Ty::Pointer(inner))
            }
            Ty::Projection {
                ty,
                trait_id,
                assoc_type,
                trait_args,
            } => {
                let ty = self.substitute_generics(ty, subst);
                let trait_args = trait_args
                    .into_iter()
                    .map(|arg| self.substitute_generics(arg, subst))
                    .collect();
                self.intern_ty(Ty::Projection {
                    ty,
                    trait_id,
                    assoc_type,
                    trait_args,
                })
            }
            primitive => self.intern_ty(primitive),
        }
    }
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p rock-lib type_view_reports_shape_without_structural_type_storage
cargo test -p rock-lib type_context_finds_previously_interned_structural_type_without_mutation
cargo test -p rock-lib type_interner_substitutes_generics_to_type_ids
```

Expected: PASS.

- [ ] **Step 6: Run full type context tests**

Run:

```bash
cargo test -p rock-lib type_context
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add lib/src/type_context/mod.rs lib/src/type_context/view.rs
git commit -m "add type context view helpers"
```

Expected: commit succeeds.

---

## Task 2: Complete HIR TypeId Coverage And Required Lookups

**Files:**
- Modify: `lib/src/hir/type_ids.rs`
- Modify: `lib/src/infer/mod.rs`
- Test: `lib/src/hir/type_ids.rs`
- Test: `lib/src/infer/mod.rs`

- [ ] **Step 1: Write failing sidecar coverage tests**

In `lib/src/hir/type_ids.rs`, add a test that constructs an expression containing a cast, method target trait args, and struct-pattern type args. Extend the existing `crate::hir` test import with `HirMethodCallTarget`, `HirPattern`, and `HirStmt`, then add this assertion shape:

```rust
#[test]
fn hir_type_ids_collect_downstream_required_payload_types() {
    let owner = def_id(200);
    let trait_id = def_id(201);
    let struct_id = def_id(202);
    let method_id = def_id(203);
    let mut context = TypeContext::new();
    let expr = HirExpr {
        kind: HirExprKind::Cast(
            Box::new(HirExpr {
                kind: HirExprKind::MethodCall(
                    Box::new(HirExpr {
                        kind: HirExprKind::Var("receiver".to_string()),
                        ty: Type::Struct {
                            id: struct_id,
                            args: vec![Type::I64],
                        },
                        span: Default::default(),
                    }),
                    "value".to_string(),
                    Vec::new(),
                    None,
                    Some(HirMethodCallTarget {
                        impl_id: Some(owner),
                        trait_id: Some(trait_id),
                        trait_args: vec![Type::Bool],
                        method_id,
                        from_index_operator: false,
                    }),
                ),
                ty: Type::I64,
                span: Default::default(),
            }),
            Type::U64,
        ),
        ty: Type::U64,
        span: Default::default(),
    };
    let pattern = HirPattern::Struct(
        "Box".to_string(),
        Some(struct_id),
        vec![Type::Bool],
        Vec::new(),
    );
    let program = program_with_function_body(owner, HirBlock {
        stmts: vec![HirStmt::Expr(expr)],
        ty: Type::Unit,
    });
    let ids = collect_hir_type_ids(&program, &mut context);

    assert!(ids.get(&HirTypeLocation::CastTarget {
        owner,
        path: vec![0, 0],
    }).is_some());
    assert!(ids.get(&HirTypeLocation::MethodCallTraitArg {
        owner,
        path: vec![0, 0, 0],
        index: 0,
    }).is_some());

    let mut pattern_context = TypeContext::new();
    let mut pattern_ids = HirTypeIds::new();
    collect_pattern_type_ids(owner, &pattern, vec![9], &mut pattern_context, &mut pattern_ids);
    assert!(pattern_ids.get(&HirTypeLocation::StructPatternArg {
        owner,
        path: vec![9],
        index: 0,
    }).is_some());
}
```

Add this local helper below the existing test helpers:

```rust
fn program_with_function_body(owner: DefId, body: HirBlock) -> HirProgram {
    let function = HirFunction {
        id: owner,
        name: "main".to_string(),
        qualified_name: None,
        generic_params: Vec::new(),
        generic_param_ids: Vec::new(),
        generic_bounds: HashMap::new(),
        params: Vec::new(),
        ret_type: body.ty.clone(),
        body,
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };
    HirProgram::from_parts(
        HashMap::from([("main".to_string(), function)]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    )
}
```

- [ ] **Step 2: Run the new test to verify it fails**

Run:

```bash
cargo test -p rock-lib hir_type_ids_collect_downstream_required_payload_types
```

Expected: FAIL with missing `HirTypeLocation` variants and inaccessible pattern collection helper.

- [ ] **Step 3: Add required location variants**

In `HirTypeLocation`, add:

```rust
    CastTarget {
        owner: DefId,
        path: Vec<usize>,
    },
    MethodCallTraitArg {
        owner: DefId,
        path: Vec<usize>,
        index: usize,
    },
    StructPatternArg {
        owner: DefId,
        path: Vec<usize>,
        index: usize,
    },
```

- [ ] **Step 4: Record cast and method trait arg payloads**

In `collect_expr`, replace the combined cast arm with a dedicated arm:

```rust
        HirExprKind::Cast(inner, target_ty) => {
            record_type(
                context,
                ids,
                HirTypeLocation::CastTarget {
                    owner,
                    path: path.clone(),
                },
                target_ty,
            );
            collect_expr(owner, inner, child_path(&path, 0), context, ids);
        }
```

In the `MethodCall` arm, record trait args before collecting children:

```rust
        HirExprKind::MethodCall(receiver, _, args, _, target) => {
            if let Some(target) = target {
                for (index, ty) in target.trait_args.iter().enumerate() {
                    record_type(
                        context,
                        ids,
                        HirTypeLocation::MethodCallTraitArg {
                            owner,
                            path: path.clone(),
                            index,
                        },
                        ty,
                    );
                }
            }
            collect_expr(owner, receiver, child_path(&path, 0), context, ids);
            for (index, arg) in args.iter().enumerate() {
                collect_expr(owner, arg, child_path(&path, index + 1), context, ids);
            }
        }
```

- [ ] **Step 5: Add pattern type collection**

In `lib/src/hir/type_ids.rs`, add a public crate helper below `collect_stmt`:

```rust
pub(crate) fn collect_pattern_type_ids(
    owner: DefId,
    pattern: &HirPattern,
    path: Vec<usize>,
    context: &mut TypeContext,
    ids: &mut HirTypeIds,
) {
    match pattern {
        HirPattern::Struct(_, _, type_args, fields) => {
            for (index, ty) in type_args.iter().enumerate() {
                record_type(
                    context,
                    ids,
                    HirTypeLocation::StructPatternArg {
                        owner,
                        path: path.clone(),
                        index,
                    },
                    ty,
                );
            }
            for (index, field) in fields.iter().enumerate() {
                collect_pattern_type_ids(
                    owner,
                    &field.pattern,
                    child_path(&path, index),
                    context,
                    ids,
                );
            }
        }
        HirPattern::Tuple(patterns) | HirPattern::Or(patterns) => {
            for (index, pattern) in patterns.iter().enumerate() {
                collect_pattern_type_ids(owner, pattern, child_path(&path, index), context, ids);
            }
        }
        HirPattern::Enum(_, _, _, patterns) => {
            for (index, pattern) in patterns.iter().enumerate() {
                collect_pattern_type_ids(owner, pattern, child_path(&path, index), context, ids);
            }
        }
        HirPattern::Wildcard | HirPattern::Binding { .. } | HirPattern::Literal(_) => {}
    }
}
```

Call it from `collect_expr` in `HirExprKind::Match` before collecting arm guards/bodies:

```rust
                collect_pattern_type_ids(
                    owner,
                    &arm.pattern,
                    child_path(&path, 1000 + index),
                    context,
                    ids,
                );
```

Use the high offset `1000 + index` to avoid colliding with existing expression/block child paths.

- [ ] **Step 6: Add required lookup API in `infer/mod.rs`**

Add this method to `impl ResolvedHirProgram`:

```rust
    pub fn require_type_id_at(&self, location: HirTypeLocation) -> Result<TypeId, ResolveError> {
        self.type_id_at(&location).ok_or_else(|| {
            ResolveError::new(format!(
                "missing finalized HIR TypeId at {:?}",
                location
            ))
        })
    }
```

Add a focused test in `infer/mod.rs`:

```rust
#[test]
fn resolved_hir_reports_missing_required_type_id_location() {
    let resolved = finalize(partial_hir_with_traits(HashMap::new())).unwrap();
    let missing = HirTypeLocation::FunctionReturn {
        function: DefId::new(CrateId(0), LocalDefId(999)),
    };

    let err = resolved.require_type_id_at(missing).unwrap_err();

    assert!(err.message.contains("missing finalized HIR TypeId"));
}
```

- [ ] **Step 7: Run focused tests**

Run:

```bash
cargo test -p rock-lib hir_type_ids_collect_downstream_required_payload_types
cargo test -p rock-lib resolved_hir_reports_missing_required_type_id_location
cargo test -p rock-lib hir_type_ids
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add lib/src/hir/type_ids.rs lib/src/infer/mod.rs
git commit -m "complete finalized hir type id coverage"
```

Expected: commit succeeds.

---

## Task 3: Thread TypeContext Through MonomorphizedProgram And Instance Identity

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/registry.rs`
- Test: `lib/src/mono/mod.rs`

- [ ] **Step 1: Write failing instance registry TypeId tests**

In `lib/src/mono/registry.rs`, replace test helper substitutions with TypeIds and add:

```rust
#[test]
fn instance_registry_uses_type_ids_for_substitution_identity() {
    let mut context = crate::type_context::TypeContext::new();
    let first_ty = context.intern_type(&Type::Struct {
        id: DefId::new(CrateId(0), LocalDefId(10)),
        args: Vec::new(),
    });
    let second_ty = context.intern_type(&Type::Struct {
        id: DefId::new(CrateId(0), LocalDefId(11)),
        args: Vec::new(),
    });
    let mut registry = InstanceRegistry::new();
    let function = DefId::new(CrateId(0), LocalDefId(1));
    let first_key = InstanceKey::new(InstanceOrigin::Function(function), vec![first_ty]);
    let second_key = InstanceKey::new(InstanceOrigin::Function(function), vec![second_ty]);

    let first = registry.intern(first_key.clone(), |id| InstanceRecord {
        id,
        origin: first_key.origin.clone(),
        substitution: first_key.substitution.clone(),
        source_name: "id".to_string(),
        backend_symbol: "id_first".to_string(),
        declared: None,
        body: None,
        provided_by_object: false,
        is_specialization: true,
    });
    let second = registry.intern(second_key.clone(), |id| InstanceRecord {
        id,
        origin: second_key.origin.clone(),
        substitution: second_key.substitution.clone(),
        source_name: "id".to_string(),
        backend_symbol: "id_second".to_string(),
        declared: None,
        body: None,
        provided_by_object: false,
        is_specialization: true,
    });

    assert_ne!(first, second);
    assert_eq!(registry.len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p rock-lib instance_registry_uses_type_ids_for_substitution_identity
```

Expected: FAIL because `InstanceKey::new` expects `Vec<Type>`.

- [ ] **Step 3: Change registry substitution fields to TypeId**

In `lib/src/mono/registry.rs`, change imports and fields:

```rust
use crate::ids::{DefId, InstanceId, TypeId};
use crate::type_context::TypeContext;
```

Change `InstanceKey` and `InstanceRecord`:

```rust
pub struct InstanceKey {
    pub origin: InstanceOrigin,
    pub substitution: Vec<TypeId>,
}

impl InstanceKey {
    pub fn new(origin: InstanceOrigin, substitution: Vec<TypeId>) -> Self {
        Self { origin, substitution }
    }
}

pub struct InstanceRecord {
    pub id: InstanceId,
    pub origin: InstanceOrigin,
    pub substitution: Vec<TypeId>,
    pub source_name: String,
    pub backend_symbol: String,
    pub declared: Option<HirFunction>,
    pub body: Option<HirFunction>,
    pub provided_by_object: bool,
    pub is_specialization: bool,
}
```

Change `MonomorphizedProgram`:

```rust
pub struct MonomorphizedProgram {
    pub program: HirProgram,
    pub instances: BTreeMap<InstanceId, InstanceRecord>,
    pub type_context: TypeContext,
}

impl MonomorphizedProgram {
    pub fn new(program: HirProgram, type_context: TypeContext) -> Self {
        Self {
            program,
            instances: BTreeMap::new(),
            type_context,
        }
    }
}
```

- [ ] **Step 4: Thread type context into `Monomorphizer`**

In `lib/src/mono/mod.rs`, add fields to `Monomorphizer`:

```rust
    type_context: crate::type_context::TypeContext,
```

Change `Monomorphizer::new()` to initialize `TypeContext::new()`.

Add a constructor:

```rust
    fn with_type_context(type_context: crate::type_context::TypeContext) -> Self {
        Self {
            type_context,
            ..Self::new()
        }
    }
```

Change `monomorphize_with_crates` to destructure the resolved HIR:

```rust
pub fn monomorphize_with_crates(
    program: ResolvedHirProgram,
    crate_ctx: &CrateContext,
) -> MonomorphizedProgram {
    let ResolvedHirProgram {
        program,
        resolver,
        type_context,
        ..
    } = program;
    let mut mono = Monomorphizer::with_type_context(type_context);
    mono.resolver = resolver;
    let program = mono.process_with_crates(program, crate_ctx);
    MonomorphizedProgram {
        program,
        instances: mono.instances.into_records(),
        type_context: mono.type_context,
    }
}
```

Change the legacy `monomorphize(program: HirProgram) -> HirProgram` helper to keep a local context and still return `HirProgram`:

```rust
pub fn monomorphize(program: HirProgram) -> HirProgram {
    let mut mono = Monomorphizer::new();
    mono.process(program)
}
```

- [ ] **Step 5: Add compatibility helper for Type -> TypeId inside mono**

Add to `impl Monomorphizer`:

```rust
    fn intern_type(&mut self, ty: &Type) -> crate::ids::TypeId {
        self.type_context.intern_type(ty)
    }

    fn type_for(&self, id: crate::ids::TypeId) -> Type {
        self.type_context.type_for(id)
    }

    fn intern_types(&mut self, types: &[Type]) -> Vec<crate::ids::TypeId> {
        types.iter().map(|ty| self.intern_type(ty)).collect()
    }
```

Use these helpers anywhere an `InstanceKey::new` or `InstanceRecord.substitution` is created from structural types.

- [ ] **Step 6: Update registry tests to use TypeIds**

In `lib/src/mono/registry.rs` tests, add helper:

```rust
fn i64_type_id() -> crate::ids::TypeId {
    let mut context = crate::type_context::TypeContext::new();
    context.intern_type(&Type::I64)
}
```

Replace `vec![Type::I64]` in registry keys with `vec![i64_type_id()]`.

- [ ] **Step 7: Run mono registry tests**

Run:

```bash
cargo test -p rock-lib mono::registry
```

Expected: PASS.

- [ ] **Step 8: Run compile check for mono errors**

Run:

```bash
cargo test -p rock-lib mono --no-run
```

Expected: PASS compilation. If errors remain, update each `InstanceKey::new(origin, structural_types)` call to pass `self.intern_types(&structural_types)` or `Vec::<TypeId>::new()`.

- [ ] **Step 9: Commit**

Run:

```bash
git add lib/src/mono/registry.rs lib/src/mono/mod.rs
git commit -m "thread type context through mono instances"
```

Expected: commit succeeds.

---

## Task 4: Convert Mono Substitution And Specialization Identity To TypeIds

**Files:**
- Modify: `lib/src/mono/specialize.rs`
- Modify: `lib/src/mono/substitute.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`
- Test: existing mono tests plus new TypeId substitution tests

- [ ] **Step 1: Write failing mono substitution test**

In `lib/src/mono/substitute.rs`, add:

```rust
#[test]
fn type_id_substitution_rewrites_generic_projection_args() {
    let owner = def_id(90);
    let trait_id = def_id(91);
    let generic = GenericParamId { owner, index: 0 };
    let assoc = AssociatedTypeKey {
        owner: trait_id,
        assoc_type_id: AssocTypeId(0),
    };
    let mut context = TypeContext::new();
    let base = context.intern_ty(Ty::Generic(generic));
    let projection = context.intern_ty(Ty::Projection {
        ty: base,
        trait_id,
        assoc_type: assoc,
        trait_args: vec![base],
    });
    let replacement = context.intern_type(&Type::Struct {
        id: def_id(92),
        args: Vec::new(),
    });
    let mut subst = HashMap::new();
    subst.insert(generic, replacement);

    let rewritten = context.substitute_generics(projection, &subst);

    assert_eq!(
        context.type_for(rewritten),
        Type::Projection {
            ty: Box::new(Type::Struct {
                id: def_id(92),
                args: Vec::new(),
            }),
            trait_id,
            assoc_type: assoc,
            trait_args: vec![Type::Struct {
                id: def_id(92),
                args: Vec::new(),
            }],
        }
    );
}
```

Import `TypeContext`, `Ty`, `AssocTypeId`, and `AssociatedTypeKey` in the test module.

- [ ] **Step 2: Run test to verify it passes with Task 1 helper**

Run:

```bash
cargo test -p rock-lib type_id_substitution_rewrites_generic_projection_args
```

Expected: PASS. This test protects the helper used by mono migration.

- [ ] **Step 3: Replace mono substitution map aliases**

In mono modules, replace semantic substitution maps:

```rust
HashMap<GenericParamId, Type>
```

with:

```rust
HashMap<GenericParamId, TypeId>
```

Use this import where needed:

```rust
use crate::ids::TypeId;
```

When a structural HIR field must be rewritten for compatibility, convert the `TypeId` substitution back through the mono context at the adapter edge:

```rust
let structural_subst: HashMap<GenericParamId, Type> = subst
    .iter()
    .map(|(param, ty_id)| (*param, self.type_context.type_for(*ty_id)))
    .collect();
```

- [ ] **Step 4: Update `current_type_args`, `type_impls`, and `var_types`**

In `Monomorphizer`, change:

```rust
current_type_args: Vec<TypeId>,
type_impls: HashMap<String, Vec<(String, Vec<TypeId>)>>,
var_types: HashMap<String, TypeId>,
```

At HIR compatibility boundaries, reconstruct structural values through:

```rust
let ty = self.type_for(ty_id);
```

At source HIR inputs, intern through:

```rust
let ty_id = self.intern_type(&expr.ty);
```

- [ ] **Step 5: Update generic call specialization keys**

In `specialize.rs`, functions that return or accept inferred type arguments should return `Vec<TypeId>` for identity. Keep the specialized HIR body structural by constructing a structural adapter map immediately before calling existing structural HIR substitution.

Use this pattern:

```rust
let inferred_type_ids: Vec<TypeId> = inferred_types
    .iter()
    .map(|ty| self.intern_type(ty))
    .collect();
let structural_subst: HashMap<GenericParamId, Type> = type_param_ids
    .iter()
    .copied()
    .zip(inferred_type_ids.iter().map(|id| self.type_for(*id)))
    .collect();
```

- [ ] **Step 6: Update method instance lookup keys**

In `methods.rs` and `process.rs`, make selected method instance lookups pass `Vec<TypeId>` substitutions into `InstanceKey`. For every receiver/trait arg `Vec<Type>`, call `self.intern_types(&args)` before constructing the key.

Use this exact construction shape:

```rust
let substitution = self.intern_types(&receiver_arg_types);
let key = InstanceKey::new(origin.clone(), substitution.clone());
let instance_id = self.instances.intern(key, |id| InstanceRecord {
    id,
    origin,
    substitution,
    source_name: source_name.to_string(),
    backend_symbol,
    declared: Some(declared.clone()),
    body: Some(body.clone()),
    provided_by_object: false,
    is_specialization: true,
});
```

- [ ] **Step 7: Update object-backed external instance registration**

In `external.rs`, when artifact generic substitutions are loaded as structural `Type`, immediately intern them into the current mono context:

```rust
let substitution = self.intern_types(&structural_substitution);
```

Do not store producer artifact TypeIds.

- [ ] **Step 8: Run mono focused tests**

Run:

```bash
cargo test -p rock-lib mono::registry
cargo test -p rock-lib mono::specialize
cargo test -p rock-lib mono::methods
cargo test -p rock-lib mono::process
cargo test -p rock-lib mono::external
```

Expected: PASS.

- [ ] **Step 9: Run generic integration filters**

Run:

```bash
cargo test -p rock-lib generic
cargo test -p rock-lib projection
cargo test -p rock-lib trait_default
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```bash
git add lib/src/mono/specialize.rs lib/src/mono/substitute.rs lib/src/mono/methods.rs lib/src/mono/process.rs lib/src/mono/external.rs
git commit -m "migrate mono substitutions to type ids"
```

Expected: commit succeeds.

---

## Task 5: Store TypeIds In MIR

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/identity.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/src/mir/builder/mod.rs`
- Test: `lib/src/mir/identity.rs`

- [ ] **Step 1: Write failing MIR TypeId storage test**

In `lib/src/mir/builder/mod.rs`, add:

```rust
#[test]
fn mir_builder_records_type_ids_for_return_locals_and_casts() {
    use crate::hir::{HirBlock, HirStmt, HirVarRef, HirVarTarget};
    use crate::mono::{InstanceOrigin, InstanceRecord, MonomorphizedProgram};

    let function_id = DefId::new(CrateId(0), LocalDefId(80));
    let instance_id = InstanceId(80);
    let local_id = crate::ids::HirLocalId(0);
    let cast_value = expr(
        HirExprKind::Cast(
            Box::new(expr(HirExprKind::IntLiteral(1), Type::I64)),
            Type::U64,
        ),
        Type::U64,
    );
    let body = HirBlock {
        stmts: vec![
            HirStmt::Let {
                name: "x".to_string(),
                local_id,
                ty: Type::U64,
                value: cast_value,
                mutable: false,
            },
            HirStmt::Expr(expr(
                HirExprKind::ResolvedVar(HirVarRef {
                    name: "x".to_string(),
                    target: HirVarTarget::Local(local_id),
                }),
                Type::U64,
            )),
        ],
        ty: Type::U64,
    };
    let function = HirFunction {
        id: function_id,
        name: "main".to_string(),
        qualified_name: Some("main".to_string()),
        generic_params: Vec::new(),
        generic_param_ids: Vec::new(),
        generic_bounds: std::collections::HashMap::new(),
        params: Vec::new(),
        ret_type: Type::U64,
        body,
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };
    let program = HirProgram::from_parts(
        std::collections::HashMap::from([("main".to_string(), function.clone())]),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    let mut type_context = crate::type_context::TypeContext::new();
    let _ = crate::hir::collect_hir_type_ids(&program, &mut type_context);
    let u64_id = type_context.id_for_type(&Type::U64).unwrap();
    let mut instances = std::collections::BTreeMap::new();
    instances.insert(
        instance_id,
        InstanceRecord {
            id: instance_id,
            origin: InstanceOrigin::Function(function_id),
            substitution: Vec::new(),
            source_name: "main".to_string(),
            backend_symbol: "main".to_string(),
            declared: None,
            body: Some(function),
            provided_by_object: false,
            is_specialization: false,
        },
    );
    let monomorphized = MonomorphizedProgram {
        program,
        instances,
        type_context,
    };

    let mir = MirBuilder::build_monomorphized(&monomorphized);
    let main = mir.function(crate::mir::MirFunctionId::Instance(instance_id)).unwrap();
    let view = crate::type_context::TypeView::new(&monomorphized.type_context);

    assert_eq!(main.ret_type, u64_id);
    assert!(main.local_decls.iter().any(|local| local.ty == u64_id));
    assert!(main.basic_blocks.iter().flat_map(|block| &block.statements).any(|stmt| {
        matches!(
            &stmt.kind,
            crate::mir::StatementKind::Assign(_, crate::mir::Rvalue::Cast(_, target))
                if *target == u64_id && matches!(view.ty(*target), crate::type_context::Ty::U64)
        )
    }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p rock-lib mir_builder_records_type_ids_for_return_locals_and_casts
```

Expected: FAIL because MIR stores structural `Type`.

- [ ] **Step 3: Change MIR type-bearing fields**

In `lib/src/mir/mod.rs`, replace structural type fields:

```rust
use crate::ids::{TypeId, VariantId};

pub struct MirFunction {
    pub id: MirFunctionId,
    pub name: String,
    pub basic_blocks: Vec<BasicBlock>,
    pub local_decls: Vec<LocalDecl>,
    pub closure_captures: Vec<MirClosureCapture>,
    pub arg_count: usize,
    pub ret_type: TypeId,
}

pub struct LocalDecl {
    pub ty: TypeId,
    pub mutability: Mutability,
    pub name: Option<String>,
    pub span: Option<Span>,
}

pub enum Rvalue {
    Use(Operand),
    Ref(Mutability, Place),
    Cast(Operand, TypeId),
    Closure(MirClosure),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnaryOp, Operand),
    Discriminant(Place),
    Aggregate(AggregateKind, Vec<Operand>),
}
```

- [ ] **Step 4: Change MIR callable trait args**

In `lib/src/mir/identity.rs`, replace:

```rust
pub enum MirCallable {
    Method {
        impl_id: Option<DefId>,
        trait_id: Option<DefId>,
        trait_args: Vec<TypeId>,
        method_id: DefId,
        instance: Option<InstanceId>,
        display_name: String,
    },
}
```

Change the import to:

```rust
use crate::ids::{DefId, FieldId, InstanceId, TypeId, VariantId};
```

- [ ] **Step 5: Update MirBuilder constructor to carry TypeContext**

In `lib/src/mir/builder/mod.rs`, add field:

```rust
type_context: &'a crate::type_context::TypeContext,
```

Change constructors:

```rust
pub fn new(program: &'a HirProgram, type_context: &'a crate::type_context::TypeContext) -> Self
```

and:

```rust
fn with_method_instances(
    program: &'a HirProgram,
    type_context: &'a crate::type_context::TypeContext,
    method_instances: HashMap<(DefId, DefId, Vec<TypeId>), InstanceId>,
    method_instance_receiver_modes: HashMap<InstanceId, Option<crate::ast::SelfReceiverMode>>,
) -> Self
```

Add helper:

```rust
fn type_id_for(&self, ty: &Type) -> TypeId {
    self.type_context.id_for_type(ty).unwrap_or_else(|| {
        panic!("MIR builder received non-interned finalized type: {:?}", ty)
    })
}
```

- [ ] **Step 6: Update MIR builder allocation and method maps**

Change `method_instances` map type to:

```rust
HashMap<(DefId, DefId, Vec<TypeId>), InstanceId>
```

Change `new_local` signatures:

```rust
fn new_local(&mut self, ty: TypeId, mutability: Mutability, name: Option<String>) -> Local
fn new_local_with_span(&mut self, ty: TypeId, mutability: Mutability, name: Option<String>, span: Span) -> Local
```

At every caller that currently passes `expr.ty.clone()`, pass:

```rust
self.type_id_for(&expr.ty)
```

At every caller that has only a structural `Type`, pass:

```rust
self.type_id_for(&ty)
```

- [ ] **Step 7: Update `build_monomorphized` entry point**

Change:

```rust
pub fn build_monomorphized(program: &crate::mono::MonomorphizedProgram) -> MirProgram
```

so every builder is created with:

```rust
&program.type_context
```

Update method instance map construction to use `record.substitution.clone()` directly, now `Vec<TypeId>`.

- [ ] **Step 8: Run compile-focused MIR tests**

Run:

```bash
cargo test -p rock-lib mir_builder_records_type_ids_for_return_locals_and_casts
cargo test -p rock-lib mir::identity
cargo test -p rock-lib mir::builder --no-run
```

Expected: PASS compilation and focused test PASS.

- [ ] **Step 9: Run MIR builder suite**

Run:

```bash
cargo test -p rock-lib mir::builder
```

Expected: PASS.

- [ ] **Step 10: Commit**

Run:

```bash
git add lib/src/mir/mod.rs lib/src/mir/identity.rs lib/src/mir/builder
git commit -m "store type ids in mir"
```

Expected: commit succeeds.

---

## Task 6: Move MIR Agreement And Borrowck Type Queries To TypeView

**Files:**
- Modify: `lib/src/mir/agreement.rs`
- Modify: `lib/src/mir/borrowck/**`
- Modify: `lib/src/mir/dataflow/**` if compile errors show MIR local type usage there
- Test: MIR agreement and borrowck suites

- [ ] **Step 1: Write failing borrowck TypeView test**

In `lib/src/mir/borrowck/mod.rs`, add a unit test that creates a `TypeContext`, interns mutable reference and pointer types, and verifies borrowck helper predicates consume TypeIds through `TypeView`:

```rust
#[test]
fn borrowck_type_queries_use_type_view() {
    let mut context = crate::type_context::TypeContext::new();
    let i64_id = context.intern_type(&Type::I64);
    let mut_ref = context.intern_ty(crate::type_context::Ty::Reference {
        mutable: true,
        inner: i64_id,
    });
    let pointer = context.intern_ty(crate::type_context::Ty::Pointer(i64_id));
    let view = crate::type_context::TypeView::new(&context);

    assert!(type_id_is_mut_reference(view, mut_ref));
    assert!(type_id_is_pointer(view, pointer));
    assert!(!type_id_is_pointer(view, mut_ref));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p rock-lib borrowck_type_queries_use_type_view
```

Expected: FAIL with missing helper functions.

- [ ] **Step 3: Add borrowck helper predicates**

In `lib/src/mir/borrowck/mod.rs`, add:

```rust
fn type_id_is_pointer(view: crate::type_context::TypeView<'_>, ty: crate::ids::TypeId) -> bool {
    matches!(view.ty(ty), crate::type_context::Ty::Pointer(_))
}

fn type_id_is_mut_reference(view: crate::type_context::TypeView<'_>, ty: crate::ids::TypeId) -> bool {
    matches!(
        view.ty(ty),
        crate::type_context::Ty::Reference { mutable: true, .. }
    )
}
```

- [ ] **Step 4: Thread TypeView into borrow checker**

Change the borrow checker entrypoint from:

```rust
BorrowChecker::run(&mir_program)
```

to:

```rust
BorrowChecker::run(&mir_program, crate::type_context::TypeView::new(&monomorphized.type_context))
```

Update `BorrowChecker::run` signature and internal structs to store `TypeView<'_>`.

At each structural pattern match like:

```rust
matches!(ty, Type::Pointer(_))
matches!(ty, Type::Reference { mutable: true, .. })
```

use:

```rust
type_id_is_pointer(self.type_view, ty_id)
type_id_is_mut_reference(self.type_view, ty_id)
```

- [ ] **Step 5: Update MIR agreement checks**

Change:

```rust
check_mir_runtime_agreement(&mir_program)
```

to:

```rust
check_mir_runtime_agreement(&mir_program, crate::type_context::TypeView::new(&monomorphized.type_context))
```

In `agreement.rs`, replace structural `Type::Unit` checks with:

```rust
matches!(type_view.ty(local.ty), crate::type_context::Ty::Unit)
```

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p rock-lib borrowck_type_queries_use_type_view
cargo test -p rock-lib mir::agreement
cargo test -p rock-lib mir::borrowck
cargo test -p rock-lib mir::dataflow
```

Expected: PASS.

- [ ] **Step 7: Run behavior filters**

Run:

```bash
cargo test -p rock-lib borrow
cargo test -p rock-lib cast
cargo test -p rock-lib pointer
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add lib/src/mir/agreement.rs lib/src/mir/borrowck lib/src/mir/dataflow lib/src/lib.rs
git commit -m "query mir types through type view"
```

Expected: commit succeeds.

---

## Task 7: Make Codegen Type APIs TypeId-Aware

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/codegen/mir/**`
- Modify: `lib/src/codegen/closures.rs`
- Modify: `lib/src/codegen/expr/**`
- Modify: `lib/src/codegen/stmt.rs`
- Modify: `lib/src/codegen/control_flow.rs`
- Modify: `lib/src/codegen/intrinsics.rs`
- Modify: `lib/src/codegen/operators.rs`
- Test: codegen and integration filters

- [ ] **Step 1: Write failing codegen TypeId API test**

In `lib/src/codegen/types.rs`, add:

```rust
#[test]
fn codegen_lowers_llvm_type_from_type_id() {
    let context = inkwell::context::Context::create();
    let mut type_context = crate::type_context::TypeContext::new();
    let i64_id = type_context.intern_type(&Type::I64);
    let mut codegen = CodeGen::new(&context, "type_id_codegen_test");
    codegen.set_type_context(type_context.clone());

    assert_eq!(codegen.llvm_type_id(i64_id).into_int_type().get_bit_width(), 64);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p rock-lib codegen_lowers_llvm_type_from_type_id
```

Expected: FAIL with missing `set_type_context` and `llvm_type_id`.

- [ ] **Step 3: Add type context storage to CodeGen**

In `CodeGen<'ctx>`, add field:

```rust
type_context: Option<crate::type_context::TypeContext>,
```

Initialize it in `CodeGen::new` with `None`.

Add methods:

```rust
pub fn set_type_context(&mut self, type_context: crate::type_context::TypeContext) {
    self.type_context = Some(type_context);
}

pub(crate) fn type_context(&self) -> &crate::type_context::TypeContext {
    self.type_context
        .as_ref()
        .expect("codegen requires a TypeContext before TypeId lowering")
}

pub(crate) fn type_view(&self) -> crate::type_context::TypeView<'_> {
    crate::type_context::TypeView::new(self.type_context())
}

pub(crate) fn structural_type_for(&self, ty: crate::ids::TypeId) -> Type {
    self.type_context().type_for(ty)
}
```

- [ ] **Step 4: Add TypeId wrappers in `codegen/types.rs`**

Add:

```rust
pub(crate) fn llvm_type_id(&self, ty: crate::ids::TypeId) -> BasicTypeEnum<'ctx> {
    let structural = self.structural_type_for(ty);
    self.llvm_type(&structural)
}

pub(crate) fn callable_code_type_id(
    &self,
    param_types: &[crate::ids::TypeId],
    ret_type: crate::ids::TypeId,
) -> FunctionType<'ctx> {
    let params: Vec<Type> = param_types
        .iter()
        .map(|id| self.structural_type_for(*id))
        .collect();
    let ret = self.structural_type_for(ret_type);
    self.callable_code_type(&params, &ret)
}

pub(crate) fn default_value_id(
    &self,
    ty: crate::ids::TypeId,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let structural = self.structural_type_for(ty);
    self.default_value(&structural)
}
```

These wrappers are temporary compatibility adapters. The final audit must classify remaining `llvm_type(&Type)` calls as product/HIR compatibility or replace them.

- [ ] **Step 5: Thread type context from compile pipeline to codegen**

In `lib/src/lib.rs`, after creating codegen, add:

```rust
codegen.set_type_context(monomorphized.type_context.clone());
```

- [ ] **Step 6: Update MIR codegen modules to use TypeId fields**

In `lib/src/codegen/mir/**`, replace calls on MIR type fields:

```rust
self.llvm_type(&local.ty)
self.default_value(&local.ty)
self.callable_code_type(&params, &ret)
```

with:

```rust
self.llvm_type_id(local.ty)
self.default_value_id(local.ty)
self.callable_code_type_id(&params, ret)
```

When existing code needs structural type matching for a MIR TypeId, use:

```rust
let ty = self.structural_type_for(type_id);
```

and keep that conversion inside codegen helper functions, not at call sites.

- [ ] **Step 7: Update closure metadata types**

Change `MirClosureCodegenMetadata` in `codegen/mod.rs`:

```rust
pub(crate) struct MirClosureCodegenMetadata {
    pub(crate) params: Vec<crate::ids::TypeId>,
    pub(crate) ret: crate::ids::TypeId,
    pub(crate) captures: Vec<crate::ids::TypeId>,
}
```

Update all constructors to use TypeIds from MIR locals/functions.

- [ ] **Step 8: Update codegen semantic maps that use Type as identity**

Change these fields in `CodeGen`:

```rust
variables: Vec<HashMap<String, (PointerValue<'ctx>, TypeId)>>,
callable_signatures_by_symbol: HashMap<String, (Vec<TypeId>, TypeId)>,
struct_info: HashMap<String, Vec<(String, TypeId)>>,
function_value_wrappers: HashMap<(String, TypeId), FunctionValue<'ctx>>,
trait_impls: HashMap<(String, Vec<TypeId>, DefId, Vec<TypeId>), crate::hir::HirImpl>,
```

At HIR/product metadata registration boundaries, intern structural fields before storing:

```rust
let field_ty = self.type_context.as_mut().unwrap().intern_type(&field.ty);
```

If borrowing prevents mutable interning inside registration, pre-intern product/HIR metadata types before populating the maps.

- [ ] **Step 9: Run codegen focused tests**

Run:

```bash
cargo test -p rock-lib codegen_lowers_llvm_type_from_type_id
cargo test -p rock-lib codegen
```

Expected: PASS.

- [ ] **Step 10: Run integration filters**

Run:

```bash
cargo test -p rock-lib generic
cargo test -p rock-lib closure
cargo test -p rock-lib enum
cargo test -p rock-lib array
cargo test -p rock-lib vec
cargo test -p rock-lib stdlib
```

Expected: PASS.

- [ ] **Step 11: Commit**

Run:

```bash
git add lib/src/codegen lib/src/lib.rs
git commit -m "make codegen consume type ids"
```

Expected: commit succeeds.

---

## Task 8: Preserve Product Structural Boundary And Re-Intern Loaded Types

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_system/**` if loaded interfaces need a context handoff
- Test: `lib/src/products.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Update product boundary tests**

In `lib/src/products.rs`, replace the intent of `compiler_products_do_not_serialize_resolved_hir_type_id_sidecar` with:

```rust
#[test]
fn compiler_products_serialize_structural_types_as_explicit_compatibility_boundary() {
    let resolved = resolved_hir_for_products();
    let ret_id = resolved
        .type_id_at(&crate::hir::HirTypeLocation::FunctionReturn {
            function: resolved.program.names.functions_by_name["identity"],
        })
        .expect("resolved HIR should have TypeId sidecar before products");
    assert_eq!(resolved.type_at(ret_id), Type::I64);

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &resolved,
        Vec::new(),
        Vec::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );

    let function = products
        .metadata
        .functions
        .values()
        .find(|function| function.name == "identity")
        .unwrap();
    assert_eq!(function.ret_type, Type::I64);
    assert_eq!(function.params[0].ty, Type::I64);
}
```

- [ ] **Step 2: Add loaded artifact re-interning test**

In `lib/src/crate_artifact/load.rs`, add:

```rust
#[test]
fn load_product_artifact_reinterns_structural_types_for_consumer_context() {
    let (base, _cleanup) = temp_test_dir("reinters_structural_types");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();
    let products = product_with_function("dep", ProductCrateId(0), 0, object_path);
    let artifact_path = base.join("dep.rkca");
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut type_context = crate::type_context::TypeContext::new();
    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path_with_type_context(artifact_path, &mut type_context)
        .unwrap();
    let record = ctx.extern_crate("dep").unwrap();
    let function = record
        .metadata()
        .interface()
        .functions
        .get("dep::answer")
        .unwrap();
    let ret_id = type_context
        .id_for_type(&function.ret_type)
        .expect("loaded structural return type should be interned into consumer context");

    assert_eq!(type_context.type_for(ret_id), function.ret_type);
}
```

- [ ] **Step 3: Run product tests to verify new test failure**

Run:

```bash
cargo test -p rock-lib compiler_products_serialize_structural_types_as_explicit_compatibility_boundary
cargo test -p rock-lib load_product_artifact_reinterns_structural_types_for_consumer_context
```

Expected: first test PASS; second FAIL due missing `load_product_artifact_from_path_with_type_context` API.

- [ ] **Step 4: Add product-loaded type interning helper**

In `crate_artifact/load.rs`, add helpers:

```rust
fn intern_loaded_interface_types(
    interface: &crate::crate_artifact::ArtifactCrateInterface,
    context: &mut crate::type_context::TypeContext,
) {
    let program = crate::hir::HirProgram::from_parts(
        interface
            .functions
            .iter()
            .map(|(name, function)| (name.clone(), function.clone()))
            .collect(),
        interface
            .structs
            .iter()
            .map(|(name, strukt)| (name.clone(), strukt.clone()))
            .collect(),
        interface
            .enums
            .iter()
            .map(|(name, enm)| (name.clone(), enm.clone()))
            .collect(),
        interface
            .traits
            .iter()
            .map(|(name, trait_def)| (name.clone(), trait_def.clone()))
            .collect(),
        interface.impls.clone(),
        interface.externs.clone(),
    );
    let _ = crate::hir::collect_hir_type_ids(&program, context);
}

fn intern_loaded_body_types(
    bodies: &crate::crate_system::ExternCrateBodies,
    context: &mut crate::type_context::TypeContext,
) {
    let program = crate::hir::HirProgram::from_parts(
        bodies
            .generic_functions()
            .iter()
            .map(|(name, function)| (name.clone(), function.clone()))
            .collect(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        bodies
            .traits_with_defaults()
            .iter()
            .map(|(name, trait_def)| (name.clone(), trait_def.clone()))
            .collect(),
        bodies.generic_impls().to_vec(),
        Vec::new(),
    );
    let _ = crate::hir::collect_hir_type_ids(&program, context);
}

fn intern_loaded_extern_crate_types(
    record: &crate::crate_system::ExternCrateRecord,
    context: &mut crate::type_context::TypeContext,
) {
    intern_loaded_interface_types(record.metadata().interface(), context);
    intern_loaded_body_types(record.bodies(), context);
}
```

Call `intern_loaded_extern_crate_types` after `extern_crate_from_products` returns and before `add_extern_crate` stores the record.

- [ ] **Step 5: Add explicit context API**

Add a narrow internal API used by tests and crate context construction:

```rust
pub(crate) fn load_product_artifact_from_path_with_type_context(
    &mut self,
    artifact_path: PathBuf,
    context: &mut crate::type_context::TypeContext,
) -> Result<(), String> {
    let products = CompilerProducts::read_artifact_from_path(&artifact_path)?;
    let remap = ProductIdentityRemap::from_products(self, &products)?;
    let extern_record = extern_crate_from_products(&products, &artifact_path, &remap, self)?;
    intern_loaded_extern_crate_types(&extern_record, context);
    self.add_extern_crate(extern_record)?;

    Ok(())
}
```

Then change `load_product_artifact_from_path` to create a temporary `TypeContext` and delegate to this helper so public CLI behavior stays unchanged:

```rust
pub fn load_product_artifact_from_path(
    &mut self,
    artifact_path: PathBuf,
) -> Result<(), String> {
    let mut context = crate::type_context::TypeContext::new();
    self.load_product_artifact_from_path_with_type_context(artifact_path, &mut context)
}
```

- [ ] **Step 6: Run product and artifact tests**

Run:

```bash
cargo test -p rock-lib product_artifact
cargo test -p rock-lib crate_artifact::load
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add lib/src/products.rs lib/src/crate_artifact/load.rs lib/src/crate_system
git commit -m "preserve product structural type boundary"
```

Expected: commit succeeds.

---

## Task 9: Pipeline Integration And Structural Type Audit Cleanup

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: any mono/MIR/codegen file still failing the audit
- No docs updates yet

- [ ] **Step 1: Run compile pipeline tests**

Run:

```bash
cargo test -p rock-lib compile_with_products
cargo test -p rock-lib test_hello_world -- --exact
cargo test -p rock-lib product_artifact
```

Expected: PASS.

- [ ] **Step 2: Run structural Type audit**

Run:

```bash
rg "Vec<Type>|HashMap<.*Type|: Type|Type\)|&Type|Type::" lib/src/mono lib/src/mir lib/src/codegen --glob '*.rs'
```

Classify every remaining match in implementation notes as one of:

```text
compatibility: structural product/HIR adapter
diagnostic-display: user-facing rendering only
local-construction: parser/lower/inference construction before final TypeId boundary
test-fixture: test-only structural setup
bug: semantic storage still using Type
```

Expected: no `bug` classifications remain.

- [ ] **Step 3: Fix any semantic structural leftovers**

For each `bug` classification, replace the structural storage with `TypeId`. Use these conversions:

```rust
let ty_id = type_context.id_for_type(&ty).expect("finalized type should be interned");
let structural = type_context.type_for(ty_id);
```

Expected: audit re-run has no `bug` classifications.

- [ ] **Step 4: Run focused high-risk suites**

Run:

```bash
cargo test -p rock-lib type_context
cargo test -p rock-lib hir_type_ids
cargo test -p rock-lib mono
cargo test -p rock-lib mir
cargo test -p rock-lib codegen
cargo test -p rock-lib product_artifact
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add lib/src/lib.rs lib/src/mono lib/src/mir lib/src/codegen lib/src/products.rs lib/src/crate_artifact/load.rs lib/src/crate_system
git commit -m "integrate type id phase boundaries"
```

Expected: commit succeeds if there are changes. If no files changed, skip commit.

---

## Task 10: Final Verification, Review, And Documentation

**Files:**
- Modify after review approval: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify after review approval: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-29-full-task-11-typeid-phase-boundaries.md`

- [ ] **Step 1: Run full verification**

Run:

```bash
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Expected: PASS. Record exact unit/integration/doctest counts in this plan under a `Final Verification Notes` section.

- [ ] **Step 2: Run final architecture audit**

Run:

```bash
rg "Vec<Type>|HashMap<.*Type|: Type|Type\)|&Type|Type::" lib/src/mono lib/src/mir lib/src/codegen --glob '*.rs'
```

Expected: remaining matches are compatibility adapters, diagnostic/display, local construction before finalization, or tests. No semantic mono/MIR/codegen storage should remain structural.

- [ ] **Step 3: Request final code review**

Use the `requesting-code-review` skill. Review scope:

```text
Full Roadmap Task 11 TypeId phase-boundary migration.
Verify mono instance identity, MIR type-bearing fields, borrowck/codegen type queries, and product structural compatibility boundary.
```

Expected: reviewer returns APPROVED or findings to fix.

- [ ] **Step 4: Fix review findings before docs**

If review returns findings, apply TDD fixes one finding at a time. Re-run focused tests and full verification before continuing.

Expected: final re-review APPROVED.

- [ ] **Step 5: Update ordered roadmap after approval**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update Task 11 status from partial to complete:

```markdown
**Status:** Complete. Compiler-owned phase boundaries after HIR finalization use context-owned `TypeId` identity across mono instance keys/substitutions, MIR type-bearing runtime forms, borrowck/MIR agreement type queries, and codegen type/layout/ABI APIs. Product artifacts intentionally remain an explicit structural compatibility boundary until a future portable artifact type-table schema is designed.
```

Update the reconciliation table Task 11 row to:

```markdown
| 11. `TypeId` phase boundaries | Complete | `ResolvedHirProgram` owns `TypeContext`/`HirTypeIds`; mono instance identity and substitutions use `TypeId`; MIR locals/returns/casts/callables carry `TypeId`; borrowck/agreement/codegen query type shape through the owning type context; product artifacts convert through an explicit structural compatibility boundary | Future artifact-schema work may add a portable serialized type table, but no remaining compiler-owned mono/MIR/codegen semantic type storage depends on structural `Type` |
```

- [ ] **Step 6: Update master audit checklist after approval**

In `docs/superpowers/plans/master-audit-checklist.md`, move this item from `Still to do` to `Done` in the Type Context section:

```markdown
- [x] Migrated downstream mono, MIR, codegen, and current artifact compatibility consumers from structural `Type` semantic ownership to context-owned `TypeId` where the resolved-HIR sidecar provides stable identity; product artifacts remain structural only at the explicit serialization/loading boundary.
```

Remove the unchecked copy:

```markdown
- [ ] Migrate downstream mono, MIR, codegen, and future artifact-schema consumers from structural `Type` to context-owned `TypeId` where the resolved-HIR sidecar now provides stable identity.
```

- [ ] **Step 7: Run docs diff check**

Run:

```bash
git diff -- docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git diff --check
```

Expected: docs wording does not overclaim product TypeId serialization; whitespace check passes.

- [ ] **Step 8: Commit docs**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-29-full-task-11-typeid-phase-boundaries.md
git commit -m "mark task 11 typeid boundaries complete"
```

Expected: commit succeeds.

- [ ] **Step 9: Final clean status**

Run:

```bash
git status --short
```

Expected: no output.

---

## Final Verification Notes

Append final command outputs here during Task 10:

```text
cargo test -p rock-lib: PASS after final code-review fix (log: /tmp/rock-lib-task10-tests-final.log)
  unit: 1366 passed, 0 failed, 1 ignored
  integration: 277 passed, 0 failed
  parser integration target: 1 passed, 0 failed
  doctests: 1 passed, 0 failed, 1 ignored
cargo fmt --all --check: PASS
git diff --check: PASS
Final architecture audit: PASS; 1599 broad matches in lib/src/mono, lib/src/mir, and lib/src/codegen (log: /tmp/type-audit-task10-final.log); Task 9 classification found 0 bug classifications.
Final code review: APPROVED after fixing failed artifact-load CrateContext staging in commit 8eae89a.
git status --short: PASS; no output after final documentation review fixes.
```

---

## Plan Self-Review

- Spec coverage: Tasks 1-2 cover TypeContext/view and HIR finalization; Tasks 3-4 cover mono TypeId identity/substitution; Tasks 5-6 cover MIR and borrowck/agreement; Task 7 covers codegen; Task 8 covers product structural compatibility; Task 10 covers verification, review, and docs.
- Placeholder scan: no prohibited placeholder markers or unspecified edge handling remain.
- Type consistency: `TypeId`, `TypeContext`, `TypeView`, `Ty`, `InstanceKey.substitution`, `InstanceRecord.substitution`, `MirFunction.ret_type`, `LocalDecl.ty`, `Rvalue::Cast`, and `MirCallable::Method.trait_args` use consistent names across tasks.
