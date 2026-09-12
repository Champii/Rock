#!/usr/bin/env python3
"""
Remove expression lowering methods from mod.rs (they're now in lower/expression.rs)
"""

import re

MOD_FILE = "lib/src/resolve/mod.rs"

# Read the file
with open(MOD_FILE, 'r') as f:
    content = f.read()
    lines = content.split('\n')

# Find the start and end lines
# Start: line with "/// Get operator precedence" (around line 2671)
# End: line before "/// Convert AST ParseType" (around line 3907)

start_idx = None
end_idx = None

for i, line in enumerate(lines):
    if '/// Get operator precedence' in line:
        start_idx = i
    if '/// Convert AST ParseType to our Type system' in line:
        end_idx = i
        break

if start_idx is None or end_idx is None:
    print(f"Could not find boundaries: start={start_idx}, end={end_idx}")
    exit(1)

print(f"Found section to remove: lines {start_idx+1} to {end_idx}")
print(f"Removing {end_idx - start_idx} lines")

# Keep lines before start_idx and from end_idx onwards
new_lines = lines[:start_idx] + lines[end_idx:]

# Write back
with open(MOD_FILE, 'w') as f:
    f.write('\n'.join(new_lines))

print(f"Done! New file has {len(new_lines)} lines (was {len(lines)})")
