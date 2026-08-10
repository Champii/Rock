use std::collections::HashMap;

use crate::hir::{
    HirAssociatedTypeDef, HirExpr, HirFunction, HirImpl, HirMethodCallTarget, HirParam,
    HirSelectedMethodTarget,
};
use crate::ids::DefId;
use crate::types::{GenericParamId, TraitBound, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverAdjustment {
    None,
    AutorefShared,
    AutorefMut,
    MutToSharedRef,
    BuiltinDeref,
    TraitDeref,
    ArrayRefToSliceRef,
    ArrayValueToMutSliceRef,
    ArrayValueToSliceRef,
    ArrayValueToSliceValue,
}

#[derive(Debug, Clone)]
pub struct ReceiverCandidate {
    pub expr: HirExpr,
    pub adjustment: ReceiverAdjustment,
    pub can_autoref_mut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedOrigin {
    InherentImpl { impl_id: DefId },
    TraitImpl { impl_id: DefId, trait_id: DefId },
    TraitBound { trait_id: DefId },
    CurrentTrait { trait_id: DefId },
    UnresolvedGeneric { trait_id: DefId },
}

#[derive(Debug, Clone)]
pub struct SelectedMethod {
    pub receiver: HirExpr,
    pub function: Option<HirFunction>,
    pub impl_def: Option<HirImpl>,
    pub target: HirMethodCallTarget,
    pub origin: SelectedOrigin,
    pub receiver_adjustment: ReceiverAdjustment,
    pub substituted_params: Vec<HirParam>,
    pub return_type: Type,
    pub pending_impl_bounds: Vec<(Type, TraitBound)>,
    pub associated_types: Vec<HirAssociatedTypeDef>,
    pub owner_substitution: HashMap<GenericParamId, Type>,
    pub owner_generic_params: Vec<GenericParamId>,
}

#[derive(Debug, Clone)]
pub struct SelectedConstructorMember {
    pub function: HirFunction,
    pub impl_def: HirImpl,
    pub target: HirMethodCallTarget,
    pub substituted_params: Vec<HirParam>,
    pub return_type: Type,
    pub pending_impl_bounds: Vec<(Type, TraitBound)>,
    pub owner_substitution: HashMap<GenericParamId, Type>,
    pub owner_generic_params: Vec<GenericParamId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionAuthority {
    pub origin: SelectedOrigin,
    pub target: HirSelectedMethodTarget,
    pub receiver_adjustment: ReceiverAdjustment,
    pub return_type: Type,
}

impl SelectionAuthority {
    pub fn impl_id(&self) -> Option<DefId> {
        match self.target {
            HirSelectedMethodTarget::ImplMethod { impl_id, .. } => Some(impl_id),
            HirSelectedMethodTarget::TraitMethod { .. } => None,
        }
    }

    pub fn trait_id(&self) -> Option<DefId> {
        match &self.target {
            HirSelectedMethodTarget::ImplMethod { selected_trait, .. } => {
                selected_trait.as_ref().map(|selected| selected.trait_id)
            }
            HirSelectedMethodTarget::TraitMethod { trait_id, .. } => Some(*trait_id),
        }
    }

    pub fn method_id(&self) -> Option<DefId> {
        match self.target {
            HirSelectedMethodTarget::ImplMethod { method_id, .. } => Some(method_id),
            HirSelectedMethodTarget::TraitMethod { member_id, .. } => Some(member_id),
        }
    }

    pub fn trait_args(&self) -> &[Type] {
        match &self.target {
            HirSelectedMethodTarget::ImplMethod { selected_trait, .. } => selected_trait
                .as_ref()
                .map(|selected| selected.trait_args.as_slice())
                .unwrap_or_default(),
            HirSelectedMethodTarget::TraitMethod { trait_args, .. } => trait_args,
        }
    }
}

impl SelectedMethod {
    pub fn authority(&self) -> SelectionAuthority {
        SelectionAuthority {
            origin: self.origin.clone(),
            target: self.target.target.clone(),
            receiver_adjustment: self.receiver_adjustment,
            return_type: self.return_type.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionDiagnostic {
    NoImplementation {
        operation: String,
        receiver: Type,
    },
    ReceiverMismatch {
        operation: String,
        receiver: Type,
    },
    AmbiguousCandidates {
        operation: String,
        receiver: Type,
        candidates: Vec<DefId>,
    },
    SelectedTargetMissing {
        method_name: String,
        target: HirMethodCallTarget,
    },
    TraitMemberMissing {
        trait_id: DefId,
        method_name: String,
    },
    TraitMemberIdMissing {
        trait_id: DefId,
        member_id: DefId,
    },
}

impl SelectionDiagnostic {
    pub fn message(&self) -> String {
        match self {
            SelectionDiagnostic::NoImplementation {
                operation,
                receiver,
            } => {
                format!(
                    "No implementation found for operator '{}' on type {}",
                    operation, receiver
                )
            }
            SelectionDiagnostic::ReceiverMismatch {
                operation,
                receiver,
            } => {
                format!(
                    "No receiver adjustment for '{}' on type {}",
                    operation, receiver
                )
            }
            SelectionDiagnostic::AmbiguousCandidates {
                operation,
                receiver,
                candidates,
            } => {
                format!(
                    "Ambiguous selection for '{}' on type {}: {:?}",
                    operation, receiver, candidates
                )
            }
            SelectionDiagnostic::SelectedTargetMissing { method_name, .. } => format!(
                "Selected method target for '{}' could not be resolved by identity",
                method_name
            ),
            SelectionDiagnostic::TraitMemberMissing {
                trait_id,
                method_name,
            } => format!("Trait {trait_id:?} does not declare selected member '{method_name}'"),
            SelectionDiagnostic::TraitMemberIdMissing {
                trait_id,
                member_id,
            } => format!("Trait {trait_id:?} does not declare selected member {member_id:?}"),
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
    fn selection_diagnostic_preserves_selected_target_identity() {
        let target = HirMethodCallTarget::impl_method(
            def_id(20),
            def_id(30),
            Some(crate::hir::HirSelectedTraitMember {
                trait_id: def_id(10),
                member_id: def_id(30),
                trait_args: vec![Type::I64],
            }),
        );
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

    #[test]
    fn selected_method_authority_records_impl_trait_method_and_args() {
        let impl_id = def_id(20);
        let trait_id = def_id(10);
        let method_id = def_id(30);
        let target = HirMethodCallTarget::impl_method(
            impl_id,
            method_id,
            Some(crate::hir::HirSelectedTraitMember {
                trait_id,
                member_id: method_id,
                trait_args: vec![Type::I64],
            }),
        );
        let selected = SelectedMethod {
            receiver: HirExpr {
                kind: crate::hir::HirExprKind::Var("value".to_string()),
                ty: Type::I64,
                span: crate::Span::default(),
            },
            function: None,
            impl_def: None,
            target,
            origin: SelectedOrigin::TraitImpl { impl_id, trait_id },
            receiver_adjustment: ReceiverAdjustment::BuiltinDeref,
            substituted_params: Vec::new(),
            return_type: Type::Bool,
            pending_impl_bounds: Vec::new(),
            associated_types: Vec::new(),
            owner_substitution: HashMap::new(),
            owner_generic_params: Vec::new(),
        };

        let authority = selected.authority();

        assert_eq!(authority.target, selected.target.target);
        assert_eq!(
            authority.receiver_adjustment,
            ReceiverAdjustment::BuiltinDeref
        );
        assert_eq!(authority.return_type, Type::Bool);
    }
}
