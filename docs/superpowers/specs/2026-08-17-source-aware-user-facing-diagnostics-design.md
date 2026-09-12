# Source-Aware User-Facing Diagnostics Design

**Date:** 2026-08-17
**Status:** Implemented
**Scope:** Compiler diagnostics from source loading through code generation, including source ownership, semantic-name rendering, typed diagnostic payloads, CLI reporting, virtual sources, and the shared foundation required by a future Rock language server

## Purpose

Rock diagnostics must describe programs in the language programmers wrote. They must not expose compiler-session identities such as `struct#0::1`, `enum#0::2`, `generic#0::4.1`, raw `DefId` values, `TypeVarId` values, monomorphization records, or debug representations of internal error enums.

Every error caused by Rock source must carry a real source location. The location must identify the source file and byte range that caused the error, including unsaved virtual files and artifact-provided source. Errors that are not caused by a Rock source range, such as malformed manifests, missing artifacts, linker failures, and toolchain failures, must carry an explicit non-source location rather than a fabricated empty `Span`.

The compiler owns this work. The CLI and a future language server must consume the same structured diagnostics, source map, and user-facing type/name formatter. Neither frontend may reconstruct semantic names, parse compiler output, reread source independently, or maintain a parallel diagnostic implementation.

## Current Problems

### Spanless Semantic Errors

`ResolveError` stores `Option<Span>` and exposes a spanless constructor. Most production construction sites use the spanless path. `SpannedError` also returns `Option<Span>`, allowing source-semantic errors to reach the final renderer without a location.

Lowering uses a mutable `current_span` fallback. That span is updated at selected declaration or expression boundaries, but it is not a reliable representation of the operation that produced every diagnostic. A stale declaration span is better than an empty span only by accident; it is not sufficient for precise CLI labels or LSP ranges.

Parser fallback variants, inference finalization, MIR dataflow, and borrow checking contain `Span::default()` fallbacks. An empty path and zero range do not identify a source location and must not silently enter a user-facing diagnostic.

### Internal Type And Identity Text

`Type` is correctly ID-based, but its ordinary Rust `Display` implementation is a structural/debug formatter. Nominal, generic, projection, and constructor types render compiler identities rather than Rock names.

Several source-semantic phases interpolate `Type` directly into strings. Unification returns `String`, so the expected and actual `Type` values are destroyed before the compiler has enough context to render canonical source names. Selection and monomorphization similarly turn structured errors into strings or debug representations too early.

### Incomplete Source Ownership

`Diagnostic` can carry owned source text through `DiagnosticSource`, but source text is mainly attached to source-loading and parser failures. Collection, lowering, inference, monomorphization, MIR, borrow checking, and code generation commonly return diagnostics that rely on later filesystem reads.

That behavior is wrong for virtual editor buffers, artifact-backed source, renamed display paths, and deterministic API consumers. The compiler session already owns the source database and must attach the matching source to every source diagnostic before returning it.

### Missing Definition Source Map

AST identifiers carry spans and HIR expressions carry spans, but most HIR declarations and bindings do not retain definition locations. Canonical semantic IDs therefore cannot always be mapped back to their source declarations.

This prevents high-quality secondary labels such as “`PtrBox` declared here,” and would force a future LSP to recreate definition mapping for hover, go-to-definition, references, and rename.

## Goals

- Require every source-caused compiler diagnostic to have a real primary `Span`.
- Represent file, project, artifact, toolchain, and linker failures explicitly without fake source spans.
- Render all user-facing types with canonical Rock names and Rock syntax.
- Preserve `DefId`, `TypeVarId`, `GenericParamId`, `FieldId`, `VariantId`, and related IDs as semantic authority.
- Keep structured types and identities in error values until final diagnostic rendering.
- Attach source text and source origin to diagnostics before returning them from `rock-lib`.
- Support filesystem, virtual, and artifact-backed sources through one source registry.
- Introduce a compiler-owned semantic source map reusable by diagnostics and future language tooling.
- Support primary labels, secondary labels, notes, help, severity, and stable diagnostic codes.
- Make CLI and future LSP rendering thin adapters over the same compiler data.
- Preserve accepted-HIR and artifact cleanliness; editor/source metadata must not become semantic identity or artifact payload by accident.

## Non-Goals

- Do not implement the LSP server in this work.
- Do not add incremental compilation or parser recovery.
- Do not put display names inside `Type` or use names for type equality.
- Do not replace canonical IDs with source strings.
- Do not serialize current-crate editor source maps into product artifacts.
- Do not promise dependency-definition navigation when an artifact does not contain dependency source metadata.
- Do not give linker or toolchain failures misleading source ranges.
- Do not redesign Ariadne or LSP protocol types into compiler-internal types.

## Core Invariants

### Source Diagnostic Invariant

A diagnostic caused by Rock source has:

- a non-empty source path,
- a valid byte range with `start <= end`,
- a primary label at that range,
- the source origin and text when the compiler session loaded that source,
- only labels whose ranges belong to known sources or explicitly identified external source files.

Zero-width ranges are valid for locations such as end of file, but an empty default span is not a valid substitute for an unknown location.

### Non-Source Diagnostic Invariant

An error not attributable to Rock source uses an explicit location category:

```rust
pub enum DiagnosticLocation {
    Source(Span),
    File(PathBuf),
    Project(PathBuf),
    Artifact(PathBuf),
    Toolchain,
}
```

The CLI may render every category. An LSP adapter publishes `Source` diagnostics to document URIs and reports other categories through workspace or window diagnostics. It must not invent an LSP range for a non-source failure.

### Semantic Identity Invariant

Diagnostic resolution uses canonical IDs first. `ResolverTables`, HIR definition indexes, generic declaration metadata, language-item registries, and dependency interfaces provide names as display views over known IDs.

Names may be used for prose and source syntax. They may not be used to rediscover semantic identity or select definitions.

### Deferred Rendering Invariant

Errors containing types or semantic definitions remain structured until a `DiagnosticRenderer` has access to the compilation diagnostic context.

For example, unification must return a typed value:

```rust
pub enum UnifyError {
    Mismatch { expected: Type, found: Type },
    InfiniteType { variable: TypeVarId, ty: Type },
    KindMismatch { expected: Kind, found: Kind },
    Message(String),
}
```

The fallback `Message` variant is for errors that genuinely contain no semantic values. It is not a compatibility path for formatting types early.

## Diagnostic Data Model

The long-term compiler diagnostic shape is:

```rust
pub struct Diagnostic {
    pub code: Option<DiagnosticCode>,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub location: DiagnosticLocation,
    pub primary: Option<DiagnosticLabel>,
    pub secondary: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub help: Vec<String>,
    pub source: Option<DiagnosticSource>,
}

pub struct DiagnosticLabel {
    pub message: String,
    pub span: Span,
}
```

Migration may retain the existing public `span`, `labels`, and `kind` fields temporarily, but new constructors must enforce the location invariants. Compatibility fields should be removed once all consumers use the structured shape; they must not become a permanent duplicate authority.

Stable diagnostic codes should be introduced by diagnostic family rather than by assigning arbitrary numbers to every existing string. Codes are optional during migration but required before external LSP consumers depend on them.

## Compilation Source Registry

The source loader remains the authority for source text and origin. It should expose a read-only iterator over loaded `SourceFile` values.

`DiagnosticSourceMap` is a compilation-session view:

```rust
pub struct DiagnosticSourceMap {
    by_path: BTreeMap<PathBuf, DiagnosticSource>,
}
```

Each loaded source is indexed by every path form the compiler may place in a span:

- original path,
- canonical path,
- display path.

The map preserves `FileSystem`, `Virtual`, and `Artifact` origins. It can attach source text to an individual diagnostic or a `Diagnostics` collection without filesystem IO.

The compile pipeline creates the source map after source graph loading and attaches it at every later error-return boundary. Source-loader parse errors continue attaching their source immediately because they can occur before a complete graph exists.

## Semantic Source Map

The semantic source map is distinct from the source-text registry. It maps compiler identities and typed syntax back to source locations.

The initial identity model is:

```rust
pub enum SourceSymbol {
    Definition(DefId),
    Local { owner: DefId, local: HirLocalId },
    Field { owner: DefId, field: FieldId },
    Variant { owner: DefId, variant: VariantId },
    AssociatedType { owner: DefId, associated: AssocTypeId },
    Generic(GenericParamId),
}
```

The map records:

- definition name span,
- optional full declaration span,
- reference spans and resolved targets,
- expression spans,
- explicit type annotation spans,
- lexical owner and scope relationships needed for local diagnostics and future completion.

Collection records module and top-level definition spans while `ItemSourceId` and AST identifiers are available. Lowering records parameter, pattern binding, local, field, variant, method, and reference spans when it assigns their canonical IDs.

The map flows through `Declarations`, `PartialHir`, and `ResolvedHirProgram`. It is session metadata, not accepted-HIR semantic authority, and is omitted from product artifact serialization.

## User-Facing Name And Type Rendering

The current structural `Display for Type` remains available for debug logs and invariant failures. User-facing diagnostics must use a context-aware formatter:

```rust
pub struct TypeDisplayContext {
    definitions: HashMap<DefId, String>,
    generics: HashMap<GenericParamId, String>,
    associated_types: HashMap<AssociatedTypeKey, String>,
}

pub fn display_type_with_context<'a>(
    ty: &'a Type,
    context: &'a TypeDisplayContext,
) -> UserTypeDisplay<'a>;
```

Definition names come from canonical resolver tables and dependency interfaces. Generic names come from the declaration that owns the `GenericParamId`. Associated type names come from the canonical trait or impl declaration.

The formatter emits Rock syntax:

- `PtrBox [I64]`, not `struct#0::1<[I64]>`,
- `Option I64`, not `enum#1::7<I64>`,
- `Result I64, IoError`,
- `T`, not `generic#0::4.0`,
- `<T as Iterator>::Item` or the accepted Rock projection spelling, not raw owner IDs,
- `_` or “unknown type” prose for unresolved inference variables in diagnostics, not `?T17`.

Unknown IDs may use a neutral placeholder in user diagnostics and may retain the structural ID only in debug notes. A source diagnostic must never expose an internal ID merely because metadata is incomplete.

The same formatter is the future authority for hover text, inlay hints, signature help, and completion details.

## Structured Phase Errors

### Lexer And Parser

Lexer and parser errors retain token or character spans. End-of-file errors receive a zero-width span at the actual source byte length. Indentation errors receive the indentation range. Parser combinator control-flow variants such as `Fail`, `ShortCircuit`, and `AssertFailed` must not escape as user diagnostics; if they do, they become internal compiler errors with an explicit project/file location.

### Collection And Lowering

`ResolveError` is split or migrated so source-semantic construction requires a `Span`. Collection uses AST name/type/path spans. Lowering passes the current operation span explicitly instead of relying on a sticky ambient fallback.

Source diagnostics use the exact syntax or operation span that caused the error. A declaration span may be used only when the declaration itself is invalid; it must never substitute for a missing expression, constraint, or obligation span.

### Inference And Selection

Unification, constraint solving, selection, and finalization return structured errors containing semantic types and IDs. Constraint errors use the span already stored on each constraint. Authority obligations use their recorded expression span. Unresolved type variables use `InferenceEngine::var_spans`.

`fresh_type_var_at` and kind-aware equivalents become the production path whenever a variable corresponds to source syntax. Synthetic inference variables either inherit the originating source operation or are linked to a constraint that owns a real span.

### Monomorphization

`MonoErrorKind` remains structured. Its renderer resolves definitions and types through compilation identity, resolver, and type-display contexts. User diagnostics do not use `Debug` formatting for the error kind.

HIR expression spans continue through specialization so monomorphization errors identify the call, method, or value that required the invalid instance.

### MIR And Borrow Checking

MIR statements, locals, closure captures, calls, and terminators retain source origins from HIR. Borrow-check diagnostics use those origins for primary and secondary labels. Missing MIR origins are internal invariant failures, not a reason to emit default source spans.

### Code Generation And Linking

Code generation errors attributable to a MIR operation use that operation's source origin. LLVM API failures include the source operation as primary and LLVM detail as a note.

Backend contract corruption and impossible missing-layout/identity cases are internal compiler errors with project or artifact locations. Object writing and linking failures use file/toolchain locations. They do not pretend to be source type errors.

## CLI And LSP Consumption

`rockc` continues to call `Diagnostics::report`, but report rendering consumes the structured location, labels, notes, help, and attached source. Filesystem reads are a fallback only for diagnostics created outside a compilation source registry.

The future LSP adapter performs only protocol conversion:

- source path to document URI,
- byte range to UTF-16 LSP range,
- compiler severity/code/message to LSP fields,
- secondary labels to related diagnostic information.

It does not format types, resolve IDs, load files, or infer source ownership.

## Implementation Plan

### Phase 1: Source Ownership Foundation

1. Expose loaded source iteration from `SourceDatabase`.
2. Add `DiagnosticSourceMap` keyed by original, canonical, and display paths.
3. Add source attachment APIs on `Diagnostic` and `Diagnostics`.
4. Attach sources at compile phase error boundaries.
5. Add virtual and artifact source regression tests.

### Phase 2: User-Facing Type Display

1. Add `TypeDisplayContext` and `UserTypeDisplay` without changing `Type`.
2. Populate canonical nominal names from `ResolverTables`.
3. Populate generic and associated type names from HIR declarations.
4. Implement Rock-syntax formatting and neutral unknown fallbacks.
5. Replace the known `struct#...` integration diagnostic and add nested type coverage.

### Phase 3: Structured Frontend Errors

1. Introduce `UnifyError` and migrate inference-engine string errors.
2. Preserve constraint and type-variable origin spans.
3. Add context-aware rendering for `SelectionDiagnostic`.
4. Migrate `ResolveError` so source errors require spans.
5. Replace ambient `current_span` diagnostic calls with explicit operation spans.
6. Remove parser default-span escape paths.

### Phase 4: Semantic Source Map

1. Record top-level definition spans during collection.
2. Record child definition and generic spans.
3. Record local definitions and resolved references during lowering.
4. Carry the map through partial and resolved HIR products.
5. Add definition/reference lookup tests across modules and shadowed locals.

### Phase 5: Downstream Diagnostics

1. Render monomorphization error kinds through diagnostic context.
2. Preserve source origins through MIR construction.
3. Remove default span fallbacks from borrow checking and MIR dataflow.
4. Attach operation origins to source-caused codegen errors.
5. Classify project, artifact, output, linker, and toolchain errors explicitly.

### Phase 6: Enforcement And Public Contract

1. Introduce stable diagnostic codes by family.
2. Make invalid source-diagnostic construction impossible through public APIs.
3. Add debug validation for paths and ranges.
4. Document byte-offset and virtual-source guarantees for external consumers.
5. Add end-to-end CLI tests and protocol-neutral serialization tests suitable for a future LSP adapter.
6. Remove `Default` from `Span` and eliminate repository-wide default-span construction and fallback unwrapping.
7. Require test fixtures to use explicit synthetic source identities rather than empty/default spans.

## Testing Strategy

### Source Registry Tests

- Filesystem sources attach canonical text and display paths.
- Virtual sources attach unsaved text without disk reads.
- Artifact sources preserve artifact origin and display path.
- Multi-file diagnostics attach the correct source to every label.

### Type Display Tests

- Local and dependency structs/enums use canonical Rock names.
- Nested generics, tuples, functions, references, arrays, and projections use Rock syntax.
- Generic parameters use declaration names.
- Unresolved variables use neutral user-facing text.
- Debug `Type::Display` remains structurally useful and semantic equality is unchanged.

### Span Tests

- Parse EOF points to the actual end of the correct source.
- Type annotation mismatches point to the annotation or value expression.
- Operator and method selection failures point to the call/operator expression.
- Constraint solver errors retain their originating constraint span.
- Ambiguous inference points to the type variable's source origin.
- Mono and borrow errors retain the original HIR expression span.

### End-To-End Regressions

- The generic `PtrBox` operator failure reports `PtrBox [I64]` and never `struct#...`.
- No source diagnostic contains `struct#`, `enum#`, `generic#`, `trait#`, raw `DefId`, or `?T` text.
- Every source diagnostic returned by `rock_lib::compile` has a valid source location and attached source when the compiler loaded that file.
- Non-source failures are explicitly categorized and carry no fabricated source range.
- The Rust workspace contains no `Span::default()`, inferred default `Span`, or fallback that substitutes an unrelated declaration/source span.

Behavioral tests are the durable enforcement mechanism. Source-content residue searches may be used during migration, but they are not a substitute for semantic regression tests.

## Success Criteria

- Compiler users see only source-level names and Rock type syntax.
- Every source-caused diagnostic has a valid primary span and source identity.
- `Span` cannot be default-constructed; missing source origins remain optional or become explicit non-source diagnostic locations.
- Virtual-buffer diagnostics require no CLI or LSP-specific source recovery.
- Unification, selection, mono, and downstream errors retain semantic values until final rendering.
- `Type`, HIR, MIR, and artifacts remain canonically ID-based.
- CLI diagnostic output and future LSP diagnostics share the same compiler-produced data.
- The semantic source map is sufficient for later hover, go-to-definition, references, and rename without rebuilding identity-to-source mappings in the LSP.

## Risks

- Replacing string errors with structured enums touches many callers and tests; migration must remain phase-local and serialized.
- Resolver tables may not contain every dependency child display name needed for rich projection output. Missing metadata must produce neutral output, not internal IDs.
- Synthetic inference variables and MIR cleanup statements may lack direct syntax origins. They must inherit an originating operation or be classified as internal errors.
- Attaching complete virtual source text increases diagnostic result size. The compiler should share source storage internally where possible before an LSP introduces frequent snapshots.
- Exact diagnostic-string tests may encode current internal text. Update them to assert user behavior, locations, labels, and stable codes rather than debug representations.

## LSP Consequences

This work substantially reduces future LSP scope. The LSP will reuse:

- `DiagnosticSourceMap` for open and dependency source ownership,
- semantic source mappings for definitions and references,
- `UserTypeDisplay` for hover, inlay hints, signatures, and completion details,
- structured diagnostics for direct protocol conversion,
- stable diagnostic codes for client filtering and code actions.

The LSP will still need document synchronization, UTF-16 position conversion, project lifecycle, cancellation, incremental analysis, and incomplete-buffer recovery. It will not need a second diagnostic engine or name formatter.

## Completion Evidence

Implemented on 2026-08-18. The completed compiler contract includes structured diagnostic families, validated source and non-source locations, attached filesystem/virtual/artifact sources, contextual Rock type rendering, structured inference/selection/mono/codegen errors, a session-only semantic source map, MIR operation origins, protocol-neutral serialization, and CLI coverage. Repository enforcement removes default or fabricated production spans and keeps editor source metadata out of product artifacts.

Verification at completion:

- `cargo test -p rock-lib --lib`: 2,316 passed.
- `cargo test -p rock-lib --test integration`: 626 passed.
- `cargo test -p rock-lib --test test_parse_struct_with_fields`: 1 passed.
- `cargo test -p rockc`: 46 passed across unit and CLI integration suites.
