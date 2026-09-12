#!/usr/bin/env python3
"""
Remove duplicated definitions from mod.rs.
"""

MOD_FILE = "lib/src/resolve/mod.rs"

with open(MOD_FILE, 'r') as f:
    lines = f.readlines()

# Find and remove each section
to_remove = []

# Find ResolveError
start = None
for i, line in enumerate(lines):
    if 'pub struct ResolveError' in line:
        start = i
    if start and 'pub type CompileError = ResolveError' in line:
        to_remove.append(('ResolveError', start, i + 1))
        break

# Find InferenceEngine
start = None
for i, line in enumerate(lines):
    if 'struct InferenceEngine {' in line:
        start = i
if start:
    # Find the end (just before Scope or Lowerer)
    for i in range(start + 1, len(lines)):
        if 'struct Scope' in lines[i] or 'pub struct Lowerer' in lines[i]:
            # Go back to find last non-empty line
            end = i
            while end > start and not lines[end-1].strip():
                end -= 1
            to_remove.append(('InferenceEngine', start, end))
            break

# Find Scope
start = None
for i, line in enumerate(lines):
    if 'struct Scope {' in line:
        start = i
if start:
    # Find the end (just before Lowerer)
    for i in range(start + 1, len(lines)):
        if 'pub struct Lowerer' in lines[i]:
            end = i
            while end > start and not lines[end-1].strip():
                end -= 1
            to_remove.append(('Scope', start, end))
            break

# Find intrinsics (is_intrinsic_name to end)
start = None
for i, line in enumerate(lines):
    if 'fn is_intrinsic_name(name: &str) -> bool' in line:
        start = i
        break
if start:
    to_remove.append(('Intrinsics', start, len(lines)))

# Sort by start line (descending) and remove
to_remove.sort(key=lambda x: x[1], reverse=True)

for name, start, end in to_remove:
    print(f"Removing {name}: lines {start+1} to {end} ({end - start} lines)")
    del lines[start:end]

# Write back
with open(MOD_FILE, 'w') as f:
    f.writelines(lines)

print(f"\nDone! File now has {len(lines)} lines")
