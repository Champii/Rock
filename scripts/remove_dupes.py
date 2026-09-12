#!/usr/bin/env python3
"""
Remove duplicated method sections from mod.rs.
The submodules (lower/expression.rs, lower/statement.rs, lower/types.rs, intrinsics.rs)
already contain this code.
"""

MOD_FILE = "lib/src/resolve/mod.rs"

with open(MOD_FILE, 'r') as f:
    lines = f.readlines()

# Find the sections to remove
to_remove = []

# 1. Find statement methods (lower_block to just before op_precedence)
stmt_start = None
stmt_end = None
for i, line in enumerate(lines):
    if 'fn lower_block(&mut self, block: &ast::Block) -> HirBlock' in line:
        stmt_start = i
    if stmt_start and '/// Get operator precedence' in line:
        stmt_end = i
        break

if stmt_start and stmt_end:
    to_remove.append(('Statement methods', stmt_start, stmt_end))

# 2. Find expression methods (op_precedence to just before lower_parse_type)
expr_start = None
expr_end = None
for i, line in enumerate(lines):
    if '/// Get operator precedence' in line:
        expr_start = i
    if expr_start and '/// Convert AST ParseType to our Type system' in line:
        expr_end = i
        break

if expr_start and expr_end:
    to_remove.append(('Expression methods', expr_start, expr_end))

# 3. Find type methods (lower_parse_type to just before finalize_types)
type_start = None
type_end = None
for i, line in enumerate(lines):
    if '/// Convert AST ParseType to our Type system' in line:
        type_start = i
    if type_start and '/// Finalize all types by resolving type variables' in line:
        type_end = i
        break

if type_start and type_end:
    to_remove.append(('Type methods', type_start, type_end))

# 4. Find intrinsics at the end
intr_start = None
for i, line in enumerate(lines):
    if 'fn is_intrinsic_name(name: &str) -> bool' in line:
        intr_start = i
        break

if intr_start:
    to_remove.append(('Intrinsics', intr_start, len(lines)))

# Sort by start line (descending) and remove
to_remove.sort(key=lambda x: x[1], reverse=True)

for name, start, end in to_remove:
    print(f"Removing {name}: lines {start+1} to {end} ({end - start} lines)")
    del lines[start:end]

# Write back
with open(MOD_FILE, 'w') as f:
    f.writelines(lines)

print(f"\nDone! File now has {len(lines)} lines")
