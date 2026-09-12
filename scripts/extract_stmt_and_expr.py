#!/usr/bin/env python3
"""
Remove both expression and statement lowering methods from mod.rs
"""

MOD_FILE = "lib/src/resolve/mod.rs"

# Read the file
with open(MOD_FILE, 'r') as f:
    lines = f.readlines()

# Find the sections to remove
# Statement methods: lower_block (line ~2734) to extract_pattern_binding end (line ~2930)
# Expression methods: op_precedence comment (line ~2932) to lower_parse_type (line ~4169)

stmt_start = None
stmt_end = None
expr_start = None
expr_end = None

for i, line in enumerate(lines):
    # Statement section start
    if 'fn lower_block(&mut self, block: &ast::Block) -> HirBlock' in line:
        stmt_start = i

    # Statement section end (just before expression section)
    if stmt_start and '/// Get operator precedence' in line:
        stmt_end = i - 1  # Include blank line before comment

    # Expression section start
    if '/// Get operator precedence' in line:
        expr_start = i

    # Expression section end (just before lower_parse_type)
    if '/// Convert AST ParseType to our Type system' in line:
        expr_end = i
        break

print(f"Statement section: lines {stmt_start+1} to {stmt_end+1} ({stmt_end - stmt_start + 1} lines)")
print(f"Expression section: lines {expr_start+1} to {expr_end} ({expr_end - expr_start} lines)")

# Remove both sections (remove from end to start to preserve indices)
# First remove expression section
new_lines = lines[:expr_start] + lines[expr_end:]

# Then remove statement section from the new array
# Need to recalculate indices since we removed expression section first
# The statement section is still in the same place since it was before expression section
new_lines = new_lines[:stmt_start] + new_lines[stmt_end+1:]

# Write back
with open(MOD_FILE, 'w') as f:
    f.writelines(new_lines)

print(f"Done! New file has {len(new_lines)} lines (was {len(lines)})")
