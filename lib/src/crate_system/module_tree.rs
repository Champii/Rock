use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::ast::{Module, TopLevel};

use super::{ModuleNode, ModuleTree};

pub(crate) fn build_module_tree(module: &Module, crate_name: &str) -> Result<ModuleTree, String> {
    let mut modules = BTreeMap::new();
    build_module_nodes(module, crate_name, &mut modules)?;
    Ok(ModuleTree { modules })
}

pub(super) fn build_module_tree_from_graph(
    graph: &crate::source_loader::ModuleGraph,
    crate_name: &str,
) -> Result<ModuleTree, String> {
    let mut modules = BTreeMap::new();
    build_module_nodes(graph.root_module(), crate_name, &mut modules)?;
    for loaded in graph.modules() {
        build_module_nodes(&loaded.module, &loaded.qualified_name, &mut modules)?;
    }
    for loaded in graph.modules() {
        link_loaded_module_node(&mut modules, &loaded.qualified_name);
    }
    Ok(ModuleTree { modules })
}

pub(super) fn module_file_cache_from_graph(
    graph: &crate::source_loader::ModuleGraph,
) -> std::collections::HashMap<PathBuf, Module> {
    graph.module_file_cache()
}

fn build_module_nodes(
    module: &Module,
    qualified_prefix: &str,
    modules: &mut BTreeMap<String, ModuleNode>,
) -> Result<(), String> {
    let mut submodules = Vec::new();

    for top_level in &module.top_levels {
        if let TopLevel::Module(mod_decl) = top_level {
            let inner_module = &mod_decl.0;
            if let Some(ref mod_name) = inner_module.name {
                let mod_name_str = mod_name.name.clone();
                let qualified_name = format!("{}::{}", qualified_prefix, mod_name_str);

                build_module_nodes(inner_module, &qualified_name, modules)?;

                submodules.push(mod_name_str);
            }
        }
    }

    let node = ModuleNode {
        name: qualified_prefix
            .rsplit("::")
            .next()
            .unwrap_or(qualified_prefix)
            .to_string(),
        qualified_name: qualified_prefix.to_string(),
        submodules,
    };

    modules.insert(qualified_prefix.to_string(), node);
    Ok(())
}

fn link_loaded_module_node(modules: &mut BTreeMap<String, ModuleNode>, qualified_name: &str) {
    let Some((parent, name)) = qualified_name.rsplit_once("::") else {
        return;
    };
    let Some(parent) = modules.get_mut(parent) else {
        return;
    };

    if !parent.submodules.iter().any(|submodule| submodule == name) {
        parent.submodules.push(name.to_string());
    }
}
