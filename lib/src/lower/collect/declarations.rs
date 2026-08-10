//! Module export expansion helpers for Lowerer.

use std::collections::HashMap;

use crate::lower::collect_exports;
use crate::lower::module_context::SourceModuleResolver;
use crate::lower::Lowerer;

impl Lowerer {
    pub(crate) fn current_module_prefix(&self) -> Option<String> {
        self.modules.current_module_prefix()
    }

    /// Expand wildcard export sentinels from an already indexed source module.
    pub(crate) fn expand_glob_exports(
        &mut self,
        exports: HashMap<String, Option<String>>,
    ) -> HashMap<String, Option<String>> {
        let mut expanded = HashMap::new();
        for (key, value) in &exports {
            if let Some(module_prefix) = key.strip_suffix("::*") {
                let segments: Vec<String> =
                    module_prefix.split("::").map(ToString::to_string).collect();
                match SourceModuleResolver::new(&mut self.modules).load_module_by_path(&segments) {
                    Ok((sub_module, resolved_prefix)) => {
                        for (name, source) in self.expand_glob_exports(collect_exports(&sub_module))
                        {
                            if !name.ends_with("::*") {
                                expanded.insert(
                                    name.clone(),
                                    Some(source.unwrap_or_else(|| {
                                        format!("{}::{}", resolved_prefix, name)
                                    })),
                                );
                            }
                        }
                    }
                    Err(error) => self.diagnostics.push(format!(
                        "Failed to expand glob export {}::*: {}",
                        module_prefix, error
                    )),
                }
            } else {
                expanded.insert(key.clone(), value.clone());
            }
        }
        expanded
    }

    pub(crate) fn expand_glob_exports_with_prefix(
        &mut self,
        prefix: &str,
        exports: HashMap<String, Option<String>>,
    ) -> HashMap<String, Option<String>> {
        let previous_prefix = self.modules.replace_qualified_module_prefix(Some(prefix));
        let expanded = self.expand_glob_exports(exports);
        self.modules
            .restore_qualified_module_prefix(previous_prefix);
        expanded
    }
}
