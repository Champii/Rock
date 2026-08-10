# Instance Call Edges Design

## Goal

Roadmap Task 15 replaces backend-symbol call targets with explicit instance call edges where monomorphization already knows the selected `InstanceId`.

After this task, backend symbols remain output metadata for LLVM and product artifacts, but monomorphization must not rewrite known instance calls to `HirExprKind::Var(record.backend_symbol)` as the semantic target. Those calls should carry an instance-backed target that codegen resolves through `MonomorphizedProgram.instances`.

## Current State

Roadmap Task 14 made `MonomorphizedProgram.instances` the authoritative callable universe for codegen declarations and body emission.

The remaining Task 15 gap is call identity:

- `lib/src/mono/specialize.rs` returns `record.backend_symbol` for reused and newly created generic function specializations.
- `lib/src/mono/methods.rs` rewrites selected generic method calls to `HirExprKind::Var(record.backend_symbol)`.
- `lib/src/mono/mod.rs::lookup_specialized_function` searches existing instance records by backend symbol.
- Codegen call paths still treat many callable targets as strings through `HirExprKind::Var`, `function_symbols_by_id`, `impl_method_backend_symbols`, and LLVM declaration maps.

Those string maps can continue to exist as compatibility and backend lookup structures, but known monomorphized instance calls should no longer use emitted symbols as semantic call targets.

## Scope

In scope:

- Add a monomorphized callable target representation that can carry an `InstanceId`.
- Rewrite generic function specialization calls to carry the `InstanceId` returned by the registry.
- Rewrite generic impl-method and trait-method specialization calls to carry the `InstanceId` returned by the registry.
- Teach codegen direct-call and callable-materialization paths to resolve instance-backed targets by looking up the corresponding `InstanceRecord.backend_symbol`.
- Keep `InstanceRecord.backend_symbol` as LLVM output metadata and object/product symbol metadata.
- Keep source names and local variable names as strings where they are diagnostics or local scope metadata.

Out of scope:

- Do not replace all monomorphizer string-keyed lookup tables in this slice.
- Do not replace HIR/name-based DCE; Roadmap Task 16 owns instance reachability.
- Do not move codegen to MIR; Roadmap Task 21 owns MIR codegen.
- Do not redesign backend symbol generation.
- Do not change product artifact schema unless a small compatibility sidecar is strictly required by this slice.
- Do not remove targetless method compatibility paths unrelated to calls where mono has a concrete `InstanceId`.

## Data Model

Add an instance-backed callable target to HIR or the monomorphized HIR compatibility layer.

Preferred model:

```rust
pub enum HirCallableTarget {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
}
```

If changing `HirVarTarget` directly creates too much churn, use an equivalent narrowly scoped representation that is still visible in monomorphized expressions and codegen. The important property is that known instance calls carry `InstanceId`, not `backend_symbol`.

`InstanceId` stays local to one `MonomorphizedProgram`; it should not be serialized into product artifacts in this task.

## Mono Design

Generic function specialization should return instance identity together with the callable signature and return type.

When a specialization already exists:

1. Look up `InstanceId` by `InstanceKey`.
2. Read the `InstanceRecord` for type/signature metadata.
3. Return the `InstanceId`, not only `record.backend_symbol`.

When a specialization is created:

1. Intern the new `InstanceRecord`.
2. Return the newly allocated `InstanceId` with the specialized function type.
3. Rewrite the call target to the instance-backed callable target.

Generic method specialization should follow the same pattern. The selected method body, receiver adjustment, and call argument construction remain unchanged; only the resulting callable target changes from backend-symbol `Var` to instance-backed target.

The monomorphizer may still keep source-name maps for finding generic templates. Those maps are not call-edge identity once an instance has been selected.

## Codegen Design

Codegen should resolve instance-backed call targets through `MonomorphizedProgram.instances`.

Declaration and body emission remain Task 14 behavior:

1. Declare every callable from `InstanceRecord` values.
2. Compile every non-object `InstanceRecord.body`.

Call resolution adds an instance path:

1. For a direct instance target, look up the record by `InstanceId`.
2. Use `record.backend_symbol` to find the already-declared LLVM function.
3. Emit a direct call with the existing ABI handling.
4. For function values, materialize the callable from the same instance-backed lookup.

Missing instance IDs should produce a clear codegen error. Codegen should not fall back to guessing a backend symbol from source names when an instance target is present.

## Invariants

- A known monomorphized instance call carries `InstanceId` as semantic identity.
- Backend symbols are generated/output metadata, not semantic call targets.
- `InstanceId` is resolved only against the `MonomorphizedProgram.instances` table from the same compilation.
- Existing `ResolvedVar(Function)` and `ResolvedVar(Extern)` paths remain valid for non-specialized direct functions and extern declarations.
- String local variables and source/display names remain available for diagnostics and local scope lookup.
- If an expression carries an instance target and the instance is missing, compilation fails explicitly.

## Tests

Use TDD for each behavior change.

Focused mono tests should cover:

- Reused generic function specializations rewrite calls to an instance target instead of a backend-symbol `Var`.
- Newly created generic function specializations rewrite calls to an instance target.
- Reused generic method specializations rewrite calls to an instance target.
- Newly created generic method specializations rewrite calls to an instance target.
- Backend symbol changes do not change the semantic call target for a reused instance.

Focused codegen tests should cover:

- Direct calls to instance-backed targets resolve through `InstanceRecord.backend_symbol`.
- Instance-backed callable values materialize through `InstanceRecord.backend_symbol`.
- Missing instance IDs produce a codegen error instead of a guessed symbol lookup.
- Codegen still declares object-backed instance records with exact backend symbols.

Integration regressions should cover:

- Generic function calls still compile and run.
- Generic impl-method calls still compile and run.
- Artifact-backed generic function or method calls still compile and link.
- Same-name/backend-symbol-sensitive regressions from Task 14 remain green.

Verification commands should include:

```bash
cargo fmt --all --check
cargo test -p rock-lib monomorphize_call
cargo test -p rock-lib selected_method_target_uses_exact_instance_backend_symbol
cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact
cargo test -p rock-lib
git diff --check
```

## Risks

- `InstanceId` is compilation-local. Accidentally serializing it or using it across product artifact boundaries would create stale identities.
- HIR is still the compatibility representation for codegen, so adding an instance target must not break parser/lower/product artifact assumptions about source HIR.
- Codegen has several call entry points: direct calls, method calls, callable materialization, and function values. Missing one can leave a backend-symbol semantic path in place.
- Some existing string maps are still needed for local variables, diagnostics, template lookup, and LLVM declarations. This task should remove backend-symbol call identity without pretending all strings are gone.
