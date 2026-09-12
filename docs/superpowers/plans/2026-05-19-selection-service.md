# Shared Selection Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Roadmap Task 12 by introducing a shared trait/method selection service that lowering uses as the authoritative dispatch selector while mono and codegen consume selected facts first.

**Architecture:** Add `lib/src/selection/` for selection data, matching helpers, and the selector service. Migrate lowering call sites in small slices: concrete methods, trait-bound methods, operators, then index dispatch. Keep mono/codegen compatibility fallback paths in place, but centralize selected-target matching helpers and make selected-target diagnostics clearer.

**Tech Stack:** Rust 2021, existing HIR/inference/lowering/type-service modules, `cargo test -p rock-lib`, `cargo fmt --all`, `git diff --check`.

---

## File Structure

- Create: `lib/src/selection/mod.rs`
  - Registers and re-exports selection APIs.
- Create: `lib/src/selection/types.rs`
  - Owns `SelectionKind`, `ReceiverAdjustment`, `SelectedOrigin`, `SelectedMethod`, `SelectionRequest`, and `SelectionDiagnostic`.
- Create: `lib/src/selection/matching.rs`
  - Owns pure type/name/generic matching helpers currently coupled to `Lowerer` and `Monomorphizer`.
- Create: `lib/src/selection/service.rs`
  - Owns `SelectionService` and selection routines over read-only HIR/resolver maps.
- Modify: `lib/src/lib.rs`
  - Exposes the new `selection` module inside `rock-lib`.
- Modify: `lib/src/lower/types_helpers/helpers.rs`
  - Delegates duplicate matching helpers to `selection::matching` while preserving existing `Lowerer` method names during migration.
- Modify: `lib/src/lower/control_flow/secondary.rs`
  - Replaces method call, method value, trait-bound, and index selection branches with calls into `SelectionService`.
- Modify: `lib/src/lower/expression.rs`
  - Replaces trait-backed operator selection branches with calls into `SelectionService`.
- Modify: `lib/src/mono/methods.rs`
  - Uses shared selected-target matching helpers when consuming `HirMethodCallTarget`.
- Modify: `lib/src/codegen/expr/mod.rs`
  - Uses shared selected-target helpers and improves selected-target diagnostics without removing fallbacks.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Marks Roadmap Task 12 complete for the shared selector service slice.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Updates audit evidence and keeps Task 13 fallback deletion as remaining work.

## Task 1: Add Selection Module Types And Pure Matching Helpers

**Files:**
- Create: `lib/src/selection/mod.rs`
- Create: `lib/src/selection/types.rs`
- Create: `lib/src/selection/matching.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Register the module and write failing selection shape tests**

In `lib/src/lib.rs`, add the module declaration near the existing public compiler modules:

```rust
pub mod selection;
```

Create `lib/src/selection/mod.rs` with this content:

```rust
mod matching;
mod service;
mod types;

pub use matching::{
    generic_substitution_for_owner, impl_matches_method_lookup_type,
    infer_generic_subst_from_types, receiver_arg_types, seed_receiver_substitution_from_impl,
    target_matches_impl, type_names_for_method_lookup, type_pattern_matches,
};
pub use service::SelectionService;
pub use types::{
    ReceiverAdjustment, SelectedMethod, SelectedOrigin, SelectionDiagnostic, SelectionKind,
    SelectionRequest,
};
```

Create `lib/src/selection/types.rs` with only this failing test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::HirMethodCallTarget;
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::types::Type;

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn selection_request_records_dispatch_kind_and_trait_requirement() {
        let request = SelectionRequest {
            kind: SelectionKind::Operator,
            method_name: "+".to_string(),
            required_trait_id: Some(def_id(10)),
            from_index_operator: false,
        };

        assert_eq!(request.kind, SelectionKind::Operator);
        assert_eq!(request.method_name, "+");
        assert_eq!(request.required_trait_id, Some(def_id(10)));
        assert!(!request.from_index_operator);
    }

    #[test]
    fn selection_diagnostic_preserves_selected_target_identity() {
        let target = HirMethodCallTarget {
            impl_id: Some(def_id(20)),
            trait_id: Some(def_id(10)),
            trait_args: vec![Type::I64],
            method_id: def_id(30),
            from_index_operator: false,
        };
        let diagnostic = SelectionDiagnostic::SelectedTargetMissing {
            method_name: "show".to_string(),
            target: target.clone(),
        };

        assert_eq!(
            diagnostic.message(),
            "Selected method target for 'show' could not be resolved by identity"
        );
        assert_eq!(diagnostic.target(), Some(&target));
    }
}
```

Create `lib/src/selection/matching.rs` with only this failing test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{HirImpl, HirImplOwner};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::types::{GenericParamId, Type};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    fn impl_for(owner: &str, type_name: &str) -> HirImpl {
        HirImpl {
            id: def_id(1),
            owner: HirImplOwner::Named(owner.to_string()),
            type_name: type_name.to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(GenericParamId {
                owner: def_id(1),
                index: 0,
            })],
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn selection_receiver_arg_types_cover_nominal_array_slice_and_ref() {
        assert_eq!(
            receiver_arg_types(&Type::Array(Box::new(Type::U8), 4)),
            vec![Type::U8]
        );
        assert_eq!(
            receiver_arg_types(&Type::Reference {
                mutable: false,
                inner: Box::new(Type::Slice(Box::new(Type::I64))),
            }),
            vec![Type::I64]
        );
        assert_eq!(receiver_arg_types(&Type::Str), Vec::<Type>::new());
    }

    #[test]
    fn selection_impl_lookup_handles_reference_and_slice_names() {
        assert!(impl_matches_method_lookup_type(
            &impl_for("&[T]", "Array"),
            "&[I64]"
        ));
        assert!(impl_matches_method_lookup_type(
            &impl_for("Vec", "Vec"),
            "Vec"
        ));
        assert!(!impl_matches_method_lookup_type(
            &impl_for("Vec", "Vec"),
            "Map"
        ));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib selection_`

Expected: FAIL with unresolved types/functions from `lib/src/selection/types.rs`, `lib/src/selection/matching.rs`, and unresolved `service` module.

- [ ] **Step 3: Implement selection types**

Replace the contents of `lib/src/selection/types.rs` with:

```rust
use crate::hir::{HirExpr, HirFunction, HirImpl, HirMethodCallTarget, HirParam};
use crate::ids::DefId;
use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    MethodCall,
    MethodValue,
    Operator,
    Index,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverAdjustment {
    None,
    BuiltinDeref,
    MutToSharedRef,
    TraitDeref,
    ArrayRefToSliceRef,
    ArrayValueToSliceRef,
    ArrayValueToSliceValue,
    Autoderef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedOrigin {
    InherentImpl { impl_id: DefId },
    TraitImpl { impl_id: DefId, trait_id: DefId },
    TraitBound { trait_id: DefId },
    CurrentTrait { trait_id: DefId },
    BuiltinIndex,
    UnresolvedGeneric { trait_id: DefId },
}

#[derive(Debug, Clone)]
pub struct SelectionRequest {
    pub kind: SelectionKind,
    pub method_name: String,
    pub required_trait_id: Option<DefId>,
    pub from_index_operator: bool,
}

#[derive(Debug, Clone)]
pub struct SelectedMethod {
    pub receiver: HirExpr,
    pub function: HirFunction,
    pub impl_def: Option<HirImpl>,
    pub target: Option<HirMethodCallTarget>,
    pub origin: SelectedOrigin,
    pub receiver_adjustment: ReceiverAdjustment,
    pub substituted_params: Vec<HirParam>,
    pub return_type: Type,
    pub builtin_index_output: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionDiagnostic {
    NoImplementation { operation: String, receiver: Type },
    ReceiverMismatch { operation: String, receiver: Type },
    AmbiguousCandidates { operation: String, receiver: Type },
    SelectedTargetMissing {
        method_name: String,
        target: HirMethodCallTarget,
    },
}

impl SelectionDiagnostic {
    pub fn message(&self) -> String {
        match self {
            SelectionDiagnostic::NoImplementation { operation, receiver } => {
                format!("No implementation found for operator '{}' on type {}", operation, receiver)
            }
            SelectionDiagnostic::ReceiverMismatch { operation, receiver } => {
                format!("No receiver adjustment for '{}' on type {}", operation, receiver)
            }
            SelectionDiagnostic::AmbiguousCandidates { operation, receiver } => {
                format!("Ambiguous selection for '{}' on type {}", operation, receiver)
            }
            SelectionDiagnostic::SelectedTargetMissing { method_name, .. } => format!(
                "Selected method target for '{}' could not be resolved by identity",
                method_name
            ),
        }
    }

    pub fn target(&self) -> Option<&HirMethodCallTarget> {
        match self {
            SelectionDiagnostic::SelectedTargetMissing { target, .. } => Some(target),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::HirMethodCallTarget;
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::types::Type;

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn selection_request_records_dispatch_kind_and_trait_requirement() {
        let request = SelectionRequest {
            kind: SelectionKind::Operator,
            method_name: "+".to_string(),
            required_trait_id: Some(def_id(10)),
            from_index_operator: false,
        };

        assert_eq!(request.kind, SelectionKind::Operator);
        assert_eq!(request.method_name, "+");
        assert_eq!(request.required_trait_id, Some(def_id(10)));
        assert!(!request.from_index_operator);
    }

    #[test]
    fn selection_diagnostic_preserves_selected_target_identity() {
        let target = HirMethodCallTarget {
            impl_id: Some(def_id(20)),
            trait_id: Some(def_id(10)),
            trait_args: vec![Type::I64],
            method_id: def_id(30),
            from_index_operator: false,
        };
        let diagnostic = SelectionDiagnostic::SelectedTargetMissing {
            method_name: "show".to_string(),
            target: target.clone(),
        };

        assert_eq!(
            diagnostic.message(),
            "Selected method target for 'show' could not be resolved by identity"
        );
        assert_eq!(diagnostic.target(), Some(&target));
    }
}
```

- [ ] **Step 4: Implement pure matching helpers**

Replace the contents of `lib/src/selection/matching.rs` with:

```rust
use std::collections::HashMap;

use crate::hir::{HirImpl, HirImplOwner, HirMethodCallTarget};
use crate::ids::DefId;
use crate::types::{GenericParamId, Type};

pub fn receiver_arg_types(ty: &Type) -> Vec<Type> {
    match ty {
        Type::Struct { args, .. } | Type::Enum { args, .. } => args.clone(),
        Type::Slice(inner) | Type::Array(inner, _) => vec![inner.as_ref().clone()],
        Type::Reference { inner, .. } => match inner.as_ref() {
            Type::Slice(elem) | Type::Array(elem, _) => vec![elem.as_ref().clone()],
            Type::Str => Vec::new(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

pub fn type_names_for_method_lookup(ty: &Type) -> Vec<String> {
    match ty {
        Type::Slice(_) => vec!["Array".to_string()],
        Type::Array(_, _) => vec![ty.to_string(), "Array".to_string()],
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
            let mut names = vec![ty.to_string()];
            names.extend(type_names_for_method_lookup(inner));
            names
        }
        Type::Reference { inner, .. } => type_names_for_method_lookup(inner),
        _ => type_name_for_method_lookup(ty).into_iter().collect(),
    }
}

pub fn type_name_for_method_lookup(ty: &Type) -> Option<String> {
    match ty {
        Type::Struct { .. } | Type::Enum { .. } => Some(ty.to_string()),
        Type::I64 => Some("I64".to_string()),
        Type::I32 => Some("I32".to_string()),
        Type::I16 => Some("I16".to_string()),
        Type::I8 => Some("I8".to_string()),
        Type::U64 => Some("U64".to_string()),
        Type::U32 => Some("U32".to_string()),
        Type::U16 => Some("U16".to_string()),
        Type::U8 => Some("U8".to_string()),
        Type::F64 => Some("F64".to_string()),
        Type::F32 => Some("F32".to_string()),
        Type::Bool => Some("Bool".to_string()),
        Type::Str => Some("Str".to_string()),
        Type::Slice(_) => Some("Array".to_string()),
        Type::Array(_, _) => Some(ty.to_string()),
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
            Some(ty.to_string())
        }
        Type::Reference { inner, .. } => type_name_for_method_lookup(inner),
        _ => None,
    }
}

fn trait_ref_type_name_matches(candidate: &str, lookup: &str) -> bool {
    if candidate == lookup {
        return true;
    }

    if (candidate == "&Str" && lookup == "Str") || (candidate == "Str" && lookup == "&Str") {
        return true;
    }

    let borrowed_slice =
        |name: &str| name.starts_with("&[") && name.ends_with(']') && !name.contains(';');
    let mut_borrowed_slice =
        |name: &str| name.starts_with("&mut [") && name.ends_with(']') && !name.contains(';');

    (borrowed_slice(candidate) && borrowed_slice(lookup))
        || (mut_borrowed_slice(candidate) && mut_borrowed_slice(lookup))
}

fn impl_owner_name_for_lookup(imp: &HirImpl) -> &str {
    match &imp.owner {
        HirImplOwner::Named(owner) => owner.as_str(),
        HirImplOwner::BuiltinSlice => imp.type_name.as_str(),
    }
}

pub fn impl_matches_method_lookup_type(imp: &HirImpl, lookup_type_name: &str) -> bool {
    trait_ref_type_name_matches(impl_owner_name_for_lookup(imp), lookup_type_name)
        || trait_ref_type_name_matches(&imp.type_name, lookup_type_name)
}

fn type_pattern_matches_after_subst(pattern: &Type, actual: &Type) -> bool {
    match (pattern, actual) {
        (
            Type::Struct {
                id: expected_id,
                args: expected_args,
            },
            Type::Struct {
                id: actual_id,
                args: actual_args,
            },
        )
        | (
            Type::Enum {
                id: expected_id,
                args: expected_args,
            },
            Type::Enum {
                id: actual_id,
                args: actual_args,
            },
        ) => {
            expected_args.len() == actual_args.len()
                && expected_id == actual_id
                && expected_args
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(expected, actual)| type_pattern_matches_after_subst(expected, actual))
        }
        (
            Type::Reference {
                mutable: left_mut,
                inner: left,
            },
            Type::Reference {
                mutable: right_mut,
                inner: right,
            },
        ) => left_mut == right_mut && type_pattern_matches_after_subst(left, right),
        (Type::Slice(left), Type::Slice(right)) | (Type::Pointer(left), Type::Pointer(right)) => {
            type_pattern_matches_after_subst(left, right)
        }
        (Type::Array(left, left_len), Type::Array(right, right_len)) => {
            left_len == right_len && type_pattern_matches_after_subst(left, right)
        }
        _ => pattern == actual,
    }
}

pub fn type_pattern_matches(
    pattern: &Type,
    actual: &Type,
    subst: &mut HashMap<GenericParamId, Type>,
) -> bool {
    let mut next_subst = subst.clone();
    infer_generic_subst_from_types(pattern, actual, &mut next_subst);
    if type_pattern_matches_after_subst(&pattern.substitute_generics(&next_subst), actual) {
        *subst = next_subst;
        true
    } else {
        false
    }
}

pub fn infer_generic_subst_from_types(
    expected: &Type,
    actual: &Type,
    subst: &mut HashMap<GenericParamId, Type>,
) {
    match expected {
        Type::Generic(param) => {
            subst.entry(*param).or_insert_with(|| actual.clone());
        }
        Type::Slice(inner_expected) => {
            if let Type::Slice(inner_actual) = actual {
                infer_generic_subst_from_types(inner_expected, inner_actual, subst);
            }
        }
        Type::Array(inner_expected, expected_len) => {
            if let Type::Array(inner_actual, actual_len) = actual {
                if expected_len == actual_len {
                    infer_generic_subst_from_types(inner_expected, inner_actual, subst);
                }
            }
        }
        Type::Tuple(expected_elems) => {
            if let Type::Tuple(actual_elems) = actual {
                for (expected_elem, actual_elem) in expected_elems.iter().zip(actual_elems.iter()) {
                    infer_generic_subst_from_types(expected_elem, actual_elem, subst);
                }
            }
        }
        Type::Function(expected_args, expected_ret) => {
            if let Type::Function(actual_args, actual_ret) = actual {
                for (expected_arg, actual_arg) in expected_args.iter().zip(actual_args.iter()) {
                    infer_generic_subst_from_types(expected_arg, actual_arg, subst);
                }
                infer_generic_subst_from_types(expected_ret, actual_ret, subst);
            }
        }
        Type::Struct {
            id: expected_id,
            args: expected_generics,
        } => match actual {
            Type::Struct {
                id: actual_id,
                args: actual_generics,
            } if expected_generics.len() == actual_generics.len() && expected_id == actual_id => {
                for (expected_generic, actual_generic) in
                    expected_generics.iter().zip(actual_generics.iter())
                {
                    infer_generic_subst_from_types(expected_generic, actual_generic, subst);
                }
            }
            _ => {}
        },
        Type::Enum {
            id: expected_id,
            args: expected_generics,
        } => match actual {
            Type::Enum {
                id: actual_id,
                args: actual_generics,
            } if expected_generics.len() == actual_generics.len() && expected_id == actual_id => {
                for (expected_generic, actual_generic) in
                    expected_generics.iter().zip(actual_generics.iter())
                {
                    infer_generic_subst_from_types(expected_generic, actual_generic, subst);
                }
            }
            _ => {}
        },
        Type::Reference {
            mutable: expected_mutable,
            inner: expected_inner,
        } => {
            if let Type::Reference {
                mutable: actual_mutable,
                inner: actual_inner,
            } = actual
            {
                if expected_mutable == actual_mutable {
                    infer_generic_subst_from_types(expected_inner, actual_inner, subst);
                }
            }
        }
        Type::Pointer(expected_inner) => {
            if let Type::Pointer(actual_inner) = actual {
                infer_generic_subst_from_types(expected_inner, actual_inner, subst);
            }
        }
        Type::Projection {
            ty: expected_ty,
            trait_id: expected_trait,
            assoc_type: expected_assoc,
            trait_args: expected_args,
        } => {
            if let Type::Projection {
                ty: actual_ty,
                trait_id: actual_trait,
                assoc_type: actual_assoc,
                trait_args: actual_args,
            } = actual
            {
                if expected_trait == actual_trait
                    && expected_assoc == actual_assoc
                    && expected_args.len() == actual_args.len()
                {
                    infer_generic_subst_from_types(expected_ty, actual_ty, subst);
                    for (expected_arg, actual_arg) in expected_args.iter().zip(actual_args.iter()) {
                        infer_generic_subst_from_types(expected_arg, actual_arg, subst);
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn seed_receiver_substitution_from_impl(
    imp: &HirImpl,
    recv_ty: &Type,
    subst: &mut HashMap<GenericParamId, Type>,
) {
    for (index, concrete) in receiver_arg_types(recv_ty)
        .iter()
        .enumerate()
        .take(imp.type_generics.len())
    {
        subst
            .entry(GenericParamId {
                owner: imp.id,
                index: index as u32,
            })
            .or_insert_with(|| concrete.clone());
    }
}

pub fn generic_substitution_for_owner(
    owner: DefId,
    params: &[String],
    args: &[Type],
) -> HashMap<GenericParamId, Type> {
    params
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            args.get(index).cloned().map(|arg| {
                (
                    GenericParamId {
                        owner,
                        index: index as u32,
                    },
                    arg,
                )
            })
        })
        .collect()
}

pub fn target_matches_impl(imp: &HirImpl, target: Option<&HirMethodCallTarget>) -> bool {
    let Some(target) = target else {
        return true;
    };

    if let Some(impl_id) = target.impl_id {
        return imp.id == impl_id;
    }

    if let Some(trait_id) = target.trait_id {
        if imp.trait_id != Some(trait_id) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{HirImpl, HirImplOwner};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::types::{GenericParamId, Type};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    fn impl_for(owner: &str, type_name: &str) -> HirImpl {
        HirImpl {
            id: def_id(1),
            owner: HirImplOwner::Named(owner.to_string()),
            type_name: type_name.to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(GenericParamId {
                owner: def_id(1),
                index: 0,
            })],
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn selection_receiver_arg_types_cover_nominal_array_slice_and_ref() {
        assert_eq!(
            receiver_arg_types(&Type::Array(Box::new(Type::U8), 4)),
            vec![Type::U8]
        );
        assert_eq!(
            receiver_arg_types(&Type::Reference {
                mutable: false,
                inner: Box::new(Type::Slice(Box::new(Type::I64))),
            }),
            vec![Type::I64]
        );
        assert_eq!(receiver_arg_types(&Type::Str), Vec::<Type>::new());
    }

    #[test]
    fn selection_impl_lookup_handles_reference_and_slice_names() {
        assert!(impl_matches_method_lookup_type(
            &impl_for("&[T]", "Array"),
            "&[I64]"
        ));
        assert!(impl_matches_method_lookup_type(
            &impl_for("Vec", "Vec"),
            "Vec"
        ));
        assert!(!impl_matches_method_lookup_type(
            &impl_for("Vec", "Vec"),
            "Map"
        ));
    }
}
```

Create `lib/src/selection/service.rs` with this temporary content so the module compiles before Task 2:

```rust
#[derive(Debug, Default)]
pub struct SelectionService;
```

- [ ] **Step 5: Run selection shape tests**

Run: `cargo test -p rock-lib selection_`

Expected: PASS for the new selection type and matching helper tests.

- [ ] **Step 6: Commit selection scaffolding**

```bash
git add lib/src/lib.rs lib/src/selection/mod.rs lib/src/selection/types.rs lib/src/selection/matching.rs lib/src/selection/service.rs
git commit -m "add selection service scaffolding"
```

## Task 2: Delegate Existing Lowerer Matching Helpers To Selection Helpers

**Files:**
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/selection/matching.rs`

- [ ] **Step 1: Add failing wrapper-equivalence tests**

In the existing `#[cfg(test)] mod tests` in `lib/src/lower/types_helpers/helpers.rs`, add this test:

```rust
    #[test]
    fn lowerer_matching_wrappers_match_selection_helpers() {
        let impl_id = DefId::new(CrateId(0), LocalDefId(80));
        let generic = GenericParamId {
            owner: impl_id,
            index: 0,
        };
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Vec".to_string()),
            type_name: "Vec".to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(generic)],
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::new(),
        };
        let recv_ty = Type::Struct {
            id: DefId::new(CrateId(0), LocalDefId(81)),
            args: vec![Type::I64],
        };
        let mut lowerer_subst = HashMap::new();
        let mut selection_subst = HashMap::new();

        Lowerer::seed_receiver_substitution_from_impl(&imp, &recv_ty, &mut lowerer_subst);
        crate::selection::seed_receiver_substitution_from_impl(&imp, &recv_ty, &mut selection_subst);

        assert_eq!(lowerer_subst, selection_subst);
        assert_eq!(
            Lowerer::impl_matches_method_lookup_type(&imp, "Vec"),
            crate::selection::impl_matches_method_lookup_type(&imp, "Vec")
        );
        assert!(Lowerer::type_pattern_matches(
            &Type::Generic(generic),
            &Type::I64,
            &mut HashMap::new()
        ));
    }
```

- [ ] **Step 2: Run wrapper-equivalence test to verify it fails or exposes duplication**

Run: `cargo test -p rock-lib lowerer_matching_wrappers_match_selection_helpers`

Expected before delegation: PASS or compile success with duplicate implementations still present. Treat this as the characterization test that must remain PASS after delegation.

- [ ] **Step 3: Replace `Lowerer` helper bodies with selection helper calls**

In `lib/src/lower/types_helpers/helpers.rs`, update these functions so they delegate to `crate::selection` while keeping their public-in-crate names stable:

```rust
    fn type_pattern_matches_after_subst(pattern: &Type, actual: &Type) -> bool {
        crate::selection::type_pattern_matches(pattern, actual, &mut HashMap::new())
    }

    pub(crate) fn type_pattern_matches(
        pattern: &Type,
        actual: &Type,
        subst: &mut HashMap<GenericParamId, Type>,
    ) -> bool {
        crate::selection::type_pattern_matches(pattern, actual, subst)
    }

    pub(crate) fn impl_matches_method_lookup_type(
        imp: &crate::hir::HirImpl,
        lookup_type_name: &str,
    ) -> bool {
        crate::selection::impl_matches_method_lookup_type(imp, lookup_type_name)
    }

    pub(crate) fn seed_receiver_substitution_from_impl(
        imp: &crate::hir::HirImpl,
        recv_ty: &Type,
        subst: &mut HashMap<GenericParamId, Type>,
    ) {
        crate::selection::seed_receiver_substitution_from_impl(imp, recv_ty, subst);
    }

    pub(crate) fn infer_generic_subst_from_types(
        expected: &Type,
        actual: &Type,
        subst: &mut HashMap<GenericParamId, Type>,
    ) {
        crate::selection::infer_generic_subst_from_types(expected, actual, subst);
    }
```

Keep `get_type_name_for_method_lookup_in_context` and `get_type_names_for_method_lookup_in_context` on `Lowerer` for now because they need resolver tables. Task 3 moves resolver-aware lookup into `SelectionService`.

- [ ] **Step 4: Run matching tests and affected lowerer tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_
cargo test -p rock-lib lowerer_matching_wrappers_match_selection_helpers
cargo test -p rock-lib type_pattern_matches
```

Expected: PASS for each command.

- [ ] **Step 5: Commit matching helper delegation**

```bash
git add lib/src/lower/types_helpers/helpers.rs lib/src/selection/matching.rs
git commit -m "share selection matching helpers"
```

## Task 3: Implement Concrete Method Selection Service

**Files:**
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/selection/mod.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`

- [ ] **Step 1: Add failing service test for concrete method selection**

Replace the temporary contents of `lib/src/selection/service.rs` with this test-first skeleton:

```rust
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::SelfReceiverMode;
    use crate::collect::resolver::ResolverTables;
    use crate::hir::{HirBlock, HirExpr, HirExprKind, HirFunction, HirImpl, HirImplOwner, HirParam};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::lexer::Span;
    use crate::types::Type;

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    fn method(id: DefId, owner: DefId) -> HirFunction {
        HirFunction {
            id,
            name: "value".to_string(),
            qualified_name: None,
            generic_params: Vec::new(),
            generic_param_ids: Vec::new(),
            generic_bounds: HashMap::new(),
            params: vec![HirParam {
                name: "self".to_string(),
                ty: Type::Struct {
                    id: owner,
                    args: Vec::new(),
                },
                mutable: false,
                is_ref: false,
            }],
            ret_type: Type::I64,
            body: HirBlock {
                stmts: Vec::new(),
                ty: Type::I64,
            },
            is_curried: false,
            is_method: true,
            self_receiver: Some(SelfReceiverMode::Shared),
            is_unsafe: false,
        }
    }

    #[test]
    fn selection_service_selects_concrete_impl_method_by_identity() {
        let owner = def_id(10);
        let impl_id = def_id(20);
        let method_id = def_id(21);
        let method = method(method_id, owner);
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Box".to_string()),
            type_name: "Box".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([("value".to_string(), method)]),
        };
        let service = SelectionService::new(
            HashMap::new(),
            &[imp],
            HashMap::new(),
            &ResolverTables::default(),
            HashMap::new(),
            None,
            HashMap::new(),
        );
        let receiver = HirExpr {
            kind: HirExprKind::Var("box".to_string()),
            ty: Type::Struct {
                id: owner,
                args: Vec::new(),
            },
            span: Span::default(),
        };

        let selected = service
            .select_concrete_method(&[receiver], "value", |ty| ty.clone())
            .expect("expected selected method");

        assert_eq!(selected.target.as_ref().and_then(|target| target.impl_id), Some(impl_id));
        assert_eq!(selected.target.as_ref().map(|target| target.method_id), Some(method_id));
        assert_eq!(selected.return_type, Type::I64);
    }
}
```

- [ ] **Step 2: Run concrete selection service test to verify it fails**

Run: `cargo test -p rock-lib selection_service_selects_concrete_impl_method_by_identity`

Expected: FAIL with unresolved `SelectionService::new` and `select_concrete_method`.

- [ ] **Step 3: Implement `SelectionService` resolver-aware context and concrete method selection**

Insert this implementation above the test module in `lib/src/selection/service.rs`:

```rust
use std::collections::HashMap;

use crate::collect::resolver::ResolverTables;
use crate::hir::{
    HirExpr, HirFunction, HirGenericBounds, HirImpl, HirMethodCallTarget, HirTrait,
};
use crate::ids::DefId;
use crate::selection::matching::{
    impl_matches_method_lookup_type, receiver_arg_types, seed_receiver_substitution_from_impl,
    type_names_for_method_lookup, type_pattern_matches,
};
use crate::selection::types::{ReceiverAdjustment, SelectedMethod, SelectedOrigin};
use crate::types::{GenericParamId, Type};

pub struct SelectionService<'a> {
    traits: &'a HashMap<String, HirTrait>,
    impls: &'a [HirImpl],
    methods: &'a HashMap<(String, String), HirFunction>,
    resolver: &'a ResolverTables,
    dependency_resolvers: &'a HashMap<String, ResolverTables>,
    current_trait: Option<&'a str>,
    current_impl_bounds: &'a HirGenericBounds,
}

impl<'a> SelectionService<'a> {
    pub fn new(
        traits: &'a HashMap<String, HirTrait>,
        impls: &'a [HirImpl],
        methods: &'a HashMap<(String, String), HirFunction>,
        resolver: &'a ResolverTables,
        dependency_resolvers: &'a HashMap<String, ResolverTables>,
        current_trait: Option<&'a str>,
        current_impl_bounds: &'a HirGenericBounds,
    ) -> Self {
        Self {
            traits,
            impls,
            methods,
            resolver,
            dependency_resolvers,
            current_trait,
            current_impl_bounds,
        }
    }

    pub fn type_name_for_method_lookup_in_context(&self, ty: &Type) -> Option<String> {
        match ty {
            Type::Struct { id, .. } | Type::Enum { id, .. } => self
                .resolver
                .item_names_by_id
                .get(id)
                .cloned()
                .or_else(|| {
                    self.dependency_resolvers
                        .values()
                        .find_map(|resolver| resolver.item_names_by_id.get(id).cloned())
                })
                .or_else(|| crate::selection::matching::type_name_for_method_lookup(ty)),
            Type::Reference { inner, .. }
                if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) =>
            {
                Some(ty.to_string())
            }
            Type::Reference { inner, .. } => self.type_name_for_method_lookup_in_context(inner),
            _ => crate::selection::matching::type_name_for_method_lookup(ty),
        }
    }

    pub fn type_names_for_method_lookup_in_context(&self, ty: &Type) -> Vec<String> {
        match ty {
            Type::Struct { .. } | Type::Enum { .. } => {
                let mut names: Vec<_> = self
                    .type_name_for_method_lookup_in_context(ty)
                    .into_iter()
                    .collect();
                if let Some(short_name) = names
                    .first()
                    .and_then(|name| name.rsplit("::").next())
                    .filter(|short_name| Some(*short_name) != names.first().map(String::as_str))
                {
                    names.push(short_name.to_string());
                }
                names
            }
            Type::Slice(_) | Type::Array(_, _) => type_names_for_method_lookup(ty),
            Type::Reference { inner, .. }
                if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) =>
            {
                let mut names = vec![ty.to_string()];
                names.extend(self.type_names_for_method_lookup_in_context(inner));
                names
            }
            Type::Reference { inner, .. } => self.type_names_for_method_lookup_in_context(inner),
            _ => self
                .type_name_for_method_lookup_in_context(ty)
                .into_iter()
                .collect(),
        }
    }

    pub fn select_concrete_method<F>(
        &self,
        receiver_candidates: &[HirExpr],
        method_name: &str,
        mut normalize: F,
    ) -> Option<SelectedMethod>
    where
        F: FnMut(&Type) -> Type,
    {
        receiver_candidates.iter().find_map(|candidate| {
            let candidate_ty = normalize(&candidate.ty);
            let receiver_args = receiver_arg_types(&candidate_ty);
            let method = self
                .type_names_for_method_lookup_in_context(&candidate_ty)
                .iter()
                .find_map(|type_name| {
                    self.impls
                        .iter()
                        .find(|imp| {
                            let mut subst = HashMap::new();
                            impl_matches_method_lookup_type(imp, type_name)
                                && imp.methods.contains_key(method_name)
                                && imp.receiver_arg_types.len() == receiver_args.len()
                                && imp
                                    .receiver_arg_types
                                    .iter()
                                    .zip(receiver_args.iter())
                                    .all(|(expected, actual)| {
                                        crate::selection::infer_generic_subst_from_types(
                                            expected,
                                            actual,
                                            &mut subst,
                                        );
                                        type_pattern_matches(expected, actual, &mut subst)
                                    })
                        })
                        .and_then(|imp| self.instantiate_impl_method(imp, candidate, &candidate_ty, method_name))
                })?;

            Some(method)
        })
    }

    fn instantiate_impl_method(
        &self,
        imp: &HirImpl,
        receiver: &HirExpr,
        receiver_ty: &Type,
        method_name: &str,
    ) -> Option<SelectedMethod> {
        let mut method_func = imp.methods.get(method_name).cloned()?;
        let mut subst = HashMap::new();
        seed_receiver_substitution_from_impl(imp, receiver_ty, &mut subst);
        if method_func.is_method {
            if let Some(self_param) = method_func.params.first() {
                let expected_self = self_param.ty.substitute_generics(&subst);
                if !matches!(expected_self, Type::Generic(_))
                    && !type_pattern_matches(&expected_self, receiver_ty, &mut subst)
                {
                    return None;
                }
            }
        }

        for param in &mut method_func.params {
            param.ty = param.ty.substitute_generics(&subst);
        }
        if method_func.is_method {
            if let Some(self_param) = method_func.params.first_mut() {
                self_param.ty = receiver_ty.clone();
            }
        }
        method_func.ret_type = method_func.ret_type.substitute_generics(&subst);
        let target = HirMethodCallTarget {
            impl_id: Some(imp.id),
            trait_id: imp.trait_id,
            trait_args: imp
                .trait_arg_types
                .iter()
                .map(|arg| arg.substitute_generics(&subst))
                .collect(),
            method_id: method_func.id,
            from_index_operator: false,
        };
        let origin = match imp.trait_id {
            Some(trait_id) => SelectedOrigin::TraitImpl {
                impl_id: imp.id,
                trait_id,
            },
            None => SelectedOrigin::InherentImpl { impl_id: imp.id },
        };
        let param_start = if method_func.is_method { 1 } else { 0 };

        Some(SelectedMethod {
            receiver: receiver.clone(),
            substituted_params: method_func.params[param_start..].to_vec(),
            return_type: method_func.ret_type.clone(),
            function: method_func,
            impl_def: Some(imp.clone()),
            target: Some(target),
            origin,
            receiver_adjustment: ReceiverAdjustment::None,
            builtin_index_output: None,
        })
    }

    pub fn trait_by_id(&self, id: DefId) -> Option<&HirTrait> {
        self.traits.values().find(|trait_def| trait_def.id == id)
    }

    pub fn trait_def_for_member(&self, trait_id: DefId, method_name: &str) -> Option<&HirTrait> {
        self.traits
            .values()
            .filter(|trait_def| trait_def.id == trait_id)
            .max_by_key(|trait_def| Self::trait_member_id_score(trait_def, method_name))
    }

    pub fn trait_member_id(&self, trait_id: DefId, method_name: &str) -> Option<DefId> {
        let trait_def = self.trait_def_for_member(trait_id, method_name)?;
        trait_def
            .methods
            .get(method_name)
            .map(|method| method.id)
            .or_else(|| trait_def.signatures.get(method_name).map(|sig| sig.id))
    }

    fn trait_member_id_score(trait_def: &HirTrait, method_name: &str) -> usize {
        let method_score = trait_def
            .methods
            .get(method_name)
            .map(|method| Self::def_id_score(method.id))
            .unwrap_or(0);
        let signature_score = trait_def
            .signatures
            .get(method_name)
            .map(|sig| Self::def_id_score(sig.id))
            .unwrap_or(0);

        method_score.max(signature_score)
    }

    fn def_id_score(id: DefId) -> usize {
        if id.crate_id.0 == u32::MAX {
            1
        } else {
            2
        }
    }
}
```

Keep the test module from Step 1 below this implementation.

- [ ] **Step 4: Add a `Lowerer` adapter for constructing the service**

In `lib/src/lower/control_flow/secondary.rs`, add this helper inside `impl Lowerer` near `concrete_method_candidate`:

```rust
    fn selection_service(&self) -> crate::selection::SelectionService<'_> {
        crate::selection::SelectionService::new(
            &self.traits,
            &self.impls,
            &self.methods,
            &self.resolver,
            &self.dependency_resolvers,
            self.current_trait.as_deref(),
            &self.current_impl_bounds,
        )
    }
```

Then replace the concrete impl part of `concrete_method_candidate` with a delegation that preserves the old return type:

```rust
        let service = self.selection_service();
        let concrete = service.select_concrete_method(
            &self.receiver_adjustment_candidates(recv),
            method_name,
            |ty| self.resolve_projection_type(&self.engine.resolve(ty)),
        );
        if let Some(selected) = concrete {
            return Some((
                selected.receiver,
                selected.function,
                selected.impl_def,
                selected.target,
            ));
        }
```

Leave the current-trait and trait-bound fallback branches in `concrete_method_candidate` for Task 4.

- [ ] **Step 5: Run concrete selection and method-target tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_service_selects_concrete_impl_method_by_identity
cargo test -p rock-lib inherent_impl_method_call_carries_exact_target
cargo test -p rock-lib concrete_receiver_does_not_use_name_only_method_map_without_impl_identity
```

Expected: PASS for each command.

- [ ] **Step 6: Commit concrete method selector**

```bash
git add lib/src/selection/service.rs lib/src/selection/mod.rs lib/src/lower/control_flow/secondary.rs
git commit -m "select concrete methods through shared service"
```

## Task 4: Move Trait-Bound And Current-Trait Method Selection Into The Service

**Files:**
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`

- [ ] **Step 1: Add failing service tests for trait-bound selection**

In `lib/src/selection/service.rs` test module, add tests equivalent to the existing lowerer behavior:

```rust
    #[test]
    fn selection_service_selects_type_var_bound_signature_by_trait_identity() {
        let trait_id = def_id(30);
        let signature_id = def_id(31);
        let trait_def = crate::hir::HirTrait {
            id: trait_id,
            name: "Readable".to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods: HashMap::new(),
            signatures: HashMap::from([(
                "read".to_string(),
                crate::hir::HirFunctionSig {
                    id: signature_id,
                    name: "read".to_string(),
                    generic_params: Vec::new(),
                    generic_param_ids: Vec::new(),
                    params: Vec::new(),
                    ret: Type::I64,
                    generic_bounds: HashMap::new(),
                    self_receiver: Some(SelfReceiverMode::Shared),
                },
            )]),
        };
        let service = SelectionService::new(
            &HashMap::from([("Readable".to_string(), trait_def)]),
            &[],
            &HashMap::new(),
            &ResolverTables::default(),
            &HashMap::new(),
            None,
            &HashMap::new(),
        );
        let receiver = HirExpr {
            kind: HirExprKind::Var("receiver".to_string()),
            ty: Type::I64,
            span: Span::default(),
        };
        let bounds = vec![crate::types::TraitBound {
            trait_id,
            type_args: Vec::new(),
        }];

        let selected = service
            .select_bound_method(receiver, &bounds, "read", Type::I64)
            .expect("expected trait-bound signature selection");

        assert_eq!(selected.target.as_ref().and_then(|target| target.trait_id), Some(trait_id));
        assert_eq!(selected.target.as_ref().map(|target| target.method_id), Some(signature_id));
        assert_eq!(selected.return_type, Type::I64);
    }
```

- [ ] **Step 2: Run trait-bound service test to verify it fails**

Run: `cargo test -p rock-lib selection_service_selects_type_var_bound_signature_by_trait_identity`

Expected: FAIL with unresolved `select_bound_method`.

- [ ] **Step 3: Implement bound method/signature selection in the service**

In `lib/src/selection/service.rs`, add this method to `impl SelectionService<'_>`:

```rust
    pub fn select_bound_method(
        &self,
        receiver: HirExpr,
        bounds: &[crate::types::TraitBound],
        method_name: &str,
        self_param_ty: Type,
    ) -> Option<SelectedMethod> {
        let mut candidates = Vec::new();

        for bound in bounds {
            let trait_def = self.trait_def_for_member(bound.trait_id, method_name)?.clone();
            let trait_subst = crate::selection::generic_substitution_for_owner(
                trait_def.id,
                &trait_def.generic_params,
                &bound.type_args,
            );

            if let Some(method_func) = trait_def.methods.get(method_name) {
                let mut method_func = method_func.clone();
                let method_id = method_func.id;
                for param in &mut method_func.params {
                    param.ty = param.ty.substitute_generics(&trait_subst);
                }
                method_func.ret_type = method_func.ret_type.substitute_generics(&trait_subst);
                let target = HirMethodCallTarget {
                    impl_id: None,
                    trait_id: Some(trait_def.id),
                    trait_args: bound.type_args.clone(),
                    method_id,
                    from_index_operator: false,
                };
                candidates.push(SelectedMethod {
                    receiver: receiver.clone(),
                    substituted_params: method_func.params
                        [usize::from(method_func.is_method)..]
                        .to_vec(),
                    return_type: method_func.ret_type.clone(),
                    function: method_func,
                    impl_def: None,
                    target: Some(target),
                    origin: SelectedOrigin::TraitBound {
                        trait_id: trait_def.id,
                    },
                    receiver_adjustment: ReceiverAdjustment::None,
                    builtin_index_output: None,
                });
            }

            if let Some(sig) = trait_def.signatures.get(method_name) {
                let sig_params: Vec<_> = sig
                    .params
                    .iter()
                    .map(|ty| ty.substitute_generics(&trait_subst))
                    .collect();
                let sig_ret = sig.ret.substitute_generics(&trait_subst);
                let mut params = Vec::new();
                if let Some(self_receiver) = sig.self_receiver {
                    params.push(crate::hir::HirParam {
                        name: "self".to_string(),
                        ty: self_param_ty.clone(),
                        mutable: matches!(self_receiver, crate::ast::SelfReceiverMode::Mut),
                        is_ref: false,
                    });
                }
                params.extend(sig_params.iter().enumerate().map(|(i, ty)| crate::hir::HirParam {
                    name: format!("arg{}", i),
                    ty: ty.clone(),
                    mutable: false,
                    is_ref: false,
                }));
                let method_func = crate::hir::HirFunction {
                    id: sig.id,
                    name: method_name.to_string(),
                    qualified_name: None,
                    generic_params: Vec::new(),
                    generic_param_ids: Vec::new(),
                    generic_bounds: HashMap::new(),
                    params,
                    ret_type: sig_ret.clone(),
                    body: crate::hir::HirBlock {
                        stmts: Vec::new(),
                        ty: sig_ret.clone(),
                    },
                    is_curried: false,
                    is_method: sig.self_receiver.is_some(),
                    self_receiver: sig.self_receiver,
                    is_unsafe: false,
                };
                let target = HirMethodCallTarget {
                    impl_id: None,
                    trait_id: Some(trait_def.id),
                    trait_args: bound.type_args.clone(),
                    method_id: sig.id,
                    from_index_operator: false,
                };
                candidates.push(SelectedMethod {
                    receiver: receiver.clone(),
                    substituted_params: method_func.params
                        [usize::from(method_func.is_method)..]
                        .to_vec(),
                    return_type: sig_ret,
                    function: method_func,
                    impl_def: None,
                    target: Some(target),
                    origin: SelectedOrigin::TraitBound {
                        trait_id: trait_def.id,
                    },
                    receiver_adjustment: ReceiverAdjustment::None,
                    builtin_index_output: None,
                });
            }
        }

        candidates.into_iter().next()
    }
```

- [ ] **Step 4: Replace duplicated type-var and generic-bound selection branches**

In `lib/src/lower/control_flow/secondary.rs`, replace the duplicated `Type::TypeVar` and `Type::Generic` lookup blocks in both `Arguments` and `Dot` handling with calls to the service. Use this shape in both places:

```rust
                        if let Type::TypeVar(var_id) = &recv_ty {
                            let bounds = self.engine.get_bounds(*var_id);
                            if let Some(selected) = self.selection_service().select_bound_method(
                                (**recv).clone(),
                                &bounds,
                                method_name,
                                recv_ty.clone(),
                            ) {
                                return Some((
                                    selected.receiver,
                                    selected.function,
                                    selected.impl_def,
                                    selected.target,
                                ));
                            }
                        }
```

For `Type::Generic(gen_param)`, pass the bounds from `self.current_impl_bounds.get(gen_param).cloned().unwrap_or_default()` and the generic receiver expression.

Preserve the receiver-is-call preference by adding this small helper in `SelectionService` if needed:

```rust
    pub fn prefer_non_ref_receiver_candidate(
        candidates: Vec<SelectedMethod>,
        receiver_is_call: bool,
    ) -> Option<SelectedMethod> {
        if receiver_is_call {
            if let Some(candidate) = candidates.iter().find(|candidate| {
                candidate.function.params.first().map_or(true, |param| {
                    !matches!(param.ty, Type::Reference { .. })
                })
            }) {
                return Some(candidate.clone());
            }
        }

        candidates.into_iter().next()
    }
```

- [ ] **Step 5: Run trait-bound and current-trait tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_service_selects_type_var_bound_signature_by_trait_identity
cargo test -p rock-lib type_var_method_call_uses_trait_bound_target_before_name_candidates
cargo test -p rock-lib type_var_signature_call_uses_signature_id_target
cargo test -p rock-lib current_trait_self_signature_call_uses_signature_return_type
cargo test -p rock-lib type_var_method_value_uses_trait_bound_target_before_name_candidates
```

Expected: PASS for each command.

- [ ] **Step 6: Commit trait-bound selector migration**

```bash
git add lib/src/selection/service.rs lib/src/lower/control_flow/secondary.rs
git commit -m "select trait bound methods through shared service"
```

## Task 5: Route Trait-Backed Operator Selection Through The Service

**Files:**
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/expression.rs`

- [ ] **Step 1: Add failing operator selection service test**

In `lib/src/selection/service.rs` test module, add:

```rust
    #[test]
    fn selection_service_selects_required_trait_operator_impl() {
        let owner = def_id(40);
        let trait_id = def_id(41);
        let impl_id = def_id(42);
        let method_id = def_id(43);
        let mut operator = method(method_id, owner);
        operator.name = "+".to_string();
        operator.params.push(HirParam {
            name: "rhs".to_string(),
            ty: Type::Struct {
                id: owner,
                args: Vec::new(),
            },
            mutable: false,
            is_ref: false,
        });
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Number".to_string()),
            type_name: "Number".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: Some("Num".to_string()),
            trait_id: Some(trait_id),
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([("+".to_string(), operator)]),
        };
        let traits = HashMap::from([(
            "Num".to_string(),
            crate::hir::HirTrait {
                id: trait_id,
                name: "Num".to_string(),
                generic_params: Vec::new(),
                associated_types: Vec::new(),
                methods: HashMap::new(),
                signatures: HashMap::new(),
            },
        )]);
        let service = SelectionService::new(
            &traits,
            &[imp],
            &HashMap::new(),
            &ResolverTables::default(),
            &HashMap::new(),
            None,
            &HashMap::new(),
        );
        let receiver = HirExpr {
            kind: HirExprKind::Var("lhs".to_string()),
            ty: Type::Struct {
                id: owner,
                args: Vec::new(),
            },
            span: Span::default(),
        };

        let selected = service
            .select_required_trait_method(&[receiver], "+", trait_id, |ty| ty.clone())
            .expect("expected operator impl selection");

        assert_eq!(selected.target.as_ref().and_then(|target| target.impl_id), Some(impl_id));
        assert_eq!(selected.target.as_ref().and_then(|target| target.trait_id), Some(trait_id));
        assert_eq!(selected.target.as_ref().map(|target| target.method_id), Some(method_id));
    }
```

- [ ] **Step 2: Run operator service test to verify it fails**

Run: `cargo test -p rock-lib selection_service_selects_required_trait_operator_impl`

Expected: FAIL with unresolved `select_required_trait_method`.

- [ ] **Step 3: Implement required-trait method selection**

In `lib/src/selection/service.rs`, add this method:

```rust
    pub fn select_required_trait_method<F>(
        &self,
        receiver_candidates: &[HirExpr],
        method_name: &str,
        trait_id: DefId,
        normalize: F,
    ) -> Option<SelectedMethod>
    where
        F: FnMut(&Type) -> Type,
    {
        self.select_concrete_method(receiver_candidates, method_name, normalize)
            .filter(|selected| {
                selected
                    .target
                    .as_ref()
                    .is_some_and(|target| target.trait_id == Some(trait_id))
            })
    }
```

If this is too strict because `select_concrete_method` finds inherent methods first, split the concrete method loop into an internal `select_impl_method_with_filter` helper and require `imp.trait_id == Some(trait_id)` before instantiating a method.

- [ ] **Step 4: Replace trait-backed binary operator selection in lowering**

In `lib/src/lower/expression.rs`, replace the `required_trait_id.and_then(|trait_id| self.find_matching_trait_impl_by_id(...))` branch for builtin trait-backed binary operators with:

```rust
            let found_method = if let Some(trait_id) = required_trait_id {
                self.selection_service().select_required_trait_method(
                    std::slice::from_ref(&left),
                    &method_name,
                    trait_id,
                    |ty| self.resolve_projection_type(&self.engine.resolve(ty)),
                )
            } else {
                None
            };
```

Then convert the `SelectedMethod` into the existing `HirExprKind::MethodCall` shape:

```rust
            if let Some(selected) = found_method {
                let subst = self.infer_method_substitution(&resolved_ty, &selected.function, &[right.clone()]);
                if let Some(param) = selected.substituted_params.first() {
                    let substituted_ty = if subst.is_empty() {
                        param.ty.clone()
                    } else {
                        param.ty.substitute_generics(&subst)
                    };
                    let _ = self.engine.unify(&substituted_ty, &right.ty);
                }
                result_ty = if subst.is_empty() {
                    selected.return_type
                } else {
                    selected.return_type.substitute_generics(&subst)
                };
                result_ty = self.resolve_projection_type(&result_ty);

                return HirExpr {
                    ty: result_ty,
                    kind: HirExprKind::MethodCall(
                        Box::new(left),
                        method_name,
                        vec![right],
                        selected.function.self_receiver,
                        selected.target,
                    ),
                    span: self.current_span.clone().unwrap_or_default(),
                };
            }
```

Keep custom-operator function fallback unchanged.

- [ ] **Step 5: Run operator regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_service_selects_required_trait_operator_impl
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact
cargo test -p rock-lib --test integration test_unary_neg_dispatches_through_stdlib_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_unary_not_generic_dispatches_through_stdlib_trait_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 6: Commit operator selector migration**

```bash
git add lib/src/selection/service.rs lib/src/lower/expression.rs
git commit -m "select trait backed operators through shared service"
```

## Task 6: Route Index Dispatch Selection Through The Service

**Files:**
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`

- [ ] **Step 1: Add failing index selection service test**

In `lib/src/selection/service.rs` test module, add:

```rust
    #[test]
    fn selection_service_prefers_user_index_impl_over_builtin_output() {
        let owner = def_id(50);
        let trait_id = def_id(51);
        let assoc_id = crate::ids::AssocTypeId(0);
        let impl_id = def_id(52);
        let method_id = def_id(53);
        let mut index_method = method(method_id, owner);
        index_method.name = "index".to_string();
        index_method.ret_type = Type::Reference {
            mutable: false,
            inner: Box::new(Type::I64),
        };
        index_method.params.push(HirParam {
            name: "index".to_string(),
            ty: Type::I64,
            mutable: false,
            is_ref: false,
        });
        let traits = HashMap::from([(
            "Index".to_string(),
            crate::hir::HirTrait {
                id: trait_id,
                name: "Index".to_string(),
                generic_params: vec!["Idx".to_string()],
                associated_types: vec![crate::hir::HirAssociatedTypeDecl {
                    id: assoc_id,
                    name: "Output".to_string(),
                }],
                methods: HashMap::new(),
                signatures: HashMap::new(),
            },
        )]);
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Bag".to_string()),
            type_name: "Bag".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: Some("Index".to_string()),
            trait_id: Some(trait_id),
            trait_generics: vec!["Idx".to_string()],
            trait_arg_types: vec![Type::I64],
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([("index".to_string(), index_method)]),
        };
        let service = SelectionService::new(
            &traits,
            &[imp],
            &HashMap::new(),
            &ResolverTables::default(),
            &HashMap::new(),
            None,
            &HashMap::new(),
        );
        let receiver = HirExpr {
            kind: HirExprKind::Var("bag".to_string()),
            ty: Type::Struct {
                id: owner,
                args: Vec::new(),
            },
            span: Span::default(),
        };

        let selected = service
            .select_index_method(&[receiver], trait_id, &Type::I64, |ty| ty.clone())
            .expect("expected user index impl");

        assert_eq!(selected.target.as_ref().and_then(|target| target.impl_id), Some(impl_id));
        assert_eq!(selected.target.as_ref().map(|target| target.from_index_operator), Some(true));
        assert!(selected.builtin_index_output.is_none());
    }
```

- [ ] **Step 2: Run index service test to verify it fails**

Run: `cargo test -p rock-lib selection_service_prefers_user_index_impl_over_builtin_output`

Expected: FAIL with unresolved `select_index_method`.

- [ ] **Step 3: Implement index selection in the service**

Add this method to `SelectionService`:

```rust
    pub fn select_index_method<F>(
        &self,
        receiver_candidates: &[HirExpr],
        index_trait_id: DefId,
        index_ty: &Type,
        mut normalize: F,
    ) -> Option<SelectedMethod>
    where
        F: FnMut(&Type) -> Type,
    {
        receiver_candidates.iter().find_map(|candidate| {
            let candidate_ty = normalize(&candidate.ty);
            let user_target = self
                .select_required_trait_method(std::slice::from_ref(candidate), "index", index_trait_id, |ty| {
                    normalize(ty)
                })
                .map(|mut selected| {
                    if let Some(target) = selected.target.as_mut() {
                        target.trait_args = vec![index_ty.clone()];
                        target.from_index_operator = true;
                    }
                    selected
                });
            if user_target.is_some() {
                return user_target;
            }

            let builtin_output = crate::type_services::facts::TypeFacts::builtin_index_output(
                &candidate_ty,
                index_ty,
            )?;
            Some(SelectedMethod {
                receiver: candidate.clone(),
                function: crate::hir::HirFunction {
                    id: DefId::new(crate::ids::CrateId(u32::MAX), crate::ids::LocalDefId(u32::MAX)),
                    name: "index".to_string(),
                    qualified_name: None,
                    generic_params: Vec::new(),
                    generic_param_ids: Vec::new(),
                    generic_bounds: HashMap::new(),
                    params: Vec::new(),
                    ret_type: Type::Reference {
                        mutable: false,
                        inner: Box::new(builtin_output.clone()),
                    },
                    body: crate::hir::HirBlock {
                        stmts: Vec::new(),
                        ty: Type::Reference {
                            mutable: false,
                            inner: Box::new(builtin_output.clone()),
                        },
                    },
                    is_curried: false,
                    is_method: true,
                    self_receiver: Some(crate::ast::SelfReceiverMode::Shared),
                    is_unsafe: false,
                },
                impl_def: None,
                target: None,
                origin: SelectedOrigin::BuiltinIndex,
                receiver_adjustment: ReceiverAdjustment::Autoderef,
                substituted_params: Vec::new(),
                return_type: Type::Reference {
                    mutable: false,
                    inner: Box::new(builtin_output.clone()),
                },
                builtin_index_output: Some(builtin_output),
            })
        })
    }
```

- [ ] **Step 4: Replace index `chosen` selection in `apply_secondary`**

In `lib/src/lower/control_flow/secondary.rs`, replace the block that builds `chosen` from `autoderef_candidates.iter().find_map(...)` with a call to `select_index_method`. Preserve the existing type-var/generic fallback target for unresolved receivers:

```rust
                let selected_index = index_trait_id.and_then(|trait_id| {
                    self.selection_service().select_index_method(
                        &autoderef_candidates,
                        trait_id,
                        &resolved_index_ty,
                        |ty| self.resolve_projection_type(&self.engine.resolve(ty)),
                    )
                });
```

Then replace the destructuring of `chosen` with:

```rust
                let (receiver_expr, receiver_ty, builtin_output, target) =
                    if let Some(selected) = selected_index {
                        (
                            selected.receiver.clone(),
                            self.resolve_projection_type(&self.engine.resolve(&selected.receiver.ty)),
                            selected.builtin_index_output,
                            selected.target,
                        )
                    } else {
                        let target = match &resolved_expr_ty {
                            Type::TypeVar(_) | Type::Generic(_) => index_trait_id.and_then(|trait_id| {
                                self.selection_service().trait_member_id(trait_id, "index").map(|method_id| {
                                    HirMethodCallTarget {
                                        impl_id: None,
                                        trait_id: Some(trait_id),
                                        trait_args: vec![index.ty.clone()],
                                        method_id,
                                        from_index_operator: true,
                                    }
                                })
                            }),
                            _ => None,
                        };
                        (expr.clone(), resolved_expr_ty.clone(), None, target)
                    };
```

Keep associated `Output` projection construction unchanged.

- [ ] **Step 5: Run index regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_service_prefers_user_index_impl_over_builtin_output
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_str_indexing -- --exact
```

Expected: PASS for each command.

- [ ] **Step 6: Commit index selector migration**

```bash
git add lib/src/selection/service.rs lib/src/lower/control_flow/secondary.rs
git commit -m "select index dispatch through shared service"
```

## Task 7: Share Selected-Target Consumption Helpers In Mono And Codegen

**Files:**
- Modify: `lib/src/selection/matching.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/codegen/expr/mod.rs`

- [ ] **Step 1: Add failing selected-target helper tests**

In `lib/src/selection/matching.rs` tests, add:

```rust
    #[test]
    fn selection_target_matching_rejects_wrong_impl_identity() {
        let imp = HirImpl {
            id: def_id(1),
            owner: HirImplOwner::Named("Box".to_string()),
            type_name: "Box".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: std::collections::HashMap::new(),
        };
        let target = HirMethodCallTarget {
            impl_id: Some(def_id(2)),
            trait_id: None,
            trait_args: Vec::new(),
            method_id: def_id(3),
            from_index_operator: false,
        };

        assert!(!target_matches_impl(&imp, Some(&target)));
        assert!(target_matches_impl(&imp, None));
    }
```

- [ ] **Step 2: Run helper test**

Run: `cargo test -p rock-lib selection_target_matching_rejects_wrong_impl_identity`

Expected: PASS if Task 1 already added `target_matches_impl`; otherwise FAIL until Step 3 completes.

- [ ] **Step 3: Add selected trait-arg helper for mono/codegen consumption**

In `lib/src/selection/matching.rs`, add:

```rust
pub fn selected_trait_args_match_receiver(
    imp: &HirImpl,
    recv_ty: &Type,
    target: Option<&HirMethodCallTarget>,
) -> bool {
    let Some(target) = target else {
        return true;
    };
    if target.trait_args.is_empty() {
        return true;
    }
    if imp.trait_arg_types.len() != target.trait_args.len() {
        return false;
    }

    let receiver_args = receiver_arg_types(recv_ty);
    let mut subst = HashMap::new();
    if imp.receiver_arg_types.len() != receiver_args.len() {
        return false;
    }
    for (expected, actual) in imp.receiver_arg_types.iter().zip(receiver_args.iter()) {
        infer_generic_subst_from_types(expected, actual, &mut subst);
        if expected.substitute_generics(&subst) != *actual {
            return false;
        }
    }

    imp.trait_arg_types
        .iter()
        .zip(target.trait_args.iter())
        .all(|(expected, actual)| expected.substitute_generics(&subst) == *actual)
}
```

Export it from `lib/src/selection/mod.rs`.

- [ ] **Step 4: Replace mono's duplicate selected-target helpers**

In `lib/src/mono/methods.rs`, replace the bodies of `impl_matches_method_target` and `impl_trait_args_match_selected_target` with calls to the shared helpers:

```rust
    fn impl_matches_method_target(imp: &HirImpl, target: Option<&HirMethodCallTarget>) -> bool {
        crate::selection::target_matches_impl(imp, target)
    }

    fn impl_trait_args_match_selected_target(
        imp: &HirImpl,
        recv_ty: &Type,
        target: Option<&HirMethodCallTarget>,
    ) -> bool {
        crate::selection::selected_trait_args_match_receiver(imp, recv_ty, target)
    }
```

- [ ] **Step 5: Improve codegen selected-target diagnostic without removing fallbacks**

In `lib/src/codegen/expr/mod.rs`, keep targeted lookup order unchanged but replace the selected-target error message with:

```rust
                    return Err(CodegenError::from(format!(
                        "Selected method target for '{}::{}' could not be resolved by identity: {:?}",
                        recv_type_name, method, method_target
                    )));
```

Update the existing test `selected_method_target_does_not_fallback_to_direct_alias` to assert the new text:

```rust
        assert!(
            err.message
                .contains("could not be resolved by identity"),
            "unexpected error: {err}"
        );
```

- [ ] **Step 6: Run mono/codegen selected-target regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_target_matching_rejects_wrong_impl_identity
cargo test -p rock-lib selected_method_target_does_not_fallback_to_direct_alias
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_matches_generic_impl_trait_arguments -- --exact
```

Expected: PASS for each command.

- [ ] **Step 7: Commit selected-target consumer helpers**

```bash
git add lib/src/selection/matching.rs lib/src/selection/mod.rs lib/src/mono/methods.rs lib/src/codegen/expr/mod.rs
git commit -m "share selected target consumption helpers"
```

## Task 8: Update Roadmap And Audit Trackers

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Run proof tests before tracker edits**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_
cargo test -p rock-lib type_var_method_call_uses_trait_bound_target_before_name_candidates
cargo test -p rock-lib inherent_impl_method_call_carries_exact_target
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 2: Update Roadmap Task 12 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, insert this baseline bullet after the existing Task 11 bullet:

```markdown
- Roadmap Task 12 has landed for the shared selection-service slice: lowering now routes trait/method/operator/index dispatch through `lib/src/selection/`, and mono/codegen consume selected-target identity through shared helpers while compatibility fallbacks remain for Task 13 cleanup.
```

In the Task 12 section, add this status line immediately after the heading:

```markdown
**Status:** Complete for the shared selector service in `docs/superpowers/plans/2026-05-19-selection-service.md`; Task 13 remains responsible for deleting or narrowing mono/codegen fallback rediscovery paths.
```

In the focused-plan list near the bottom, add this bullet after `TypeId HIR Boundary`:

```markdown
8. `Shared Selection Service`: complete in `docs/superpowers/plans/2026-05-19-selection-service.md`; covers Task 12 by centralizing method, trait-bound, operator, and index selection behind `lib/src/selection/` while keeping Task 13 fallback cleanup explicit.
```

Replace:

```markdown
The first seven focused implementation plans are complete. Later work can continue with downstream `TypeId` consumer migration, selection-service, instance, MIR, or codegen boundaries.
```

with:

```markdown
The first eight focused implementation plans are complete. Later work can continue with Task 13 selection fallback cleanup, downstream `TypeId` consumer migration, instance, MIR, or codegen boundaries.
```

- [ ] **Step 3: Update audit tracker evidence and remaining work**

Before editing `docs/superpowers/plans/master-audit-checklist.md`, run `git rev-parse --short HEAD` and use that command output as the checked implementation commit marker.

In the semantic identity or dispatch-related evidence section, add this bullet:

```markdown
- `lib/src/selection/` centralizes trait, method, operator, and index selection for lowering and provides selected-target matching helpers used by mono/codegen, leaving broad fallback deletion to Roadmap Task 13.
```

In the `Done:` list for dispatch/selection work, add:

```markdown
- [x] Added a shared selection service for lowering-time trait/method/operator/index dispatch and shared selected-target helpers for downstream consumers.
```

In the `Still to do:` list for dispatch/selection work, add or keep:

```markdown
- [ ] Delete or narrow mono/codegen fallback rediscovery paths now that selected method targets are produced by the shared selection service.
```

- [ ] **Step 4: Run documentation diff checks**

Run: `git diff --check`

Expected: PASS with no whitespace errors.

- [ ] **Step 5: Commit tracker updates**

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for selection service"
```

## Task 9: Final Verification

**Files:**
- Verify: formatting, focused tests, integration regressions, full `rock-lib` suite, and clean worktree.

- [ ] **Step 1: Format Rust changes**

Run: `cargo fmt --all`

Expected: command exits successfully.

- [ ] **Step 2: Verify formatting is stable**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 3: Run focused selection tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib selection_
cargo test -p rock-lib lowerer_matching_wrappers_match_selection_helpers
cargo test -p rock-lib selection_service_selects_concrete_impl_method_by_identity
cargo test -p rock-lib selection_service_selects_type_var_bound_signature_by_trait_identity
cargo test -p rock-lib selection_service_selects_required_trait_operator_impl
cargo test -p rock-lib selection_service_prefers_user_index_impl_over_builtin_output
```

Expected: PASS for each command.

- [ ] **Step 4: Run lowering and selected-target regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_var_method_call_uses_trait_bound_target_before_name_candidates
cargo test -p rock-lib type_var_signature_call_uses_signature_id_target
cargo test -p rock-lib current_trait_self_signature_call_uses_signature_return_type
cargo test -p rock-lib type_var_method_value_uses_trait_bound_target_before_name_candidates
cargo test -p rock-lib inherent_impl_method_call_carries_exact_target
cargo test -p rock-lib selected_method_target_does_not_fallback_to_direct_alias
```

Expected: PASS for each command.

- [ ] **Step 5: Run user-visible integration regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_uses_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_matches_generic_impl_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_same_name_trait_default_methods_select_bound_trait_body -- --exact
cargo test -p rock-lib --test integration test_same_name_generic_trait_default_methods_select_bound_trait_body -- --exact
cargo test -p rock-lib --test integration test_same_name_trait_default_projection_uses_selected_trait_identity -- --exact
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact
cargo test -p rock-lib --test integration test_unary_neg_dispatches_through_stdlib_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_unary_not_generic_dispatches_through_stdlib_trait_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 6: Run the full documented library suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 7: Verify final diff whitespace**

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 8: Verify worktree state**

Run: `git status --short`

Expected: clean.

## Self-Review Notes

- Spec coverage: Tasks 1-2 create the selection module and pure matching boundary; Tasks 3-6 migrate concrete, trait-bound, operator, and index selection into the service; Task 7 makes mono/codegen use shared selected-target helpers and improves diagnostics; Task 8 updates roadmap/audit evidence; Task 9 verifies focused and full behavior.
- Scope control: The plan intentionally keeps broad mono/codegen fallback deletion for Task 13 while making selected targets the preferred path now.
- Product compatibility: The plan does not add `TypeId` persistence or require product artifact schema changes. Any `HirMethodCallTarget` extension discovered during implementation must add artifact compatibility tests before commit.
