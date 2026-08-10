# Compiler Architecture Audit Design

**Date:** 2026-04-24
**Status:** Draft for review
**Scope:** Whole `lib/src` compiler architecture, with CLI/crate-loading references only where they affect compiler boundaries

## Purpose

This document is an audit design, not an implementation plan. Its purpose is to lay down the architectural problems in the Rock compiler so future work sessions can iterate on them deliberately instead of fixing symptoms one bug at a time.

The audit is problem-first. It groups issues by root cause, then cites phase-specific evidence. It also answers the major design questions with opinionated long-term decisions that favor cleanliness, stable identity, and clear compiler phase ownership.

## Executive Summary

The compiler has top-level phase modules, but the semantic boundaries are still blurred. The main problem is not that files are organized poorly; it is that core compiler facts are still represented as strings, mutable maps, and reconstructed type/name conventions across multiple phases.

The strongest recurring issues are:

- Semantic identity is mostly string-based. Names, qualified paths, receiver pseudo-names, and backend symbols often stand in for real compiler IDs.
- `Lowerer` is still a semantic god object. It owns declarations, scopes, inference, modules, crates, prelude injection, trait context, constraints, and body lowering.
- Later phases reconstruct facts that earlier phases should have resolved. MIR, monomorphization, DCE, and codegen rediscover methods, traits, fields, variants, layouts, indexes, externs, and symbols from names and types.
- Canonical ownership is unclear. Methods, trait defaults, prelude imports, export aliases, cross-crate generic bodies, and object-backed bodies may be cloned or registered in multiple places.
- Crate/artifact loading details leak into lowering, monomorphization, and codegen instead of being exposed through a stable compiler-facing dependency interface.
- MIR is not currently the executable backend boundary. Borrow checking runs over MIR, but codegen lowers HIR directly and therefore sees a different model of the program.

The long-term direction should be to introduce canonical IDs and indexed tables, separate name resolution from lowering, make type and trait facts centrally owned, and decide that MIR is the semantic boundary consumed by codegen.

## Architecture Principles We Want

The audit uses these principles to judge current architecture and future refactors:

- Each compiler phase has one primary concern and a documented input/output contract.
- Semantic identity is stable and ID-based, not reconstructed from strings, display paths, type formatting, or backend symbols.
- Names are for source display and diagnostics. Backend symbols are for object output. Neither should be primary semantic identity.
- Imports, exports, prelude items, and aliases point to canonical definitions instead of cloning definitions under new names.
- HIR contains resolved references, not unresolved path conventions.
- Type lowering produces semantic types that refer to definitions by ID.
- Trait and method selection is centralized and produces resolved selections before backend lowering.
- MIR becomes the executable semantic boundary for borrow checking, optimization, drop insertion, and codegen.
- Monomorphization is keyed by `(DefId, Substitution)` and produces explicit `InstanceId`s.
- Codegen consumes MIR, layout information, ABI-lowered signatures, and resolved instances. It should not perform frontend semantic lookup.
- Crate and artifact loading expose compiler-facing capabilities and stable IDs, not source/object storage details.

## Cross-Cutting Problems

### 1. String-Based Semantic Identity

Definitions, modules, functions, structs, enums, traits, impls, methods, externs, and monomorphized instances are primarily addressed through strings. Existing `DefId` and `TypeId` types do not yet serve as real compiler-wide semantic identity.

Examples of the problem shape:

- HIR program maps are keyed by `String`.
- `Type::Struct`, `Type::Enum`, `Type::Generic`, and `Type::Projection` embed names instead of IDs.
- Method lookup uses receiver/type strings such as `Array`, `[U8]`, formatted array names, and mangled method names.
- Generic specialization and backend symbol generation use type strings and formatted names as semantic keys.
- Cross-crate identity is represented by qualified strings and crate-name prefixes.

This makes correctness depend on naming conventions instead of compiler-owned identity. It risks collisions, stale aliases, duplicated definitions, missed specializations, and poor diagnostics.

### 2. `Lowerer` As Semantic God Object

The current split between `collect`, `lower`, and `infer` is not a clean phase split. `collect` constructs a `Lowerer`, runs declaration collection through it, and extracts its fields. `Lowerer::from_declarations` then reconstructs scope and rehydrates another `Lowerer`.

`Lowerer` currently owns too many concerns:

- Inference engine and constraint store.
- Lexical scope.
- Declaration maps for structs, enums, traits, impls, externs, functions, and methods.
- Current function, trait, impl, crate, module, span, unsafe, and prelude context.
- Module loading/caching state.
- Crate/artifact summaries.
- Import aliases, export aliases, module-local aliases, and prelude exports.
- Trait conformance and default method injection context.

This makes phase order fragile because many operations depend on ambient mutable state being correct at the exact moment a body, trait, import, module, or artifact is processed.

### 3. Phase Boundary Bleeding

Lowering currently does more than AST to HIR conversion. It performs name resolution, type lowering, inference mutations, trait/method lookup, generic substitution, coercion, projection resolution, builtin handling, module loading, crate registration, and partial semantic validation.

Inference finalization then receives a `PartialHir`, but much of inference has already happened during lowering. Trait solving and method selection are spread across lower, infer, mono, and codegen.

The desired boundary is:

- Collection allocates item/module IDs and records declarations.
- Name resolution maps paths/imports/exports/prelude to IDs.
- Type lowering maps parsed type syntax to semantic types.
- HIR lowering consumes resolved AST and emits HIR plus constraints.
- Inference solves constraints and normalizes projections.
- Trait/method selection produces explicit semantic selections.
- MIR lowers resolved typed HIR into executable CFG.
- Mono creates concrete instances.
- Codegen lowers MIR and layouts to LLVM.

### 4. Canonical Ownership Is Unclear

Several semantic items can be owned or copied in more than one place:

- A method may exist in an impl, a trait, a `methods` map, a `functions` map under a mangled name, or a codegen registry.
- Trait default methods are copied into impls rather than referenced from the trait definition unless overridden.
- Prelude and export aliases clone functions or types under short names instead of mapping aliases to canonical definitions.
- Cross-crate generic bodies may exist in artifact HIR, lowerer maps, monomorphization state, or source-backed module caches.
- Object-backed bodies are partly represented as HIR items, partly as linked objects, and partly as codegen suppression rules.

The compiler needs one canonical owner for each semantic entity, with aliases and backend symbols as references/views.

### 5. Backend Reconstructs Frontend Semantics

MIR and codegen currently both inspect HIR and types to rediscover semantic facts. Codegen lowers HIR directly and contains method dispatch, trait lookup, builtin index handling, array/slice fallback rules, layout decisions, and runtime declarations.

This duplicates frontend semantics and makes borrowck/codegen disagreement possible. MIR may consider an approximate model while codegen emits behavior from a richer HIR model. The correct long-term goal is for MIR to be the executable semantic boundary, and for codegen to be mostly mechanical LLVM lowering.

### 6. Crate And Artifact Concerns Leak Into Compiler Phases

Loaded crates currently mix metadata, source ASTs, module caches, interface data, prelude exports, object paths, artifact mode, source bundle status, and cross-crate HIR. Lowering, monomorphization, and codegen branch on these details.

This couples compiler semantics to dependency storage mode. A source-backed crate, object-backed artifact, source-bundled artifact, and interface-only dependency should expose capabilities through a stable interface instead of changing how phases reason about definitions.

### 7. Testing And Diagnostics Risk

String-keyed identity and duplicated semantic lookup make tests less reliable as architecture safeguards. A test can pass because one phase handles a case while another phase still has a divergent reconstruction path.

Diagnostics also suffer because later phases often do not carry canonical declaration identity. Stable IDs would let diagnostics point to original declarations, distinguish same-name items in different modules/crates, and avoid errors based on backend or display names.

## Phase-Specific Evidence

### Front-End: AST, Parser, Macro Expansion, Formatter

The AST is broad and under-normalized. It acts as parser output, macro substrate, formatter substrate, and semantic pre-HIR carrier. Parsed types and paths carry strings and parser-specific structures that later phases reinterpret.

The parser also owns too much module/file behavior. It reads files, resolves module paths, and carries filesystem state in parse context. That makes parsing, module discovery, and IO harder to test and harder to reuse over virtual sources or artifact-provided modules.

Macro expansion reparses generated token fragments through parser internals and uses default configuration during reparse. That makes macro expansion a parser-coupled phase rather than a clean AST/token-tree transformation with explicit source mapping and diagnostics.

Formatting leaks into AST and global mutable formatter state. Formatting should use an explicit formatter context and either a syntax tree/trivia side table or formatter-specific preservation data, not semantic AST fields and global indentation mutexes.

### Middle: Collect, Lower, Infer, HIR, Types

The middle of the compiler contains the highest concentration of boundary problems.

`collect` is not a true independent item collection phase because it uses `Lowerer` internally. `Lowerer` then performs declaration collection, body lowering, module handling, trait conformance, inference mutations, and constraint creation.

HIR is described as resolved and typed, but many HIR and type structures still store strings. `HirExprKind::Var(String)`, string-keyed maps, string trait/type names, and string projections mean later phases still have to resolve or reconstruct semantic identity.

The `Type` model mixes semantic type structure, name identity, inference variables, generic names, projection names, builtin indexing knowledge, copy semantics, and display/substitution helpers. Type facts should move toward a type context and semantic IDs.

### MIR And Borrow Checking

MIR is currently used for borrow checking and optional debug printing, but it is not the representation consumed by codegen. It also imports HIR concepts and stores function identity as strings.

Several HIR constructs are lowered approximately or as placeholders in MIR. That is acceptable only if MIR is explicitly a borrowck-only IR that precisely models borrow-relevant effects. It is not acceptable if MIR is intended to become the backend IR.

The best long-term goal is to make MIR the executable semantic boundary. That means MIR must represent calls, resolved callees, projections, aggregate construction, enum variants, discriminants, matches, closure captures, bounds checks, drops, casts, and runtime-relevant operations explicitly.

Borrow checking should move toward indexed dataflow tables: `Location`, `LoanId`, `MovePathId`, `Local`, `PlacePathId`, and bitsets over IDs rather than cloned maps of loan data.

### Backend: Mono, Codegen, DCE

Monomorphization currently reconstructs semantic identity from function names, receiver names, type strings, and hardcoded external generic names. The long-term key should be `(DefId, Substitution)`, with generated `InstanceId`s and separate backend symbols.

Codegen currently repeats trait/method/index/layout knowledge. It should instead receive resolved instances, layout data, ABI-lowered signatures, and MIR operations. Codegen can own LLVM mechanics, runtime helper declarations, object emission, and linking, but not frontend semantic selection.

DCE is HIR/name-based and does not have a precise resolved call graph. It should eventually run over resolved `DefId`/`InstanceId` reachability after monomorphization, with explicit edges for method calls, function values, wrappers, trait impls, and runtime helpers.

### Crates, Artifacts, Sysroot

`LoadedCrate` mixes too many responsibilities: metadata, source, AST, interface, module tree, parsed module cache, prelude exports, object path, artifact mode, source-bundle status, and cross-crate HIR.

Artifact building currently re-drives compiler phases and owns semantic policy. It should instead serialize compiler products produced by a normal compile/session pipeline.

Module loading/cache logic is duplicated between crate loading, artifact building, parser/module parsing, and lowerer body loading. The compiler needs one module resolver/source provider keyed by crate/module identity, not just canonical filesystem paths.

Stdlib/prelude handling should become a crate-interface feature. The string `stdlib` can remain a CLI/sysroot convention, but compiler phases should see only that a loaded crate provides a prelude and that config permits prelude injection.

## Identity And ID Model Gaps

The compiler should introduce real semantic identity in stages. The target should distinguish source names, semantic IDs, type variables, monomorphized instances, and backend symbols.

Recommended ID families:

- `CrateId`: compiler-session identity for a loaded crate/dependency.
- `ModuleId`: identity for a module within a crate.
- `LocalDefId`: identity for a definition local to one crate.
- `DefId { crate_id: CrateId, local: LocalDefId }`: canonical cross-crate identity.
- Dedicated typed wrappers where useful: `FunctionId`, `StructId`, `EnumId`, `TraitId`, `ImplId`.
- Child IDs: `FieldId`, `VariantId`, `AssocTypeId`, `MethodId`.
- `TypeVarId`: inference variable identity.
- `TypeId`: only if the compiler actually interns types. Otherwise, remove or leave unused until type interning exists.
- `InstanceId`: monomorphized function/method identity after substitution.
- MIR IDs: `Local`, `BasicBlockId`, `Location`, `LoanId`, `MovePathId`, `PlacePathId`.

Names should remain available through tables for diagnostics, debug output, source display, and symbol generation. They should not be used as primary keys for semantic correctness.

Types should move from `Type::Struct(String, Vec<Type>)` toward semantic references such as `TyKind::Adt(DefId, Substitution)` or typed wrappers like `TyKind::Struct(StructId, Args)`. Associated type projections should reference trait and associated type IDs, not names.

## Module Ownership And Boundary Decisions

These are the opinionated long-term answers to the current architectural questions.

### MIR Should Become The Sole Codegen Input

MIR should not remain a borrowck-only side representation. The clean long-term architecture is for codegen to consume MIR plus layout/ABI/context tables. This avoids borrowck/codegen disagreement and gives one backend semantic boundary for drops, control flow, calls, closures, and runtime checks.

### `DefId` Should Be Cross-Crate

Use `DefId { crate_id, local }`, not a single global integer and not a plain local `u32`. This supports multiple crates, future multiple versions of the same crate, stable artifact references, and precise diagnostics.

### `TypeId` Should Mean Interned Type Identity Or Not Exist

`TypeId` should not be a nominal placeholder. If type interning is introduced, `TypeId` should identify interned canonical types in a type context. Until then, use explicit `TypeVarId` for inference variables and structured `Ty` values for semantic types.

### Methods Are Definitions Owned By Def Tables

Methods should have canonical `DefId`s. Impl and trait tables should reference method IDs. A method can be associated with an impl or trait, but it should not be cloned into unrelated maps under mangled names.

### Trait Default Methods Should Be Referenced, Not Copied

Default methods should remain owned by the trait. An impl without an override should point to the trait default method with an explicit selection/default marker. If instantiation or specialization requires generated bodies, those should be derived as instances, not copied into the impl as canonical source definitions.

### Prelude Should Be A Crate Interface Feature

The compiler should not special-case stdlib beyond the policy that prelude injection is enabled only when a crate providing a prelude is explicitly loaded and config permits injection. The loaded dependency interface should expose prelude exports as aliases to canonical `DefId`s.

### Object Artifacts Should Carry Generic/Default Bodies As Explicit Capabilities

Object-backed artifacts should expose concrete object symbols plus a separate body provider for generic functions, generic impl methods, and trait defaults that may need downstream instantiation. The compiler should query capabilities, not inspect artifact mode directly.

### Source Bundles Should Be Transitional, Not The Core Architecture

Source-bundled artifacts are useful for bootstrapping and debugging, but the clean long-term model is interface plus body sections plus link artifacts. Source bundles should not be required for normal dependency semantics.

### Parser Should Be Separated From Module IO

The parser should consume source text/tokens and return syntax. Module discovery, filesystem IO, source caches, artifact-provided modules, and virtual files should belong to a source/module loader layer.

### Crate Identity Should Support Multiple Versions

The compiler should not key loaded crates only by crate name. Use `CrateId` internally and retain package metadata such as name/version/source for diagnostics and artifact compatibility. This keeps the architecture compatible with real dependency graphs.

### Artifact Freshness Should Include Compiler And Dependency Identity

Artifact freshness should eventually include source hashes, compiler version, target triple, relevant config/features, dependency identities, and artifact schema version. Source hash alone is not enough.

## Candidate Future Architecture

This section describes the desired shape without prescribing an implementation order.

### Source And Module Loader

Owns files, source text, module discovery, canonical paths, virtual sources, and artifact-provided module bodies. The parser receives source content; it does not resolve sibling modules or load files itself.

### Item Collection And Definition Tables

Allocates crate, module, and definition IDs. Records item headers, module membership, visibility, source spans, and raw declarations. It should not lower bodies or solve types.

### Name Resolution

Resolves imports, exports, prelude aliases, paths, value names, type names, methods where syntactically named, and module namespaces to canonical IDs. Aliases map to IDs, not cloned definitions.

### Type Lowering And Type Context

Converts parsed type syntax into semantic types referring to definition IDs. Owns type variables, generic parameters, projections, substitutions, and optional type interning.

### HIR Lowering

Converts resolved AST bodies into HIR. HIR references canonical IDs for variables, functions, structs, fields, enums, variants, traits, impl methods, and associated items. It emits constraints but does not own global resolution policy.

### Inference And Trait Solving

Solves constraints, selects impls, normalizes associated type projections, applies coercions, and records method/trait selections. Operates on IDs and semantic types.

### MIR

Lowers typed/resolved HIR to executable control-flow graph. Encodes calls, resolved callees, places, projections, drops, aggregate construction, enum discriminants, matches, closures, bounds checks, and runtime-relevant operations.

### Monomorphization

Creates concrete instances from `(DefId, Substitution)`. Produces `InstanceId`s and maps them to backend symbol names separately. Cross-crate generic/default bodies are loaded through a body provider interface.

### Codegen

Consumes MIR, `InstanceId`s, layouts, ABI-lowered signatures, runtime helper requirements, and link metadata. It should not perform trait selection, method lookup, or source-level name resolution.

### Artifacts And Dependencies

Expose dependency interfaces, body providers, and link providers. Loaded dependency storage mode should be hidden behind capabilities. Compiler phases should not branch on source-backed versus object-backed artifacts except at explicit provider boundaries.

## Testing And Diagnostics Risks

The current architecture makes several categories of bugs likely:

- A method, type, or alias may resolve correctly in lowering but be reconstructed differently in mono or codegen.
- Borrowck may accept a program based on approximate MIR while codegen emits behavior from richer HIR semantics.
- Cross-crate and prelude behavior may differ depending on whether a dependency is loaded from source, source-bundled artifact, or object-backed artifact.
- Generic method specialization may silently miss or duplicate instances because semantic identity is encoded in strings.
- Diagnostics from later phases may report backend names or reconstructed type names instead of canonical declarations.

Future tests should target phase contracts, not only user-visible examples. Useful test categories include:

- Alias/import/export identity tests that verify one canonical definition is referenced through multiple names.
- Cross-crate tests for source-backed and object-backed dependencies producing equivalent semantic results.
- Method/trait selection tests that verify resolved selections before codegen.
- MIR/codegen agreement tests once codegen consumes MIR.
- Artifact capability tests for generic bodies, trait defaults, and object symbols.
- Diagnostic tests that verify errors point to canonical declarations and use source-level names.

## Candidate Refactor Tracks

These tracks are intentionally independent workstreams, not an ordered implementation plan.

### Identity And Arenas

Introduce crate/module/definition IDs, canonical definition tables, source-name tables, and alias maps. Stop using strings as primary semantic keys.

### Real Collection And Name Resolution

Make collection allocate IDs and gather declarations without constructing `Lowerer`. Add a resolver that maps paths and imports to IDs.

### Type Context And Semantic Types

Move from name-bearing `Type` variants toward semantic `Ty` values that reference IDs. Decide whether to intern types and make `TypeId` real.

### Lowerer Decomposition

Split `Lowerer` responsibilities into body lowering, scope management, type lowering, trait/method selection interface, module context, and diagnostics.

### Trait And Method Selection Service

Centralize impl lookup, receiver adjustment, generic substitution, associated type projection normalization, and method selection.

### MIR As Backend Boundary

Complete MIR lowering and move codegen from HIR to MIR. Encode runtime checks, drops, enum operations, closures, calls, and projections explicitly.

### Monomorphization Instances

Key mono by `(DefId, Substitution)`, produce `InstanceId`, and separate semantic instance identity from backend symbol names.

### Crate And Artifact Interface Split

Split loaded crate data into metadata, interface, source provider, body provider, and link provider. Hide storage mode from lower/mono/codegen.

### Parser, Macro, And Module Loader Cleanup

Move file/module IO out of parser. Give macro expansion explicit context, diagnostics, and source mapping. Avoid reparsing through default config.

### Formatter And Trivia Separation

Move formatting-only preservation data out of semantic AST and replace global formatter state with explicit formatter contexts.

### Borrowck Indexed Dataflow

Move borrowck toward indexed loan/move/location tables and bitset dataflow. Preserve span and origin data for diagnostics.

## Non-Goals For This Audit

This audit does not choose a first implementation task. It does not require all proposed IDs to be introduced at once. It does not prescribe a migration order. It does not require preserving deprecated code paths or compatibility shims unless a future work session identifies persisted data, external consumers, or shipped behavior requiring them.

The next step after this document is reviewed should be to choose one refactor track and write a focused implementation plan for that track.
