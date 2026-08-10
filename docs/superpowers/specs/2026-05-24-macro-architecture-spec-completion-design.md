# Macro Architecture Spec Completion Design

## Goal

Close the remaining gaps between the completed 14-task macro architecture plan and the approved macro expansion architecture spec. The branch should satisfy the broader acceptance criteria for active declarative token-tree matching, source-map-aware expansion origins, richer proc-macro protocol metadata, depth-overflow trace diagnostics, stable verification, and user-visible regression coverage.

## Scope

This delta keeps the existing syntax and compiler phase boundaries. It does not add a package manager, proc-macro build orchestration, WASM execution, attribute/derive syntax, formatter trivia redesign, or full canonical macro identity resolution.

## Declarative Matcher Activation

`DeclarativeMacro::from_ast` already compiles macro definitions into `MacroMatcher` and `MacroTemplate`. Expansion should use those compiled matcher fragments as the active matching boundary instead of passing AST `MacroFragment` definitions into `MacroArgMatcher`.

`MacroArgMatcher` can remain as the matching engine, but its input should be `declarative::MatcherFragment` so the active boundary is the compiled declarative model. Expression parsing must continue to use `MacroExpansionContext.config`, and repetition behavior must preserve current macro tests.

## Source-Map-Aware Token Origins

Template expansion should assign token origins while preserving current user-visible spans:

- Captured invocation tokens become `TokenOrigin::Captured` with the current `ExpansionId`, invocation span, and original capture span.
- Tokens written in macro templates become `TokenOrigin::Generated` with the current `ExpansionId`, generated source identity, and template-definition span.
- Repetition expansion preserves the same origin rules for each repeated token.

The generated parser may still flatten token streams to parser tokens during this migration, but it must receive token streams carrying real origins. Existing expansion trace labels remain the diagnostic bridge until later phases consume origin metadata directly.

## Proc-Macro Protocol Metadata

The process-isolated proc-macro protocol should expose the required metadata without introducing build-system discovery:

- `ProcMacroArtifact` records artifact format version, crate identity string, protocol version, host triple, executable path, declared capabilities, and exports.
- `ProcMacroExport` records name, kind, stable macro identity string, and accepted input shape.
- `ProcMacroRequest` records protocol version, compiler version, macro identity/name, optional source/module identity, optional expansion ID, and encoded input tokens.

Because proc-macro artifacts are serialized into compiler products, adding artifact fields requires incrementing the product artifact format version in both compiler and shared sysroot code.

## Depth Trace Diagnostics

Macro expansion depth overflow should report a structured diagnostic at the best available invocation span and include expansion trace labels from the latest recorded expansion. This replaces the current default-span-only overflow diagnostic while preserving the depth limit behavior.

## Verification Stability And Coverage

The proc-macro shell test fixture should read stdin before writing its response so the runner's request writer does not race with an early-exiting child process.

Coverage should include focused tests for active matcher use, captured/generated token origins, enriched protocol roundtrips, depth-overflow trace labels, and proc-macro request metadata. Add at least one integration-style user-visible macro/proc-macro regression if it can be expressed with the existing integration harness without adding package-management behavior.

## Acceptance Criteria

- No `Config::default()` macro reparse path returns in `lib/src/macro_expansion/**`.
- Declarative expansion uses compiled `MacroMatcher` and `MacroTemplate` structures as the active boundary.
- Template expansion emits `Captured` and `Generated` token origins with expansion and generated-source identities.
- Proc-macro artifacts and requests include the required version, identity, input-shape, context, and capability metadata.
- Depth overflow diagnostics include expansion trace labels.
- The proc-macro smoke test is stable under full `cargo test -p rock-lib` runs.
- Focused and full verification pass: `cargo fmt --all --check`, `git diff --check`, and `cargo test -p rock-lib`.
