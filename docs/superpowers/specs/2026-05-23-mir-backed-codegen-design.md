# MIR-Backed Codegen Design

## Goal

Finish roadmap Task 21 by making codegen consume MIR as the executable compiler boundary. HIR and monomorphization may still provide declarations, layout metadata, instance records, product metadata, and link inputs, but LLVM function bodies should be emitted from `MirProgram` rather than HIR expression or statement trees.

## Current State

Task 19 made MIR carry canonical identities and runtime-complete forms for calls, methods, aggregates, enum variants, matches, closures, casts, runtime checks, and drops. Task 20 moved borrow checking onto typed MIR locations and indexed loan state. The remaining backend gap is that codegen still emits executable bodies from HIR:

- `compile_impl` builds MIR before monomorphization for borrow checking and debug printing, then monomorphizes HIR and calls `CodeGen::compile_program(&MonomorphizedProgram)`.
- `CodeGen::compile_program` declares layouts, externs, runtime helpers, and instance symbols from monomorphized HIR metadata, then calls `compile_function` on HIR bodies.
- `codegen::expr`, `codegen::stmt`, and `codegen::control_flow` still rediscover or reapply frontend semantics while emitting LLVM.
- MIR agreement checks currently guard that MIR has enough runtime shape, but codegen does not consume those MIR facts.

Task 21 should cut the executable-body path over to MIR in one full feature boundary.

## Non-Goals

- Do not redesign LLVM type layout, object emission, linking, or product artifact serialization.
- Do not remove monomorphization or the instance registry. Codegen still needs monomorphized metadata and backend symbols.
- Do not change user-facing language behavior, diagnostics, or runtime output intentionally.
- Do not introduce a permanent dual backend or HIR fallback for executable bodies.
- Do not broaden stdlib/sysroot discovery or compiler-owned stdlib loading.
- Do not implement new MIR optimizations as part of the backend switch.

## Architecture

The production boundary should be:

```text
Resolved HIR
    -> monomorphization / instance registry / DCE
    -> MIR built from monomorphized HIR
    -> MIR borrowck and MIR runtime agreement checks
    -> LLVM declarations and layouts from mono metadata
    -> LLVM function bodies from MIR
```

`lib/src/codegen/` remains responsible for LLVM mechanics: `Context`, `Module`, `Builder`, type mapping, ABI signatures, runtime helper declarations, object writing, linking, and symbol metadata. New MIR-specific codegen modules should own executable lowering from `MirProgram`.

HIR and mono remain inputs for metadata that MIR does not currently store directly:

- Instance records and backend symbols.
- Struct and enum layout declarations.
- Extern declarations and C ABI names.
- Trait implementation metadata needed to declare already-monomorphized targets.
- Product link records and object artifact paths.

Executable function bodies should come from MIR only. Existing HIR body codegen may remain in the tree during implementation as reference code, but the final compile path must not call HIR expression, statement, block, or control-flow lowering to emit function bodies.

## Module Structure

Create a MIR backend under `lib/src/codegen/mir/` with small files by responsibility:

- `mod.rs`: top-level MIR program and function body emission, block creation, branch finalization, and public entry point from `CodeGen`.
- `place.rs`: MIR `Place` address and value lowering for locals, deref, fields, indexes, and enum downcasts.
- `operand.rs`: MIR `Operand` lowering for copies, moves, constants, callable values, and unit values.
- `rvalue.rs`: MIR `Rvalue` lowering for use, refs, casts, closures, unary/binary operators, discriminants, and aggregates.
- `terminator.rs`: MIR `Terminator` lowering for return, goto, switch, call, and drop.
- `assert.rs` or a small section in `rvalue.rs`/`mod.rs`: MIR `StatementKind::Assert` lowering for bounds checks and runtime failure paths.

Reuse existing codegen helpers where they are backend mechanics rather than HIR semantics:

- `types.rs` for Rock type to LLVM type mapping and coercion.
- `operators.rs` and `intrinsics.rs` for primitive operations, if their APIs can accept already-lowered LLVM operands.
- Runtime declarations in `CodeGen::declare_runtime`.
- Object output and linking in `output.rs`.
- Closure ABI helpers where they operate on closure layouts rather than HIR expression trees.

## MIR Program Construction

`MirBuilder::build` should run on the monomorphized HIR program used for codegen, not only on the pre-monomorphized HIR used for earlier borrow checking. The final Task 21 path should use this monomorphized MIR for borrow checking, agreement checks, and codegen. If an early pre-monomorphized MIR diagnostic pass is retained, it must be documented as diagnostic-only and must not feed codegen or act as an executable-body fallback.

MIR function IDs must align with mono instance records:

- Non-generic functions map to `MirFunctionId::Function(DefId)` when their backend symbol is the ordinary function symbol.
- Specialized functions and methods map to `MirFunctionId::Instance(InstanceId)` and use `InstanceRecord.backend_symbol` for LLVM declarations.
- Closure bodies map to a MIR-owned closure function identity, such as `MirFunctionId::Closure(Box<MirClosureId>)` or an equivalent boxed/non-recursive key, so closure codegen does not need to read HIR lambda bodies.
- Externs map to `MirFunctionId::Extern(DefId)` for callable constants and declarations, but do not have MIR bodies.

If MIR builder cannot yet produce `MirFunctionId::Instance` bodies for monomorphized instances, Task 21 must add that capability before switching codegen.
If MIR builder cannot yet produce closure body MIR functions, Task 21 must add that capability before switching codegen for lambdas and closures.

## LLVM Declarations And Symbols

`CodeGen` should keep declaration setup separate from body emission:

1. Register struct and enum layout metadata from the monomorphized HIR program.
2. Register trait implementation metadata and trait member IDs only for declaration and symbol lookup needs.
3. Register instance records and backend symbols.
4. Declare runtime helpers and extern functions.
5. Declare LLVM functions for every non-object instance record.
6. Emit bodies by looking up the corresponding MIR function and lowering it.

Call resolution in MIR codegen should use canonical `MirCallable` or `MirFunctionId` data first. It must not search HIR method names, trait names, or receiver type names to rediscover frontend selection. If a MIR callable target cannot be resolved to a declared LLVM function or callable closure value, emit a `CodegenError` with the MIR callable and containing function context.

## Function Body Lowering

MIR function lowering should allocate and bind locals by `Local` index, not by HIR variable name:

- Create an LLVM function from the already-declared symbol and ABI signature.
- Create one LLVM basic block per MIR basic block before lowering statements.
- Allocate stack slots for MIR locals that need addressable storage.
- Bind parameters to the first `arg_count` locals according to the function ABI.
- Lower statements in MIR order.
- Lower terminators to LLVM branches, calls, drops, and returns.
- Verify each LLVM function after body emission.

Readable local names may be used for alloca/debug labels, but local identity is `Local`.

## Places And Operands

MIR `Place` lowering should be the only path for lvalues:

- A root `Local` resolves to its alloca/addressable storage or parameter storage.
- `Projection::Deref` loads thin pointer values directly; for fat references, it continues from the data pointer while preserving the length metadata required by the projected type.
- `Projection::Field` uses field indexes and `MirFieldIdentity` for struct/tuple layout validation.
- `Projection::Index(Local)` lowers the index local, emits the MIR-recorded bounds behavior for checked indexing, and computes the element address.
- `Projection::Downcast(VariantId)` resolves enum payload access through enum layout metadata and validates the variant identity.

MIR `Operand` lowering should be value-oriented:

- `Copy(place)` reads from a place without invalidating storage.
- `Move(place)` reads from a place; move validity is borrowck's responsibility.
- `Constant` emits LLVM constants or callable values.

## Rvalues, Statements, And Terminators

Rvalue lowering should use MIR semantics directly:

- `Use` stores or returns an operand value.
- `Ref` computes a place address and emits the correct reference representation.
- `Cast` emits primitive, pointer, slice, and array-to-slice conversions represented in MIR.
- `Closure` emits the same callable/closure ABI as current codegen, keyed by `MirClosureId` and capture places. The callable code pointer must target the MIR closure body function, not a HIR lambda body.
- `BinaryOp` and `UnaryOp` call backend operator helpers with lowered operands.
- `Discriminant` extracts enum tags.
- `Aggregate` constructs tuple, array, struct, and enum values using `AggregateKind` canonical IDs.

Statement lowering should handle:

- `Assign(place, rvalue)` by lowering the rvalue and storing/coercing into the place.
- `Assert(MirAssert)` by emitting the runtime check and failure path.
- `StorageLive` and `StorageDead` as no-ops or debug/lifetime markers in the first implementation.

Terminator lowering should handle:

- `Goto` as unconditional branch.
- `SwitchInt` as LLVM switch or conditional chain.
- `Return` by loading the return local/value convention and applying the existing `main` i32 ABI behavior.
- `Call` by lowering callable operands and arguments, storing the result into the destination place, and branching to the target.
- `Drop` by preserving existing drop behavior; if current codegen has no explicit drop glue, emit a no-op branch while keeping the MIR hook and tests proving behavior parity.

## Error Handling

MIR codegen should fail loudly with `CodegenError` instead of silently falling back to unit values or HIR reconstruction.

Required internal errors include:

- Missing MIR body for a non-object instance record.
- Unknown `MirFunctionId` or `MirCallable` target.
- Missing struct/enum layout metadata for canonical IDs.
- Local index out of range for the current `MirFunction`.
- Projection mismatch such as field access on a non-aggregate layout.
- Unsupported MIR `Rvalue`, `StatementKind`, or `Terminator` in the active backend.
- Invalid callable ABI or indirect call operand shape.

Errors should include the containing MIR function name or ID and the relevant canonical ID when possible.

## Testing Strategy

Task 21 needs representation-level MIR codegen tests plus full behavior parity.

Focused tests should cover:

- MIR function declaration/body lookup by `MirFunctionId::Function` and `MirFunctionId::Instance`.
- MIR closure body construction and lookup by `MirClosureId`.
- Local alloca/parameter binding by `Local` index.
- Place lowering for local, deref, field, index, and downcast projections.
- Operand lowering for primitive constants, callable constants, copy, and move.
- Rvalue lowering for refs, casts, aggregates, discriminants, unary/binary ops, closures, and assertions.
- Terminator lowering for goto, switch, return, call, and drop.
- Error tests for missing MIR bodies and unresolved canonical callable targets.

Behavior verification should include existing integration coverage for:

- Functions, recursion, higher-order functions, currying, and lambdas.
- Methods, trait dispatch, operators, and index dispatch.
- Struct construction, field access, tuple access, enum construction, and enum matches.
- Arrays, slices, strings, bounds checks, and vector indexing.
- Closures and captured variables.
- Borrowck-sensitive programs and raw pointer cases.
- Artifact-backed compilation and dependency object linkage.

Main verification commands:

```bash
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib codegen -- --nocapture
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo test -p rock-lib --test integration test_enum -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
cargo test -p rock-lib --test integration test_generic -- --nocapture
cargo test -p rock-lib --test integration test_stdlib -- --nocapture
cargo fmt --all --check
cargo test -p rock-lib
git diff --check
```

## Completion Criteria

Task 21 is complete when:

- The active compile path builds executable MIR from the monomorphized program before codegen.
- Borrow checking and MIR agreement checks run on the MIR used for codegen, or any retained early MIR pass is explicitly diagnostic-only.
- LLVM function bodies are emitted from MIR statements and terminators.
- The active compile path does not call HIR expression, statement, block, or control-flow lowering for executable bodies.
- Calls, methods, trait dispatch, intrinsics, closures, aggregates, enum matches, bounds checks, casts, and drops are emitted from canonical MIR facts.
- HIR/mono remain only metadata inputs for layout, symbols, instances, products, externs, and linking.
- No permanent HIR body fallback remains for unsupported MIR forms.
- Current integration behavior and runtime output remain stable unless a change is explicitly intentional and covered by tests.
- Focused MIR/codegen tests, behavior filters, full `cargo test -p rock-lib`, formatting, and `git diff --check` pass.
- Final code review finds no Critical or Important issues.

## Risks

- The current MIR builder may not yet produce monomorphized `MirFunctionId::Instance` bodies. That is the first architectural risk to resolve.
- Existing HIR codegen contains many ABI details for references, fat slices, closures, methods, and builtins. MIR codegen should reuse backend helpers carefully without reintroducing HIR semantic lookup.
- Enum downcast and match payload access must match current layout exactly to avoid subtle runtime regressions.
- The `main` return ABI currently truncates to i32 in HIR codegen; MIR codegen must preserve that behavior.
- A full boundary cutover is large. The implementation plan should use internal TDD slices and commits, but the final state must not keep a dual backend.
