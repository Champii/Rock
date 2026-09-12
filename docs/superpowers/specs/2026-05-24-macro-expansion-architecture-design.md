# Macro Expansion Architecture Design

## Goal

Replace the current parser-coupled macro expansion path with a production-grade macro architecture that supports both declarative macros and process-isolated proc macros. Macro expansion should become an explicit frontend phase that consumes parser output and token streams, produces expanded AST plus source-map metadata, preserves precise diagnostics, and never reparses generated fragments with implicit `Config::default()` state.

## Current Problems

- `lib/src/macro_expansion/mod.rs` expands declarative macro bodies by replacing fragments, flattening tokens, and reparsing through `module_inline.process(ParseCtx::from(&body, &Config::default()))`.
- Macro expansion has no explicit context for parser configuration, current source identity, expansion stack, generated source identity, or diagnostics policy.
- Generated tokens keep only ordinary lexer spans, so diagnostics cannot reliably explain both the generated code location and the macro invocation/definition that produced it.
- Expansion depth overflow panics instead of reporting a structured diagnostic.
- Declarative macros are not represented by a stable token-tree matcher/expander boundary, which makes hygiene, imports, and proc macros hard to layer in safely.
- There is no compiler-owned proc-macro crate kind, host/target distinction, plugin protocol, source-map integration, or sandbox/trust model.

## Scope

In scope:

- Declarative macro rework around compiler-owned token trees, matchers, repetitions, captures, expansion context, hygiene, and source maps.
- Proc macros as a first-class production target, not a vague future extension.
- A process-isolated host-binary proc-macro execution model with a versioned token-stream protocol.
- Macro visibility/import/export rules that fit the existing module, product artifact, and source-loader architecture.
- Structured macro diagnostics with expansion traces.
- Staged implementation slices that keep the compiler usable throughout the redesign.

Out of scope for the initial architecture spec:

- Implementing an unrestricted package manager or build system for third-party macro crates.
- WASM proc-macro execution. The protocol should be compatible with adding a WASM runner later, but the target model is host binaries.
- Full Rust-compatible macro semantics. Rock should define its own minimal, stable macro semantics.
- Formatter/trivia redesign beyond preserving enough token/source origin data for future formatter work.

## Architecture

Add a `macro_expansion` frontend boundary with these concepts:

- `TokenTree`: compiler-owned token tree with delimiters, leaves, and stable token kinds. It should not expose parser-internal `ParseCtx` or ad hoc flattened token vectors as the macro ABI.
- `TokenStream`: ordered token-tree stream used by declarative macro bodies, proc-macro inputs/outputs, generated parser fragments, diagnostics, and source maps.
- `TokenOrigin`: origin metadata for every token tree node. It records the original source span, macro definition span, invocation span, matched capture span, and generated expansion ID when applicable.
- `MacroExpansionContext`: explicit phase state containing `Config`, source/module identity, macro registry, expansion stack, generated source IDs, diagnostics, recursion/depth limits, and source-map builder.
- `MacroRegistry`: registry of available declarative macros and proc macros keyed by resolved macro identity. It should separate display names from semantic IDs once macro declarations receive canonical IDs.
- `ExpansionResult`: expanded AST/module plus source-map metadata and diagnostics. Later phases should be able to report a diagnostic at generated code and attach an expansion trace.

The expansion phase should sit after parsing source modules and macro discovery, but before semantic collection/lowering. The parser remains responsible for parsing source text or generated token streams into AST. Macro expansion owns macro lookup, token-tree matching, token generation, proc-macro invocation, and expansion source mapping. Collection and lowering consume expanded AST plus source-map data; they should not perform macro lookup or parse generated source themselves.

## Declarative Macros

Declarative macros should be reworked before or alongside proc macros because proc macros need the same token-stream and source-map foundation.

Declarative macro declarations should lower into an internal form:

- `DeclarativeMacro`: macro identity, display name, definition module/source, match arms, and exported visibility.
- `MacroMatcher`: token-tree pattern with named captures, capture kinds, repetition groups, separators, and nesting metadata.
- `MacroTemplate`: token-tree template with capture references, repetition expansion, and generated-token origin rules.
- `CaptureSet`: matched token streams keyed by capture identity and repetition depth.

Matching should operate on token trees, not source strings. Repetition expansion should validate repetition lengths, nesting shape, missing captures, and separator behavior with structured diagnostics.

Hygiene should be explicit:

- Captured identifiers keep call-site origin and resolve at the invocation site.
- Identifiers written in macro definitions default to definition-site origin for helper references.
- Generated identifiers carry generated origin and must opt into call-site or definition-site resolution through explicit marker policy if the language later adds one.
- Source names remain diagnostic metadata; resolver integration should use macro/module identity where available.

The first implementation can keep existing Rock macro syntax if possible, but it should compile macro declarations into the new matcher/template model and delete the parser-internal `Config::default()` reparse path.

## Proc Macros

Proc macros are a first-class target of the redesign.

Rock proc macros should be compiled as host executables, not loaded as dynamic libraries into the compiler process. The compiler invokes a proc-macro host binary with a versioned request/response protocol over stdin/stdout.

The proc-macro artifact model should include:

- Crate identity and artifact format version.
- Proc-macro protocol version.
- Host platform triple used to build the macro executable.
- Path or artifact reference for the host executable.
- Exported macro list with kind, name, identity, and accepted input shape.
- Declared capabilities, such as filesystem/network access if such capabilities are ever allowed.

The initial proc-macro kinds should be deliberately small:

- Function-like proc macros: `name!(tokens)` or the Rock equivalent, producing a token stream.
- Attribute-like and derive-like macros should be designed in the protocol but can be implemented after function-like macros if they require AST attribute syntax that is not yet stable.

The host protocol should be stable and explicit:

- Request: protocol version, compiler version, macro identity, source/module identity, invocation token stream, expansion context metadata, and optional environment/capability data.
- Response: success token stream plus token origins, or structured diagnostics with spans/origins and optional notes.
- Failure: nonzero exit, timeout, malformed response, protocol mismatch, or denied capability becomes a structured compiler diagnostic.

Proc macros must run with bounded inputs and outputs, timeout control, and process isolation. The compiler should not trust proc-macro code to be deterministic, safe, or well-behaved. The design should allow caching deterministic responses later, but only after inputs, environment, macro binary identity, protocol version, and capability state are all part of the cache key.

## Macro Visibility And Artifacts

Macro names should eventually participate in the same canonical identity architecture as functions, types, and traits.

The design should add macro declaration metadata through a frontend macro-discovery boundary first, then through collection/product boundaries in staged form:

- Current-crate declarative macros are discovered from parsed modules by a macro registry builder before expansion of uses that need them.
- Exported declarative macro metadata can be serialized into product artifacts once macro imports from dependencies are supported.
- Proc-macro artifacts carry only the stable plugin interface and exported macro metadata, not arbitrary compiler internals.
- Macro lookup should use module/import/export/prelude rules, but the expanded AST should not preserve macro names as semantic dependencies after expansion.

The first implementation slices can keep macro lookup source-name based inside the macro registry if canonical macro IDs are not yet available, but the architecture should treat that as compatibility only.

## Source Mapping And Diagnostics

Every expansion should allocate an `ExpansionId` and record:

- macro identity and display name;
- invocation span;
- definition span;
- parent expansion, if nested;
- generated token stream/source identity;
- capture origins for tokens substituted from invocation arguments.

Diagnostics should be able to render:

- the primary generated-code span;
- a note pointing to the macro invocation;
- a note pointing to the macro definition or template fragment;
- nested expansion trace when useful.

Macro expansion depth overflow should produce a diagnostic with the expansion stack instead of panicking. Proc-macro execution failures should include macro name, executable/artifact identity, protocol status, and any diagnostics returned by the macro process.

## Parser Boundary

Generated fragments should be parsed through public parser APIs that accept a `TokenStream` or generated source identity plus explicit `Config`. They should not construct `ParseCtx` directly from macro internals.

The long-term preferred boundary is token-stream parsing. If the parser needs an intermediate generated-source string during migration, that path must still be explicit:

- generated source has a synthetic source identity;
- every generated token is mapped back to `TokenOrigin`;
- `Config` comes from `MacroExpansionContext`;
- parser diagnostics are remapped through expansion source maps.

## Staged Implementation

This is a large rework and must remain in the `macro-architecture-redesign` worktree/branch or another explicitly isolated worktree.

Recommended slices:

1. Macro context and diagnostics foundation: add `MacroExpansionContext`, remove `Config::default()`, replace depth panic with diagnostics, and keep behavior otherwise unchanged.
2. Token origin and expansion source-map model: add `ExpansionId`, `TokenOrigin`, generated source metadata, and diagnostic remapping tests.
3. Declarative token-tree model: compile existing macro syntax into `MacroMatcher` and `MacroTemplate`, then expand without ad hoc flattened body reparsing.
4. Declarative hygiene and registry: define call-site/definition-site origin behavior and macro registry visibility rules for current-crate modules.
5. Generated parser boundary: parse generated token streams or explicit generated sources through public parser APIs with source-map remapping.
6. Proc-macro artifact and protocol types: define host executable metadata, request/response schema, protocol versioning, and error handling without executing untrusted macros yet.
7. Proc-macro host execution: invoke host binaries with bounded IO, timeout, protocol validation, and structured diagnostics.
8. Proc-macro integration: support function-like proc macros end-to-end, then add attribute-like or derive-like forms only after syntax and AST placement are explicit.
9. Artifact/import integration: serialize exported macro metadata and proc-macro artifact references through product artifacts.
10. Cleanup: delete compatibility parser-internal macro reparse paths and update roadmap/audit status.

Each slice should use TDD, focused parser/macro unit tests, and at least one integration-style compile test when user-visible behavior changes.

## Verification Strategy

Focused tests:

- Declarative matcher captures identifiers, expressions, types, nested repetitions, and separators.
- Declarative expansion preserves capture origins and generated-token origins.
- Hygiene tests distinguish call-site captured identifiers from definition-site helper identifiers.
- Macro expansion diagnostics include invocation and definition notes.
- Expansion depth overflow reports a diagnostic instead of panicking.
- Generated parser errors remap through expansion source maps.
- Proc-macro protocol rejects version mismatch, malformed response, timeout, excessive output, and nonzero exit.
- Function-like proc macro smoke test invokes a host binary fixture and expands returned tokens.

Regression tests:

- Existing macro expansion tests continue to pass unless intentionally updated for improved diagnostics.
- Multi-module macro visibility behaves consistently with import/export rules.
- `cargo test -p rock-lib` stays green after each implementation slice.

Final verification:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

## Risks

- Proc macros introduce arbitrary compile-time code execution. Process isolation and capability declarations reduce risk but do not make untrusted macros safe by default.
- Hygiene semantics can change user-visible macro behavior. The implementation should introduce tests before changing existing expansion rules.
- Token-stream parser integration may uncover parser assumptions about file-backed lexing and indentation. The generated parser boundary should be a dedicated slice.
- Product artifact integration can expand scope quickly. Keep proc-macro artifact metadata minimal until the host protocol is stable.

## Acceptance Criteria

- Macro expansion no longer reparses generated fragments with `Config::default()` or parser-internal context hidden inside `macro_expansion`.
- Declarative macros are represented by explicit matcher/template/token-tree data structures with source-map-aware expansion.
- Macro diagnostics can report generated locations with invocation and definition context.
- Expansion depth and proc-macro failures are structured diagnostics, not panics or opaque errors.
- Proc macros have a versioned process-isolated host-binary protocol and artifact model.
- Initial function-like proc macro expansion works end-to-end through the same token-stream/source-map infrastructure as declarative macros.
- Existing macro behavior is preserved where compatible, and intentional behavior changes are covered by tests.
