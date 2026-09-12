# Independent Local Collection Design

**Date:** 2026-04-27
**Status:** Approved for implementation planning
**Scope:** Move local source declaration gathering out of `Lowerer` into a real `collect`-owned phase while keeping dependency crate registration and body lowering behavior unchanged.

## Purpose

The architecture audit says the `Real Collection And Name Resolution` track should make collection allocate IDs and gather declarations without constructing `Lowerer`, then add a resolver later. After the item-index work, the next clean step is to make local source declaration gathering a true collection responsibility instead of having `collect::collect` instantiate a `Lowerer`, run `lowerer.collect_declarations(&program.module)`, and then peel fields back out of it.

This design deliberately extracts only the local source declaration walk. It does not attempt resolver work yet, and it does not replace the existing dependency crate registration path. That keeps the change behavior-neutral while moving one real phase responsibility out of `Lowerer`.

## Goals

- Introduce a dedicated local collector in `collect/` that gathers local source declarations from the root program/module tree.
- Remove the use of `lowerer.collect_declarations(&program.module)` from `collect::collect` for local source declarations.
- Preserve the current `Declarations` shape closely enough that `Lowerer::from_declarations` and the rest of lowering continue to work.
- Keep dependency crate registration, stdlib prelude injection, loaded dependency summaries, and later body lowering behavior unchanged.

## Non-Goals

- Do not add a resolver yet.
- Do not replace dependency crate declaration registration yet.
- Do not rewrite `Lowerer::from_declarations` in this step.
- Do not change HIR, MIR, monomorphization, or codegen.
- Do not change parser/module IO behavior.

## Why This Is The Next Logical Step

The current `collect::collect` function still violates the phase boundary called out in the audit:

- It constructs a `Lowerer`.
- It asks that `Lowerer` to collect local declarations.
- It then extracts `Lowerer` fields into `Declarations`.

That means collection is still not a real phase. The next long-term-correct move is to stop using `Lowerer` as the collector for local source declarations and instead make `collect` own that walk directly.

Keeping dependency crate registration in `Lowerer` for now is intentional. External crate/prelude behavior is a separate seam tied to the later `Crate And Artifact Interface Split` work. Mixing that into this step would make the extraction broader, riskier, and less behavior-neutral.

## Architecture

### New Local Collector

Add a dedicated collector type under `lib/src/collect/`, for example `collect::collector::Collector`.

Its responsibility is narrow:

- Walk the root/local AST and inline/source-backed local modules.
- Gather local declaration headers and bookkeeping into a `Declarations`-compatible partial result.
- Reuse existing lowering helpers where needed to build the same `HirStruct`, `HirEnum`, `HirTrait`, `HirImpl`, `HirFunction`, and `HirExtern` header data that lowering expects.

This collector is not a replacement for `Lowerer`. It is a phase-specific builder for local declarations.

### Shared Semantics, Narrow Ownership Change

Long-term, collection should own more of the semantic tables directly. In this step, the best behavior-neutral path is to keep using the same header-construction logic already embedded in `Lowerer` helper methods and move orchestration ownership first.

That means the new collector may temporarily instantiate a helper `Lowerer` internally or share utility functions for header construction, but `collect::collect` itself should no longer rely on `lowerer.collect_declarations(&program.module)` as the mechanism for local collection.

This is an important boundary shift:

- Before: `collect::collect` delegates local collection to a general-purpose `Lowerer`.
- After: `collect::collect` drives a dedicated collector for local declarations and uses `Lowerer` only for the remaining non-local responsibilities.

### Source-Backed Local Modules

The local collector should preserve the current behavior for local `mod foo` declarations:

- It should use the already-existing local module loading path that collection uses today through `handle_mod_decl` behavior.
- It should keep populating `loaded_module_paths` and `module_file_cache` in the same shape expected by later lowering.
- It should continue collecting declarations from inline modules and source-backed local modules so the local declaration maps remain behavior-compatible.

The collector must not invent a new loader abstraction in this step. It should preserve current local module behavior and data shapes while moving ownership of the collection pass itself.

### Dependency Crates Stay On The Old Path For Now

Dependency crate registration should remain where it is today:

- `lowerer.register_crate_functions(crate_ctx)`
- optional `lowerer.inject_stdlib_prelude(crate_ctx)`
- existing loaded dependency summaries and module cache propagation

This means `collect::collect` will still use a `Lowerer`, but only for dependency/prelude/bootstrap responsibilities. The dedicated local collector will then gather local declarations.

That is the smallest architecture-improving step that clearly reduces `Lowerer`'s responsibility without entangling the dependency-interface seam.

## Data Flow

After this change, `collect::collect` should conceptually do the following:

1. Create bootstrap state needed for dependency crate registration and prelude policy.
2. Let the existing `Lowerer` path register dependency crates and prelude-related state.
3. Run the new local collector over `program.module`.
4. Build the multi-module `ItemIndex` from the root/local AST and already-known local source-backed ASTs.
5. Assemble `Declarations` from:
   - local declaration results from the new collector
   - dependency/prelude/module-cache state preserved from the bootstrap `Lowerer`

The key idea is that local declaration gathering becomes collection-owned, even if some supporting semantic construction logic is still shared with `Lowerer` internally.

## Boundary Constraints

To stay aligned with the audit and keep the step behavior-neutral:

- `collect::collect` should no longer call `lowerer.collect_declarations(&program.module)`.
- `Lowerer::from_declarations` should continue to work without semantic changes.
- Existing string-keyed declaration maps remain the active lowering inputs for now.
- The multi-module `ItemIndex` remains informational for future resolver work.

## Testing

Add focused tests that prove the local collector, not `Lowerer::collect_declarations`, is responsible for local source declaration gathering while preserving behavior.

Useful coverage for this step:

- A collect-level test showing local inline/source-backed module declarations are still gathered into `Declarations` after removing the `lowerer.collect_declarations(&program.module)` call from `collect::collect`.
- A regression test showing local imports / infix precedence / function signatures still appear in `Declarations` exactly as before.
- A test that dependency crate registration still works through the preserved `Lowerer` bootstrap path.
- Existing `collect` tests and `cargo test -p rock-lib` must stay green.

## Risks

- If the new local collector reaches too deeply into `Lowerer` internals, the extraction becomes cosmetic instead of architectural. The implementation should move pass ownership first, then keep helper reuse narrow and explicit.
- If dependency/prelude/bootstrap state and local declaration state become interleaved incorrectly, `Declarations` may stop matching what `Lowerer::from_declarations` expects.
- If this step tries to solve dependency crate collection too, scope will sprawl into the later interface-split track.

## Follow-Up Work

- Add a resolver that maps paths/imports/exports/prelude aliases to canonical IDs.
- Replace string-keyed declaration maps with canonical definition/alias tables.
- Split dependency crate interfaces away from `Lowerer` bootstrapping.
- Continue decomposing `Lowerer` so body lowering consumes resolved collection outputs rather than reconstructing phase state.
