# Instance Registry Callable Universe Design

## Goal

Roadmap Task 14 makes `InstanceRegistry` the authoritative backend input for every emitted callable body.

After this task, `MonomorphizedProgram.instances` is the complete callable universe for codegen declarations and body emission. `HirProgram` still carries type, layout, trait, impl-template, extern, and compatibility metadata, but codegen must not emit callable bodies by directly iterating HIR function or impl-method maps.

## Current State

The compiler already has `InstanceRegistry`, `InstanceKey`, `InstanceOrigin`, `InstanceRecord`, and `MonomorphizedProgram.instances` in `lib/src/mono/registry.rs`.

Existing instance records cover important backend paths:

- Generic function specializations in `lib/src/mono/specialize.rs`.
- Generic impl-method and trait-method specializations in `lib/src/mono/methods.rs`.
- Object-backed dependency functions and methods in `lib/src/mono/external.rs`.
- Codegen declaration and body emission for records in `lib/src/codegen/mod.rs`.

The remaining split callable universe is in mono and codegen:

- `lib/src/mono/process.rs` partitions current-crate functions into `concrete_functions` and `generic_functions`, then rebuilds `program.functions` from concrete entries.
- `lib/src/mono/external.rs` does the same current-crate function partitioning while also registering object-backed records.
- `lib/src/codegen/mod.rs` still declares current-crate functions by iterating `program.functions_by_id()`.
- `lib/src/codegen/mod.rs` still declares and compiles concrete impl methods by iterating `program.impls_in_order()`.
- `lib/src/codegen/mod.rs` still compiles current-crate function bodies from HIR maps before compiling instance bodies.

This means codegen still has multiple sources for emitted callables: direct HIR functions, direct HIR impl methods, generic specialization records, and object-backed records.

## Scope

Task 14 changes the backend callable boundary, not call-edge identity.

In scope:

- Register every concrete current-crate function that may be emitted as an `InstanceRecord` with empty substitution.
- Register every concrete current-crate impl method that may be emitted as an `InstanceRecord` with empty substitution.
- Represent concrete trait default method bodies that may be emitted with an explicit instance origin instead of relying on direct HIR maps.
- Keep generic function and generic method specializations in the existing substitution-keyed instance path.
- Keep object-backed function and method declarations in instance records with `provided_by_object = true`.
- Make codegen declare callable records from `MonomorphizedProgram.instances`.
- Make codegen compile callable bodies only from `MonomorphizedProgram.instances`.
- Preserve HIR metadata consumption for structs, enums, traits, impl lookup, extern declarations, receiver ABI metadata, and compatibility lookup.

Out of scope:

- Do not replace backend-symbol call targets with `InstanceId` edges; Roadmap Task 15 owns that migration.
- Do not remove all string-keyed monomorphizer lookup tables; later identity cleanup owns remaining semantic map removal.
- Do not replace HIR/name-based DCE with instance reachability; Roadmap Task 16 owns that migration.
- Do not move codegen to MIR; Roadmap Task 21 owns that migration.
- Do not change product artifact schemas unless a small sidecar adjustment is strictly required by this slice.
- Do not preserve duplicate transitional emission paths for backward compatibility once the instance records are authoritative.

## Instance Model

`InstanceKey { origin, substitution }` remains the semantic identity for emitted callables. Backend symbols remain record metadata and are not identity.

The zero-substitution records should use the same registry as generic specializations:

- Current-crate concrete functions use `InstanceOrigin::Function(function.id)` and `substitution = []`.
- Current-crate concrete impl methods use `InstanceOrigin::ImplMethod { owner, method }` and `substitution = []`.
- Generic function and method specializations keep their existing origin and concrete substitution.
- Object-backed declarations use the same origin as the source callable and `substitution = []` unless they represent a specialized artifact body.

Trait default methods need an explicit origin. The preferred model is extending `InstanceOrigin` with a trait-default variant that carries the owning trait ID and method ID. This avoids pretending a default body is a top-level function or an impl method when its canonical owner is a trait.

Every record must carry enough information for codegen to declare the LLVM function:

- `source_name` for diagnostics and display.
- `backend_symbol` for LLVM symbol emission and temporary Task 15 call compatibility.
- `declared` when a declaration signature exists without a compiled body.
- `body` when codegen should compile the callable.
- `provided_by_object` when the callable body is supplied by linked object code.

## Mono Design

Mono should build a complete instance registry before returning `MonomorphizedProgram`.

For current-crate functions:

1. Continue partitioning generic templates from concrete functions so generic templates are not emitted directly.
2. Process concrete function bodies as today.
3. Intern each processed concrete function into the registry with empty substitution.
4. Keep `program.functions` available as HIR metadata if needed, but do not rely on it as the backend emission list.

For current-crate impl methods:

1. Keep collecting impl metadata for method selection and specialization.
2. Process concrete methods that do not contain unresolved generic, type-var, or projection types.
3. Intern each processed concrete method into the registry with empty substitution and the same backend symbol codegen currently uses.
4. Leave generic methods as templates until a call site creates a specialization record.

For trait defaults:

1. Use `HirProgram` method indexes to identify trait default methods by trait ID and method ID.
2. Register concrete default bodies when they are emitted as standalone callable bodies.
3. Use the explicit trait-default origin so identity remains separate from impl-provided methods.

For object-backed dependencies:

1. Keep registering object-backed functions and methods as declarations-only records.
2. Ensure current-crate copies of object-backed bodies are not also registered as compilable records.

Duplicate registration must reuse the existing `InstanceId`. This is the same behavior specializations already rely on.

## Codegen Design

Codegen should treat `MonomorphizedProgram.instances` as the only callable emission list.

Declaration flow:

1. Register structs, enums, traits, impl metadata, extern declarations, runtime functions, and receiver ABI metadata as today where those are not emitted callable bodies.
2. Call `register_instances(instances)` to declare every callable record.
3. Populate backend-symbol helper maps such as `impl_method_backend_symbols` from records, not from direct HIR method loops.

Compilation flow:

1. Iterate `instances.values()`.
2. Skip `provided_by_object` records.
3. Compile `record.body` using `record.backend_symbol`.
4. Error if a non-object record has no body; declaration-only records must be object-backed.

The direct codegen loops over `program.functions_by_id()` and `program.impls_in_order()` must stop declaring or compiling emitted callable bodies. HIR iteration can remain for non-callable metadata, selected-target validation, and compatibility tables, but not as an emission source.

## Invariants

- Every callable body that reaches LLVM has exactly one authoritative `InstanceRecord`.
- Object-backed callable bodies are declared from records and never compiled.
- Declaration-only instance records are object-backed; non-object records carry compiled bodies.
- Generic templates are not emitted unless a concrete specialization record exists.
- Duplicate `InstanceKey` registration reuses one `InstanceId`.
- Backend symbols are outputs and lookup aids, not semantic identity.
- Task 14 may still allow `HirExprKind::Var(record.backend_symbol)` call targets as temporary Task 15 compatibility.
- `MonomorphizedProgram.program` remains available for metadata, but not as the callable body emission list.

## Tests

Use TDD for each behavior change.

Focused unit tests should cover:

- Registering a concrete current-crate function creates one zero-substitution `InstanceRecord`.
- Registering the same concrete function twice reuses the same `InstanceId`.
- Registering concrete impl methods distinguishes owners and method IDs while using empty substitution.
- Generic functions and generic methods continue to reuse specialization records for duplicate substitutions.
- Object-backed functions and methods remain declaration-only records and are not compiled.
- Trait default methods have explicit record identity and do not collide with impl methods.

Codegen tests should cover:

- Codegen declares callable records from `instances`.
- Codegen compiles record bodies from `instances`.
- Codegen no longer needs direct `program.functions_by_id()` or `program.impls_in_order()` emission loops to compile function and impl-method bodies.
- Non-object records without bodies produce a clear codegen error.

Integration regressions should cover:

- Current-crate `main` and helper functions still compile and run.
- Concrete impl methods still compile and dispatch.
- Generic functions and generic methods still specialize and reuse duplicate records.
- Trait defaults still dispatch correctly.
- Object-backed dependency artifacts still declare linked symbols without compiling their bodies.

Verification commands should include:

```bash
cargo fmt --all --check
cargo test -p rock-lib instance_registry_reuses_same_function_instance
cargo test -p rock-lib --test integration test_hello_world -- --exact
cargo test -p rock-lib
git diff --check
```

## Risks

- Existing codegen helper maps still support selected-target and receiver ABI behavior. The migration must repopulate them from records without accidentally removing non-callable metadata that codegen still needs.
- Some current HIR methods may appear concrete by signature but contain unresolved generic or projection expressions. Registration must use the same concrete check codegen currently uses before emitting methods.
- Backend-symbol call targets remain until Task 15. Task 14 must not make calls less resolvable while removing direct HIR emission loops.
- Trait defaults have ambiguous ownership if forced into existing origins. Adding an explicit origin is safer but requires updating registry tests and any origin match code.
- The current `MonomorphizedProgram.program` shape may invite accidental future direct emission. Tests should guard the codegen path rather than relying only on convention.
