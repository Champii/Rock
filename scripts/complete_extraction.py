#!/usr/bin/env python3
"""
Complete extraction of mod.rs into submodules.
Uses temp files as source for method implementations.
"""

import os
import re
from pathlib import Path

BASE_DIR = Path("lib/src/resolve")
TEMP_DIR = BASE_DIR / "temp"

# Mapping of temp files to target modules
# Format: (temp_file, target_file, module_name, imports_needed)
EXTRACTIONS = [
    # types/helpers.rs
    ("temp_05_lowerer_type_helpers.rs", "types/helpers.rs", "helpers", ["crate::types::Type"]),

    # lower/program.rs
    ("temp_07_lowerer_entry_points.rs", "lower/program.rs", "program", [
        "crate::ast", "crate::hir::*", "crate::crate_system::CrateContext",
        "crate::Config", "super::super::context::Lowerer",
    ]),

    # crates/registration.rs
    ("temp_08_lowerer_registration.rs", "crates/registration.rs", "registration", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "crate::crate_system::CrateContext", "super::super::context::Lowerer",
    ]),

    # crates/bodies.rs
    ("temp_09_lowerer_crate_bodies.rs", "crates/bodies.rs", "bodies", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "super::super::context::Lowerer",
    ]),

    # collect/declarations.rs
    ("temp_10_lowerer_collect_decls.rs", "collect/declarations.rs", "declarations", [
        "std::collections::HashSet", "crate::ast", "crate::hir::*",
        "crate::types::Type", "super::super::context::Lowerer",
        "super::super::{path_names, collect_exports}",
    ]),

    # imports/resolver.rs
    ("temp_11_lowerer_imports.rs", "imports/resolver.rs", "resolver", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "crate::lexer::Span", "crate::new_parser",
        "super::super::context::Lowerer", "super::super::path_names",
    ]),

    # collect/types.rs
    ("temp_12_lowerer_collect_types.rs", "collect/types.rs", "types", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "super::super::context::Lowerer",
    ]),

    # collect/traits.rs (combining collect_trait from temp_13)
    ("temp_13_lowerer_collect_sigs.rs", "collect/traits.rs", "traits", [
        "crate::ast", "crate::hir::*", "crate::types::{TraitBound, Type}",
        "super::super::context::Lowerer",
    ]),

    # collect/impls.rs
    ("temp_14_lowerer_collect_impls.rs", "collect/impls.rs", "impls", [
        "crate::ast", "crate::hir::*", "crate::types::{TraitBound, Type}",
        "super::super::context::Lowerer",
    ]),

    # lower/function.rs
    ("temp_15_lowerer_func_headers.rs", "lower/function.rs", "function", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "super::super::context::Lowerer",
    ]),

    # lower/module.rs (from bodies temp)
    ("temp_16_lowerer_bodies.rs", "lower/module.rs", "module", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "super::super::context::Lowerer",
    ]),

    # traits/defaults.rs
    ("temp_17_lowerer_trait_bodies.rs", "traits/defaults.rs", "defaults", [
        "crate::ast", "crate::hir::*", "crate::types::Type",
        "super::super::context::Lowerer",
    ]),

    # types/finalize.rs
    ("temp_23_lowerer_finalize.rs", "types/finalize.rs", "finalize", [
        "crate::hir::*", "crate::types::Type", "super::super::context::Lowerer",
    ]),

    # types/generalize.rs (combining multiple temps)
    ("temp_25_lowerer_generalize_1.rs", "types/generalize.rs", "generalize", [
        "std::collections::BTreeMap", "std::collections::BTreeSet",
        "crate::hir::*", "crate::types::Type", "super::super::context::Lowerer",
    ]),
]

def read_temp_file(name):
    """Read a temp file and extract the method implementations."""
    path = TEMP_DIR / name
    with open(path, 'r') as f:
        return f.read()

def create_impl_file(target, imports, content):
    """Create a file with an impl Lowerer block."""
    # Clean up the content - remove temp file header
    lines = content.split('\n')
    start_idx = 0
    for i, line in enumerate(lines):
        if line.strip().startswith('fn ') or line.strip().startswith('pub fn '):
            start_idx = i
            break
        if line.strip().startswith('impl Lowerer'):
            start_idx = i
            break

    content = '\n'.join(lines[start_idx:])

    # If content doesn't start with impl, wrap it
    if not content.strip().startswith('impl Lowerer'):
        content = f"impl Lowerer {{\n{content}\n}}"

    # Build the file
    imports_str = '\n'.join(f"use {imp};" for imp in imports)

    return f"""//! Lowerer implementation methods

{imports_str}

{content}
"""

def create_mod_rs(subdir, modules):
    """Create a mod.rs file for a subdirectory."""
    mod_decls = '\n'.join(f"mod {m};" for m in modules)
    reexports = '\n'.join(f"pub use {m}::*;" for m in modules)

    return f"""//! Submodule for resolve

{mod_decls}

{reexports}
"""

def main():
    print("Starting complete extraction...")

    # Create mod.rs files for each subdirectory
    subdirs = {
        "types": ["helpers", "finalize", "generalize"],
        "collect": ["declarations", "types", "traits", "impls"],
        "crates": ["registration", "bodies"],
        "traits": ["defaults", "conformance"],
        "imports": ["resolver"],
    }

    for subdir, modules in subdirs.items():
        mod_content = create_mod_rs(subdir, modules)
        mod_path = BASE_DIR / subdir / "mod.rs"
        with open(mod_path, 'w') as f:
            f.write(mod_content)
        print(f"Created {mod_path}")

    # Process each extraction
    for temp_name, target_path, module_name, imports in EXTRACTIONS:
        content = read_temp_file(temp_name)
        file_content = create_impl_file(target_path, imports, content)

        target = BASE_DIR / target_path
        target.parent.mkdir(parents=True, exist_ok=True)

        with open(target, 'w') as f:
            f.write(file_content)
        print(f"Created {target}")

    print("\nDone! Now you need to:")
    print("1. Update lib/src/resolve/mod.rs to use these submodules")
    print("2. Fix any import issues")
    print("3. Run tests")

if __name__ == "__main__":
    main()
