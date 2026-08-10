#!/usr/bin/env python3
"""
Remove statement lowering methods from mod.rs (they're now in lower/statement.rs)
"""

import re

MOD_FILE = "lib/src/resolve/mod.rs"

# Read the file
with open(MOD_FILE, 'r') as f:
    content = f.read()
    lines = content.split('\n')

# Find the start and end lines
# Start: line with "fn lower_block" (around line 2473)
# End: line before "/// Get operator precedence" (which is already removed)
# Actually, we need to find what comes after extract_pattern_binding

start_idx = None
end_idx = None

for i, line in enumerate(lines):
    if 'fn lower_block(&mut self, block: &ast::Block) -> HirBlock' in line:
        start_idx = i
    # Find the end - look for the closing brace of extract_pattern_binding
    # followed by either another method or end of impl block
    if start_idx and 'fn extract_pattern_binding' in line:
        # Find the closing brace of this function
        brace_count = 0
        for j in range(i, len(lines)):
            brace_count += lines[j].count('{') - lines[j].count('}')
            if brace_count < 0:  # Found the end
                end_idx = j
                break
        if end_idx:
            break

if start_idx is None or end_idx is None:
    print(f"Could not find boundaries: start={start_idx}, end={end_idx}")
    exit(1)

print(f"Found section to remove: lines {start_idx+1} to {end_idx+1}")
print(f"Removing {end_idx - start_idx + 1} lines")

# Keep lines before start_idx and from end_idx+1 onwards
new_lines = lines[:start_idx] + lines[end_idx+1:]

# Write back
with open(MOD_FILE, 'w') as f:
    f.write('\n'.join(new_lines))

print(f"Done! New file has {len(new_lines)} lines (was {len(lines)})")
