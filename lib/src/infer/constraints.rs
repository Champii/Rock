//! Constraint store for deferred type constraint verification.
//!
//! During lowering, constraints are accumulated here instead of being checked
//! immediately. A dedicated solver pass (Phase 4) discharges them, producing
//! errors for unsatisfied trait bounds or ambiguous integer literal types.

use crate::ids::TypeVarId;
use crate::lexer::Span;
use crate::types::{TraitBound, Type};

/// A deferred type constraint accumulated during lowering.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// The type must implement the given trait (from BinOp, method calls, etc.)
    Trait {
        ty: Type,
        bound: TraitBound,
        span: Span,
        context: String,
    },
    /// The type variable must resolve to some integer type (from integer literals)
    IntLiteral { var: TypeVarId, span: Span },
    /// The type variable must resolve to some float type (from float literals)
    FloatLiteral { var: TypeVarId, span: Span },
    /// Two types must unify once deferred call-site inference has resolved them.
    Equality {
        left: Type,
        right: Type,
        span: Span,
        context: String,
    },
}

/// Accumulated type constraints produced during lowering.
#[derive(Debug, Clone, Default)]
pub struct ConstraintStore {
    pub constraints: Vec<Constraint>,
}

impl ConstraintStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, ty: Type, bound: TraitBound, span: Span, context: &str) {
        self.constraints.push(Constraint::Trait {
            ty,
            bound,
            span,
            context: context.to_string(),
        });
    }

    pub fn add_int_literal(&mut self, var: TypeVarId, span: Span) {
        self.constraints.push(Constraint::IntLiteral { var, span });
    }

    pub fn add_float_literal(&mut self, var: TypeVarId, span: Span) {
        self.constraints
            .push(Constraint::FloatLiteral { var, span });
    }

    pub fn add_equality(&mut self, left: Type, right: Type, span: Span, context: &str) {
        self.constraints.push(Constraint::Equality {
            left,
            right,
            span,
            context: context.to_string(),
        });
    }

    pub fn literal_default_type(&self, var: TypeVarId) -> Option<Type> {
        self.literal_default_type_for_representative(var, Some)
    }

    /// Returns the literal default for one resolved type-variable class.
    /// Mixed integer and float evidence deliberately produces no default.
    pub fn literal_default_type_for_representative(
        &self,
        representative: TypeVarId,
        mut resolve_representative: impl FnMut(TypeVarId) -> Option<TypeVarId>,
    ) -> Option<Type> {
        let mut default = None;
        for constraint in &self.constraints {
            let candidate = match constraint {
                Constraint::IntLiteral {
                    var: constrained, ..
                } if resolve_representative(*constrained) == Some(representative) => Type::I64,
                Constraint::FloatLiteral {
                    var: constrained, ..
                } if resolve_representative(*constrained) == Some(representative) => Type::F64,
                _ => continue,
            };
            match &default {
                Some(existing) if *existing != candidate => return None,
                Some(_) => {}
                None => default = Some(candidate),
            }
        }
        default
    }

    pub fn has_literal_evidence_for_representative(
        &self,
        representative: TypeVarId,
        mut resolve_representative: impl FnMut(TypeVarId) -> Option<TypeVarId>,
    ) -> bool {
        self.constraints.iter().any(|constraint| match constraint {
            Constraint::IntLiteral {
                var: constrained, ..
            }
            | Constraint::FloatLiteral {
                var: constrained, ..
            } => resolve_representative(*constrained) == Some(representative),
            Constraint::Trait { .. } | Constraint::Equality { .. } => false,
        })
    }
}
