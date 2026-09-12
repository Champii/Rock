#!/usr/bin/env python3
"""
Comprehensive cleanup of mod.rs - remove all duplicated definitions
"""

import re

MOD_FILE = "lib/src/resolve/mod.rs"

with open(MOD_FILE, 'r') as f:
    content = f.read()

# Remove ResolveError struct and impl (lines ~39-73)
content = re.sub(
    r'/// A resolution error.*?\npub type CompileError = ResolveError;',
    '',
    content,
    flags=re.DOTALL
)

# Remove Scope struct and impl (between "/// Scope for name resolution" and "/// The main lowering context")
content = re.sub(
    r'/// Scope for name resolution.*?(?=/// The main lowering context)',
    '',
    content,
    flags=re.DOTALL
)

# Remove InferenceEngine struct and impl (between "/// Type inference engine" and "/// Scope for name resolution")
content = re.sub(
    r'/// Type inference engine.*?(?=/// Scope for name resolution)',
    '',
    content,
    flags=re.DOTALL
)

# Remove intrinsics functions at the end (is_intrinsic_name, infer_intrinsic_return_type, infer_intrinsic_arg_types)
content = re.sub(
    r'\n/// Check if a name matches the intrinsic pattern.*$',
    '',
    content,
    flags=re.DOTALL
)

# Clean up multiple blank lines
content = re.sub(r'\n{3,}', '\n\n', content)

with open(MOD_FILE, 'w') as f:
    f.write(content)

# Count lines
lines = content.split('\n')
print(f"Done! File now has {len(lines)} lines")
