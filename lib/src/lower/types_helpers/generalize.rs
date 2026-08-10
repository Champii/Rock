//! Function generalization: thin wrapper around the canonical implementation in `infer/generalize`.

use crate::hir::*;
use crate::lower::Lowerer;

impl Lowerer {
    /// Generalize a single function by converting type variables to generic parameters.
    /// Delegates to the canonical `infer::generalize_single_function`.
    pub(crate) fn generalize_single_function(&self, func: HirFunction) -> HirFunction {
        crate::infer::generalize_single_function(
            &self.engine,
            &self.constraint_store,
            &self.function_type_vars,
            func,
        )
    }
}
