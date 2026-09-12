# Instance Reachability DCE Design

## Goal

Roadmap Task 16 replaces HIR/name-based dead-code elimination with reachability over explicit callable identities after monomorphization.

After this task, the compiler should decide which callable records codegen sees by traversing `MonomorphizedProgram.instances` and their resolved call edges. Source names and backend symbols remain metadata; they must not be the semantic reachability key for monomorphized callables.

## Current State

Task 14 made `MonomorphizedProgram.instances` the authoritative callable universe for codegen declarations and body emission.

Task 15 made known generic function and method call sites carry `HirVarTarget::Instance(InstanceId)` and taught codegen to resolve those targets through registered `InstanceRecord.backend_symbol` values.

The remaining Task 16 gap is reachability:

- `lib/src/dce.rs` still builds a function call graph with HIR source names.
- `prune_dead_functions` and `count_dead_functions` operate on `HirProgram`, not `MonomorphizedProgram.instances`.
- The main compile pipeline does not currently call the old DCE pass before mono/codegen.
- Codegen declares and emits every `InstanceRecord` it receives.
- Product link records are attached after codegen by scanning all `MonomorphizedProgram.instances`.

This means callable reachability is not yet owned by the same explicit instance graph that codegen consumes.

## Scope

In scope:

- Add an instance reachability pass over `MonomorphizedProgram`.
- Run the pass after monomorphization and before codegen and product link-record attachment.
- Use `InstanceId` as the primary reachability node for concrete callable records.
- Traverse `HirVarTarget::Instance` expression edges in instance bodies.
- Keep compatibility traversal for direct `HirVarTarget::Function` and `HirVarTarget::Extern` references where no instance edge exists.
- Retain only reachable non-object instance bodies for codegen emission.
- Retain object-backed instance declarations only when reachable from the instance graph.
- Preserve product artifact behavior for generic bodies and metadata without serializing local `InstanceId`s.
- Retire the old HIR/name DCE as compiler authority; it may remain as legacy helper/test coverage if useful during the slice.

Out of scope:

- Do not redesign product artifact schema.
- Do not serialize `InstanceId` into product artifacts.
- Do not move codegen to MIR; Roadmap Task 21 owns MIR codegen.
- Do not redesign backend symbol generation.
- Do not build a full trait-selection reachability engine for targetless method calls; Task 15 call-edge work should already have resolved concrete generic calls that mono selected.
- Do not remove names used for diagnostics, aliases, template lookup, or product metadata.

## Reachability Model

The reachability graph is keyed by local `InstanceId` values from a single `MonomorphizedProgram`.

Each node is an `InstanceRecord`:

- `body: Some(HirFunction)` records can contribute outgoing edges by scanning their HIR body.
- `provided_by_object: true` records can be reachable leaves. They are declared for linking but not emitted by codegen.
- `declared` records without bodies do not contribute outgoing edges.

The pass should produce a pruned `MonomorphizedProgram.instances` map. The surrounding `HirProgram` remains available for metadata, compatibility lookups, and products, but it is not the source of callable emission authority.

## Roots

Primary roots:

1. The current crate entry function instance named `main` or whose origin is `InstanceOrigin::Function(main_def_id)` when the program has a canonical `main` function.
2. Concrete instance records that are required by reachable call edges from `main`.

Compatibility roots:

1. If an instance body contains `HirVarTarget::Function(def_id)` and an instance exists for `InstanceOrigin::Function(def_id)` with an empty substitution, mark that instance reachable.
2. If an instance body contains `HirVarTarget::Extern(def_id)`, no instance root is required; extern declarations still come from `HirProgram.externs` and codegen's existing extern declaration path.
3. If a direct name-based `HirExprKind::Var(name)` refers to a concrete function and a matching zero-substitution function instance exists, the pass may retain that instance as a compatibility bridge. This should be limited to existing non-migrated direct calls, not generic specializations that now carry `InstanceId`.

Not roots by default:

- All impl methods.
- All trait defaults.
- All object-backed dependency declarations.
- All generic specializations.

Those records become reachable only through explicit instance/function edges or narrowly defined compatibility roots.

Runtime C helper declarations such as `puts`, `malloc`, and `exit` remain codegen-owned runtime declarations and are not modeled as instance roots in this task.

## Edge Collection

The pass should traverse every expression inside reachable instance bodies and collect callable references from:

- `HirExprKind::ResolvedVar(HirVarRef { target: HirVarTarget::Instance(id), .. })`.
- `HirExprKind::Call(callee, args)`, including the callee expression and all arguments.
- Function values passed as arguments or stored in locals through `ResolvedVar`.
- Lambdas, blocks, branches, loops, matches, struct literals, enum variants, array/tuple literals, assignments, casts, refs, derefs, field accesses, and intrinsic arguments.

The traversal must not infer reachability from `HirVarRef.name` for `HirVarTarget::Instance`; the `InstanceId` is the semantic edge.

For unresolved compatibility forms:

- `HirVarTarget::Function(def_id)` may map to a zero-substitution function instance by origin.
- `HirExprKind::Var(name)` may map to a zero-substitution function instance by canonical HIR function name only when needed for existing non-migrated direct calls.
- `HirExprKind::MethodCall` should still traverse receiver and arguments. If a method call remains targetless after monomorphization, the pass should not try to reconstruct selection; preserving all methods for targetless compatibility is allowed only if a focused regression proves it is still needed.

## Pruning Behavior

The pass should remove unreachable entries from `MonomorphizedProgram.instances`.

Codegen then keeps its Task 14 behavior: declare and emit records from the remaining instances only.

Product link-record attachment should use the pruned instance set so dead object-backed symbols are not recorded as required link records.

The pass should not delete HIR metadata from `MonomorphizedProgram.program` in this slice. Product metadata and generic body export still operate from the resolved HIR/product pipeline, not from local `InstanceId` values.

## Product Artifact Behavior

Product artifacts cannot persist local `InstanceId` edges.

This task should preserve the current artifact boundary:

- Product metadata still records current-crate function, type, trait, impl, extern, and display/export identity by product IDs.
- Generic function bodies, generic impls, and trait default bodies remain available for downstream monomorphization.
- Local `HirVarTarget::Instance` values are still rejected when loading product artifacts.
- Link records should describe reachable object symbols needed by the compiled artifact, using product IDs and backend symbols.

If product emission needs to avoid writing monomorphized local instance targets into product bodies, do that by ensuring products are still built from resolved pre-mono HIR or by validating product bodies before serialization. Do not add an `InstanceId` sidecar to products in this task.

## API Shape

Preferred API in `lib/src/dce.rs`:

```rust
pub fn prune_unreachable_instances(program: &mut MonomorphizedProgram) -> InstanceDceReport
```

Suggested report:

```rust
pub struct InstanceDceReport {
    pub removed_instances: usize,
    pub retained_instances: usize,
}
```

The report is for tests and diagnostics only. The compile pipeline does not need to print it by default.

The existing `prune_dead_functions` and `count_dead_functions` helpers can remain temporarily for legacy tests, but their docs should no longer describe them as the compiler's authoritative DCE pass.

## Pipeline Integration

In `lib/src/lib.rs`:

1. Build MIR and run borrow checking as today.
2. Run `mono::monomorphize_with_crates`.
3. Run `dce::prune_unreachable_instances(&mut monomorphized)`.
4. Run codegen with the pruned monomorphized program.
5. Attach product link records from the same pruned monomorphized program.

This order keeps borrow checking on the full resolved HIR and makes codegen/link metadata consume the same reachable instance universe.

## Tests

Use TDD for each behavior change.

Focused DCE tests should cover:

- A reachable `main` instance is retained.
- An unreachable current-crate concrete function instance is removed.
- A direct `HirVarTarget::Instance` edge retains the target instance.
- A function value carried by `HirVarTarget::Instance` retains the target instance.
- A reused generic specialization instance reachable from `main` is retained.
- An unused generic specialization instance is removed.
- An object-backed instance reached by an instance edge is retained.
- An object-backed instance not reached by the graph is removed.
- A missing `InstanceId` edge is reported or ignored deterministically without falling back to names; prefer reporting in the testable report or preserving a clear invariant failure if the codebase already treats malformed HIR as invalid.

Pipeline or integration tests should cover:

- An unused current-crate function is not emitted in generated LLVM IR.
- A reachable generic function/method specialization still compiles and runs.
- An unused generic specialization does not appear in emitted LLVM IR.
- Artifact-backed reachable object symbols are still linked.
- Artifact-backed unreachable object symbols are not added to product link records when they are not needed.
- Existing Task 15 generic function/method call-edge regressions remain green.

Verification commands should include:

```bash
cargo fmt --all --check
cargo test -p rock-lib prune_unreachable_instances
cargo test -p rock-lib --test integration test_generic_instance_call_edges_for_function_and_method -- --exact
cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact
cargo test -p rock-lib
git diff --check
```

## Risks

- Starting from too few roots can remove callable records that are still needed by non-migrated compatibility paths.
- Starting from too many roots preserves the current no-DCE behavior and fails the task's purpose.
- Product bodies must not serialize local `InstanceId`s, even though codegen now relies on them after mono.
- Object-backed declarations are leaves; pruning them incorrectly can create link failures.
- Function values are call edges even when not immediately called at the same expression site.
- The old HIR/name DCE tests can give false confidence because the compiler now emits from instances, not HIR function maps.

## Success Criteria

- The compile pipeline prunes unreachable instance records before codegen.
- Codegen declarations, body emission, and product link records consume the same pruned instance set.
- Known monomorphized callable edges are traversed by `InstanceId`, not backend symbols or display names.
- Product artifacts continue to reject serialized local `InstanceId`s.
- Existing Task 15 regressions pass.
- Full `cargo test -p rock-lib` passes.
