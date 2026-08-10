# Collect-Owned Context Design

**Date:** 2026-04-27
**Status:** Auto-approved for implementation
**Scope:** Replace `LocalCollector`'s full `Lowerer` state bag with a narrower collect-owned context while keeping dependency bootstrap on `Lowerer` and preserving current `Declarations` outputs.

## Purpose

The last step moved local traversal and header construction into `collect`, but `LocalCollector` still carries a full `Lowerer` and mutates that general-purpose lowering context throughout collection. That leaves the collect phase coupled to a semantic god object even though collection now needs only a smaller subset of state.

The next narrow step is to introduce a collect-owned mutable context for local collection and header building. `collect::collect` should still bootstrap dependency crates and stdlib prelude state through `Lowerer`, then hand only the collection-relevant state into `collect`.

## Goals

- Introduce a `collect`-owned context type that holds only the mutable state needed during local collection.
- Switch `LocalCollector` and `collect::headers` to use that collect-owned context instead of a full `Lowerer`.
- Preserve current `Declarations` outputs and `Lowerer::from_declarations` behavior.
- Preserve local inline-module, local source-backed module, import, export, and dependency bootstrap behavior.

## Non-Goals

- Do not move dependency crate registration or stdlib prelude injection out of `Lowerer` yet.
- Do not add resolver tables or canonical ID resolution yet.
- Do not change `Declarations` shape.
- Do not change body lowering, MIR, monomorphization, or codegen.

## Existing Seam

`collect::collect` already uses `Lowerer` only for bootstrap work plus the mutable state later consumed by `LocalCollector`. After the previous extraction, the remaining coupling is that `LocalCollector` stores a `Lowerer` and `collect::headers` accepts `&mut Lowerer`, even though the active collection path only touches a subset of fields and helpers:

- declaration maps and collect-time alias maps
- collect-time scope definitions for imports and function headers
- type-lowering context needed for header construction
- module loader and module/export expansion helpers
- trait/header bookkeeping such as `current_trait`, `function_type_vars`, and errors

Everything else on `Lowerer` is collect-irrelevant baggage for this phase.

## Architecture

### New Collect Context

Add `lib/src/collect/context.rs` with a `CollectContext` type.

`CollectContext` owns the mutable state used by the active collect path:

- declaration maps and vectors (`structs`, `enums`, `traits`, `impls`, `externs`, `functions`, `function_sigs`, `methods`)
- collect-time bookkeeping (`function_type_vars`, `import_aliases`, `export_function_aliases`, `infix_precedence`, `errors`)
- type/header state (`engine`, `scope`, `current_trait`, `current_trait_generics`, `current_function`)
- module/source bookkeeping (`current_crate_name`, `current_module_path`, `loaded_modules`, `loaded_module_paths`, `module_file_cache`, `artifact_module_index`, `stdlib_prelude_exports`)

It should not carry body-lowering-only state such as impl-body context, unsafe lowering flags, deferred constraints, or module-local body aliases.

### Bootstrap Handoff

`collect::collect` should keep the current bootstrap path:

1. Create a bootstrap `Lowerer` with `Lowerer::bootstrap_for_collection(...)`.
2. Register dependency crates and optional stdlib prelude on that bootstrap lowerer.
3. Convert the bootstrap lowerer into `CollectContext`.
4. Run local collection entirely on `CollectContext`.

This keeps the dependency/bootstrap seam stable while removing `Lowerer` from the active local collection path.

### Helper Ownership In This Step

Move the active collect-time helpers needed by `LocalCollector` into `CollectContext`:

- parse-type lowering for header construction
- parameter/self header helpers used by `collect::headers`
- impl receiver/type header helpers used by `collect::headers`
- local module loading and path-based module loading used by collect-time import/export expansion
- collect-time import/glob-import handling and glob-export expansion
- collect-time error recording

This is intentionally limited to the active collect path. Legacy lower-owned collection code can remain in `lib/src/lower/collect/**` for now.

## Boundary Rules

- `LocalCollector` must no longer store a `Lowerer`.
- `collect::headers` must no longer accept `&mut Lowerer`.
- `collect::collect` may still use `Lowerer` for dependency bootstrap before the handoff.
- `Lowerer::from_declarations` must remain unchanged in behavior.
- Keep the active string-keyed declaration maps and existing `Declarations` fields unchanged.

## Testing

Add focused coverage for the new ownership boundary:

- a collect-level test proving bootstrap state can be imported into `CollectContext`
- updated header-builder unit tests proving `collect::headers` works against `CollectContext`
- existing collect regressions must continue to pass unchanged
- full `cargo test -p rock-lib` must still pass

## Risks

- Copying helper logic into `collect` can drift if the moved code is not kept narrowly scoped to the active collect path.
- Missing a collect-time field during bootstrap handoff could break imports, qualification, or module loading in subtle ways.
- Accidentally moving body-lowering concerns into `CollectContext` would blur the phase boundary again.

## Follow-Up Work

- remove or shrink the legacy lower-owned collection helpers once no active paths depend on them
- split dependency bootstrap from `Lowerer`
- add resolver-owned canonical path/alias tables
