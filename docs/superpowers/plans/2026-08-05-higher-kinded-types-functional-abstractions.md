# Higher-Kinded Types And Eager Functional Abstractions Implementation Plan

## Status

Completed on branch `feature/higher-kinded-types` on 2026-08-08. Tasks 1-14,
the acceptance audit, closure gates, and independent reviews are complete.
Strict workspace Clippy baseline cleanup is deferred to priority-1 task
`new_lang2-n2e`; no lint suppression was introduced.

## Goal

Implement the approved HKT design in
`docs/superpowers/specs/2026-08-05-higher-kinded-types-functional-abstractions-design.md`:

- kinded generic parameters;
- constructor application, explicit type lambdas, and type sections;
- decidable higher-kinded inference;
- constructor-level traits, supertraits, orphan checks, and coherence;
- canonical constructor substitutions through mono and artifacts;
- ordinary stdlib Functor, Applicative, Monad, Foldable, and Traversable;
- eager Vec transformation and traversal;
- a functional refactor of `test_projects/new_new/main.rk` where appropriate.

## Architecture

Kinds and constructor terms remain compile-time semantic data. Collection owns
generic declaration identity and kinds. A shared type normalizer owns alias
expansion and alpha/beta/eta canonicalization. Inference performs only
kind-correct constructor-pattern unification. Constructor impl selection is
static and ID-based. Monomorphization substitutes every constructor before MIR.
Products serialize portable kinded terms; LLVM receives only concrete runtime
types.

## Execution Policy

- Work on `feature/higher-kinded-types`.
- Track execution in beads epic `new_lang2-gdt`, created from specification
  issue `new_lang2-ds4`; child issues `new_lang2-gdt.1` through
  `new_lang2-gdt.14` mirror the dependency graph below. This document is the
  execution contract, not a second status tracker.
- Keep each numbered task compiling and independently reviewable.
- Use TDD: run the named RED test, implement the minimum complete semantic
  slice, then run the named GREEN and regression gates.
- Run test commands serially. Save one log for broad suites rather than
  rerunning them to inspect failures.
- Do not preserve deprecated internal representations or artifact compatibility
  shims.
- Do not stage, commit, amend, push, or otherwise mutate Git state unless the
  current user prompt explicitly requests it.
- Do not touch unrelated untracked files or `.sisyphus/`.

## Dependency Graph

```text
Task 1: shared type traversal
  -> Task 2: kinds and generic descriptors
  -> Task 3: parser/AST/formatter/tree-sitter
  -> Task 4: constructor terms and normalization
  -> Task 5: kind lowering and phase validation
  -> Task 6: kind-aware inference
  -> Task 7: typed predicates, supertraits, associated constructors
  -> Task 8: constructor impls, orphan rules, overlap
  -> Task 9: constructor selection and lowering authority
  -> Task 10: TypeContext, mono, MIR barrier
  -> Task 11: complete artifacts and cross-crate HKT
  -> Task 12: stdlib type-class hierarchy
  -> Task 13: eager collection operations
  -> Task 14: new_new migration and closure gates
```

Tasks 3 and the non-semantic portions of Task 2 may be developed in parallel,
but they must merge before Task 4. All other tasks are ordered because they
change shared semantic representations.

## Planned File Surface

### New Compiler Modules

- `lib/src/type_services/visit.rs`
  - Central immutable/mutable type traversal and folding.
- `lib/src/type_services/kind.rs`
  - Kind queries and declaration-kind environment.
- `lib/src/type_services/normalize.rs`
  - Alias expansion, application flattening, beta/eta reduction, saturation,
    and canonical-form validation.
- `lib/src/traits/coherence.rs`
  - Shared orphan and overlap checking for local and artifact impl headers.

Exact module names may change to match existing ownership, but these
responsibilities must not be duplicated across phases.

### Primary Existing Compiler Files

- Parser and syntax:
  - `lib/src/lexer/token.rs`
  - `lib/src/lexer/lexer.rs`
  - `lib/src/parser/engine/token_type.rs`
  - `lib/src/parser/items/parse_type.rs`
  - `lib/src/parser/items/top_level.rs`
  - `lib/src/parser/items/trait.rs`
  - `lib/src/parser/items/impl.rs`
  - `lib/src/parser/items/function_sig.rs` or the current signature parser
  - `lib/src/ast/tree.rs`
  - `lib/src/ast/visit.rs`
  - `lib/src/fmt/mod.rs`
  - `lib/src/fmt/decl.rs`
  - `tree-sitter-rock/grammar.js`
  - `tree-sitter-rock/test/corpus/**`
- Semantic types and lowering:
  - `lib/src/types/mod.rs`
  - `lib/src/type_context/mod.rs`
  - `lib/src/type_context/view.rs`
  - `lib/src/type_lowering.rs`
  - `lib/src/type_services/display.rs`
  - `lib/src/type_services/facts.rs`
  - `lib/src/type_services/layout.rs`
  - `lib/src/type_services/projection.rs`
  - `lib/src/collect/context.rs`
  - `lib/src/collect/headers.rs`
  - `lib/src/collect/item_index.rs`
  - `lib/src/lower/types.rs`
  - `lib/src/lower/function.rs`
  - `lib/src/lower/body_context.rs`
  - `lib/src/lower/traits/conformance.rs`
- HIR and inference:
  - `lib/src/hir/mod.rs`
  - `lib/src/hir/accepted.rs`
  - `lib/src/hir/type_ids.rs`
  - `lib/src/infer/engine.rs`
  - `lib/src/infer/constraints.rs`
  - `lib/src/infer/solve.rs`
  - `lib/src/infer/generalize.rs`
  - `lib/src/infer/type_vars.rs`
  - `lib/src/infer/finalize.rs`
  - `lib/src/infer/authority.rs`
- Selection, mono, and MIR:
  - `lib/src/selection/matching.rs`
  - `lib/src/selection/service.rs`
  - `lib/src/selection/types.rs`
  - `lib/src/lower/paths.rs`
  - `lib/src/lower/control_flow/secondary.rs`
  - `lib/src/mono/mod.rs`
  - `lib/src/mono/specialize.rs`
  - `lib/src/mono/substitute.rs`
  - `lib/src/mono/process.rs`
  - `lib/src/mono/methods.rs`
  - `lib/src/mono/registry.rs`
  - `lib/src/mir/agreement.rs`
  - `lib/src/mir/builder/mod.rs`
  - `lib/src/mir/backend_contract.rs`
  - `lib/src/codegen/types.rs`
- Products and dependencies:
  - `lib/src/products.rs`
  - `lib/src/products/type_table.rs`
  - `lib/src/crate_artifact/load.rs`
  - `lib/src/crate_artifact/types.rs`
  - `lib/src/crate_artifact/tests.rs`
  - `lib/src/crate_system/extern_store.rs`
  - `rock-shared/src/sysroot.rs`
- Tests and stdlib:
  - `lib/tests/integration.rs`
  - co-located unit tests in every changed subsystem
  - `stdlib/lib.rk`
  - `stdlib/prelude.rk`
  - `stdlib/callable.rk`
  - `stdlib/option.rk`
  - `stdlib/result.rk`
  - `stdlib/vec.rk`
  - new functional trait modules under `stdlib/`
  - `test_projects/new_new/main.rk`

Before each task, refresh this inventory with CodeGraph and inspect every new
exhaustive `Type`, generic metadata, impl header, and artifact match introduced
since this plan was written.

## Task 1: Centralize Recursive Type Traversal

### Purpose

Reduce the risk of silently omitting HKT variants from the compiler's many
manual `Type` walkers before adding those variants. This task must not change
source language behavior.

### Files

- Create: `lib/src/type_services/visit.rs`
- Modify: `lib/src/type_services/mod.rs`
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/type_context/mod.rs`
- Modify: `lib/src/infer/engine.rs`
- Modify: `lib/src/infer/generalize.rs`
- Modify: `lib/src/infer/type_vars.rs`
- Modify: `lib/src/infer/finalize.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/hir/type_ids.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/bodies.rs`
- Modify: `lib/src/lower/types_helpers/type_vars.rs`
- Modify: `lib/src/type_services/facts.rs`
- Modify: `lib/src/type_services/layout.rs`
- Modify: `lib/src/selection/matching.rs`
- Modify: `lib/src/mono/specialize.rs`
- Modify: `lib/src/mono/substitute.rs`
- Modify: product/DefId remapping walkers in `lib/src/products.rs`
- Modify: artifact remapping walkers in `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/products/type_table.rs`

### RED Tests

Add co-located tests for a synthetic deeply nested existing type containing:

- nominal arguments;
- function params, return, and captures;
- references and pointers;
- projections and trait args;
- generic and inference variables.

Tests must prove immutable visiting, mutable folding, generic collection,
inference-variable collection, and DefId remapping visit every nested location
exactly once.

Run:

```bash
cargo test -p rock-lib type_services::visit -- --nocapture
```

Expected RED: the shared visitor/folder APIs do not exist.

### Implementation

Add syntax-neutral traversal traits or functions with these capabilities:

```rust
visit_type(&Type, &mut impl TypeVisitor)
fold_type(Type, &mut impl TypeFolder) -> Type
try_fold_type(Type, &mut impl TryTypeFolder) -> Result<Type, E>
```

The implementation must distinguish binders from ordinary children once Task 4
adds lambdas. Do not encode lambda-unaware substitution assumptions in this
initial API.

Migrate mechanical recursive operations first:

- collect generics and type vars;
- apply substitutions;
- finalize inference variables;
- remap DefIds;
- collect HIR type IDs;
- artifact type row recursion.

The migration inventory must include `collect/headers.rs` generic collection,
`lower/bodies.rs` impl-generic rehoming, every HIR substitution helper in
`hir/mod.rs`, products/artifact remapping, and runtime fact/layout classifiers.
Do not treat the file list as permission to leave another mechanical walker on
an exhaustive wildcard.

Keep shape-sensitive operations such as layout, unification variance, callable
ABI, and impl matching explicit.

### GREEN Gates

```bash
cargo test -p rock-lib type_services::visit -- --nocapture
cargo test -p rock-lib infer:: -- --nocapture
cargo test -p rock-lib hir::type_ids -- --nocapture
cargo test -p rock-lib mono::specialize -- --nocapture
cargo test -p rock-lib products::type_table -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 2: Add Kinds And Canonical Generic Descriptors End To End

### Purpose

Replace semantic `Vec<String>` generic metadata with owner-scoped descriptors.
All existing generic parameters initially have kind `Type`, preserving current
behavior while establishing one end-to-end kind authority.

### Files

- Create: `lib/src/type_services/kind.rs`
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/body_context.rs`
- Modify: `lib/src/infer/generalize.rs`
- Modify: `lib/src/products.rs`
- Modify: `lib/src/products/type_table.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/type_context/mod.rs`
- Modify: fixture constructors throughout `lib/src/**`

### RED Tests

Add tests proving:

- a generic function's parameter name, owner, index, and kind survive
  collection through accepted HIR;
- structs, enums, traits, signatures, impl generics, and methods preserve the
  same metadata;
- same-spelled generic parameters with different owners remain distinct;
- product artifact generic descriptors roundtrip with product-local owner IDs.

Run:

```bash
cargo test -p rock-lib generic_param_descriptor -- --nocapture
```

Expected RED: declarations expose strings and IDs in parallel rather than one
descriptor.

### Implementation

Add `Kind` and `GenericParamDecl`. Replace paired fields such as:

```rust
generic_params: Vec<String>
generic_param_ids: Vec<GenericParamId>
```

with one ordered descriptor vector. Migrate `HirFunction`, `HirStruct`,
`HirEnum`, `HirTrait`, `HirFunctionSig`, impl metadata, type information,
product interfaces, serialized DTOs, and tests.

Use the declaration descriptor as the only kind lookup authority. Do not add a
parallel `HashMap<String, Kind>` semantic path. Local source contexts may keep
temporary name-to-ID scopes while resolving names.

Increment the product artifact version from 43 to 44 in both constants as soon
as kind metadata becomes serialized. Format 43 must be rejected before full
decode. Later tasks extend format 44 with constructor rows; no intermediate
format is released from this feature branch.

### GREEN Gates

```bash
cargo test -p rock-lib collect:: -- --nocapture
cargo test -p rock-lib hir:: -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact
cargo fmt --all --check
git diff --check
```

## Task 3: Parse And Format HKT Surface Syntax

### Purpose

Represent kinded binders, constructor applications, explicit type lambdas, and
type sections without assigning semantic meaning in the parser.

### Files

- Modify: `lib/src/lexer/token.rs`
- Modify: `lib/src/lexer/lexer.rs`
- Modify: `lib/src/parser/engine/token_type.rs`
- Modify: `lib/src/parser/items/parse_type.rs`
- Modify: `lib/src/parser/items/top_level.rs`
- Modify: `lib/src/parser/items/trait.rs`
- Modify: `lib/src/parser/items/impl.rs`
- Modify: current function-signature and where-clause parsers
- Modify: `lib/src/ast/tree.rs`
- Modify: `lib/src/ast/visit.rs`
- Modify: `lib/src/fmt/mod.rs`
- Modify: `lib/src/fmt/decl.rs`
- Modify: parser test modules under `lib/src/parser/items/tests/**`
- Modify: `tree-sitter-rock/grammar.js`
- Modify: `tree-sitter-rock/test/corpus/**`

### RED Tests

Add parser and formatter tests for:

```rock
trait Functor for F _
trait Bifunctor for F _, _
struct Compose (F _), (G _), A
type ResultWith E = \T -> Result T, E
impl Functor for Result _, E
fmap: M -> F A -> F B where M: FnMut A, B
apply_f: F A -> A where F _: Functor
constructor_identity: F A -> F A where F _
identity: T -> T
(Result _, IoError)::Applicative::pure 42
```

Include nested lambdas, multiple holes, explicit higher-order kinds, malformed
holes, missing lambda bodies, and application/function/lambda precedence.
Add tests proving implicit uppercase ordinary generics retain current behavior,
`F _, _` has ordinary application associativity, parentheses delimit adjacent
constructor declaration parameters, and higher kinds are never guessed from an
otherwise invalid application.

Run:

```bash
cargo test -p rock-lib parser::items::tests -- --nocapture
cargo test -p rock-lib fmt:: -- --nocapture
```

Expected RED: the new syntax is rejected or collapsed into ambiguous
`ParseTypeInner` shapes.

### Implementation

Introduce dedicated AST nodes for generic declarations, application, lambda,
and hole. Preserve source spans on every binder and hole. Do not infer kinds or
resolve names in parser code.

Preserve implicit ordinary generic discovery for function/member signatures and
impl headers. Add explicit constructor binder forms only in declaration and
kind-declaring bound-subject contexts. Allocate all binder IDs in deterministic
source discovery order.

Lex `\` as a dedicated type-lambda introducer. Parse `F _` and `F _, _` with
ordinary left-associative, space-separated type application. In declaration
parameter lists, parentheses delimit adjacent constructor parameters, as in
`(F _), (G _), A`; they do not define constructor arity. Support both bounded
`where F _: Functor` and unbounded `where F _` constructor declarations. Emit a
kind diagnostic rather than inferring a higher kind when an implicit ordinary
generic is applied as a constructor.

Update tree-sitter in the same task so editor and compiler grammars do not
diverge.

### GREEN Gates

```bash
cargo test -p rock-lib parser::items::tests -- --nocapture
cargo test -p rock-lib fmt:: -- --nocapture
cargo fmt --all --check
git diff --check
```

From `tree-sitter-rock/`:

```bash
tree-sitter generate
tree-sitter test
```

## Task 4: Add Constructor Terms And Canonical Normalization

### Purpose

Add the semantic representation for unsaturated constructors, application, and
type lambdas. Make one normalizer authoritative for definitional equality.

### Files

- Create: `lib/src/type_services/normalize.rs`
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/type_services/visit.rs`
- Modify: `lib/src/type_services/display.rs`
- Modify: `lib/src/type_context/mod.rs`
- Modify: `lib/src/type_context/view.rs`
- Modify: every exhaustive semantic `Type` match found by CodeGraph

### RED Tests

Add unit tests for:

- constructor kind-preserving application;
- section desugaring with holes bound left-to-right;
- beta reduction;
- fewer-than, equal-to, and greater-than binder-count application;
- de Bruijn shifting and capture avoidance;
- alpha equivalence;
- eta reduction;
- flattening nested applications;
- saturation into `Type::Struct` and `Type::Enum`;
- nested constructor composition;
- normalization idempotence;
- alias cycles and normalization depth limits;
- canonical TypeContext interning.
- fatal normalization limit/cycle behavior with no partial canonical-store
  insertion.

Run:

```bash
cargo test -p rock-lib type_services::normalize -- --nocapture
```

Expected RED: constructor/lambda variants and normalizer do not exist.

### Implementation

Add semantic constructor, application, lambda, and bound-variable forms from
the design. Use de Bruijn identity internally. Keep source binder names only in
diagnostic metadata.

Implement the normalizer as a service with an explicit declaration/alias/kind
environment. It owns alias expansion, beta/eta reduction, application
flattening, saturation, and cycle/depth handling.

Require canonical terms on TypeContext insertion. Provide a validation method
for accepted HIR and artifact decode. Do not normalize opportunistically in
`Display`, `Hash`, layout, selection, or codegen.

Layout and runtime fact services must return a structured `not a runtime type`
error for unsaturated terms rather than using placeholders.

Update `TypeFacts::is_concrete` and every similar wildcard classifier to match
new HKT variants explicitly. Add tests proving constructors, lambdas, abstract
applications, and constructor projections are not accidentally classified as
concrete/layoutable runtime types.

### GREEN Gates

```bash
cargo test -p rock-lib type_services::normalize -- --nocapture
cargo test -p rock-lib type_context -- --nocapture
cargo test -p rock-lib type_services -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 5: Lower And Validate Kinds, Sections, Aliases, And Projections

### Purpose

Resolve parsed HKT syntax into canonical semantic terms and enforce phase kind
invariants before body inference.

### Files

- Modify: `lib/src/type_lowering.rs`
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/collect/declarations.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/item_index.rs`
- Modify: `lib/src/parser/items/trait.rs`
- Modify: `lib/src/parser/items/impl.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/lower/items.rs`
- Modify: `lib/src/lower/pipeline.rs`
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/hir/accepted.rs`
- Modify: `lib/src/hir/type_ids.rs`
- Modify: `lib/src/type_services/projection.rs`
- Modify: resolver/export structures that currently omit `TopLevel::NewType`
- Modify: HIR declaration stores to add `HirTypeAlias`
- Modify: `lib/src/infer/mod.rs` and `PartialHir` construction to transport
  aliases into resolved/accepted HIR

### RED Tests

Add exact tests for:

- `F A` preserving `A` when `F` is a constructor generic;
- nominal arity and kind errors;
- unsaturated constructors rejected in fields, locals, params, and returns;
- constructors accepted in constructor arguments and impl targets;
- `Result _, E` lowering to the same canonical term as an explicit lambda;
- transparent constructor aliases and cycle errors;
- alias collection, import/export, module qualification, and canonical DefId
  identity;
- `type Family _` and constructor-valued associated projections;
- accepted HIR rejecting noncanonical or wrong-kind terms.

Run:

```bash
cargo test -p rock-lib type_lowering -- --nocapture
cargo test -p rock-lib kind_check -- --nocapture
```

Expected RED: generic heads remain leaf `Type::Generic` values and arguments are
lost or mis-kinded.

### Implementation

Extend `TypeLoweringContext` with ID-based constructor, alias, generic
descriptor, and associated-kind lookup. Lower names to nominal constructors or
rigid generic parameters, then apply arguments through the normalizer.

Promote parsed `TopLevel::NewType` into a real ID-owned alias declaration.
Collection must no longer index aliases and then ignore them. Add
`HirTypeAlias`, resolver/import/export support, visibility, generic descriptors,
body lowering, and dependency-interface hooks. Rename the AST variant if
`NewType` would incorrectly imply nominal runtime identity.

Add aliases to `DeclarationItems`, checked ID maps, `LowerItems`,
`LoweringPipeline::finish`, `PartialHir`, resolved/accepted HIR construction,
and every declaration validation/index. A test must prove an alias survives each
named boundary rather than only proving collection can resolve it temporarily.

Desugar holes only when the complete containing application is known. Reject a
bare hole. Lower lambda source binders into de Bruijn-bound semantic terms.

Validate all declaration signatures and runtime type locations. Accepted HIR
may contain a kind-`Type` application such as `F A`, but every unsaturated term
must occur only in a compile-time constructor position.

Do not infer missing nominal arguments. Partial application is explicit in the
type term and must be expected by its context.

Parse and lower generic associated declarations such as `type Family _` and
associated definitions with matching binders. Define projection reduction by
exact trait/impl/AssocTypeId; reject unresolved projections in coherence heads
and executable pre-MIR types.

### GREEN Gates

```bash
cargo test -p rock-lib type_lowering -- --nocapture
cargo test -p rock-lib collect::headers -- --nocapture
cargo test -p rock-lib lower::types -- --nocapture
cargo test -p rock-lib hir::accepted -- --nocapture
cargo test -p rock-lib hir::type_ids -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 6: Implement Kind-Aware Inference And Generalization

### Purpose

Infer constructor arguments in decidable aligned patterns while preserving the
strict declared-generic versus inference-variable boundary.

### Files

- Modify: `lib/src/infer/engine.rs`
- Modify: `lib/src/infer/constraints.rs`
- Modify: `lib/src/infer/generalize.rs`
- Modify: `lib/src/infer/type_vars.rs`
- Modify: `lib/src/infer/finalize.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/src/lower/types_helpers/type_vars.rs`
- Modify: call-site generic instantiation in lowering

### RED Tests

Add tests for:

```text
F A ~ Option I64
F (G A) ~ Vec (Option I64)
F A ~ Result A, E with explicit F = Result _, E
F I64 ~ Result Bool, I64 with inferred F = Result Bool
```

Also test:

- kind mismatch before type mismatch;
- kind-aware occurs checks;
- lambda binder escape rejection;
- constructor variable generalization with stable descriptor order;
- strict rejection of unresolved constructor vars;
- rejection of `F I64 ~ Result I64, Bool` because it would require synthesizing
  the non-rigid-prefix lambda `Result _, Bool`;
- no numeric defaulting of constructor variables;
- unchanged ordinary generic and literal inference.

Run:

```bash
cargo test -p rock-lib infer::engine -- --nocapture
cargo test -p rock-lib infer::generalize -- --nocapture
```

Expected RED: inference variables have no kind and unification cannot decompose
generic constructor applications.

### Implementation

Record a `Kind` for every inference variable. Normalize before unification and
require equal kinds. Add structural cases for constructor, application, lambda,
and bound variables.

Implement only the constructor-spine rule in the design: split a rigid actual
application into a prefix constructor and trailing arguments aligned with the
metavariable application. Do not consider constant-function or synthesized
lambda alternatives. Emit a diagnostic with the unsolved equation and the
explicit section/annotation required by the user.

Generalization creates `GenericParamDecl`s with kinds. Finalization and every
type-variable replacement path use the shared folder and remain
capture-avoiding under lambdas.

### GREEN Gates

```bash
cargo test -p rock-lib infer::engine -- --nocapture
cargo test -p rock-lib infer::generalize -- --nocapture
cargo test -p rock-lib infer::finalize -- --nocapture
cargo test -p rock-lib strict_finalization -- --nocapture
cargo test -p rock-lib literal_default -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 7: Replace String Where Clauses With Typed Predicates And Supertraits

### Purpose

Allow trait obligations on constructors and represent the
Functor-Applicative-Monad hierarchy without string-subject or display-name
semantics.

### Files

- Modify: where-clause parsers under `lib/src/parser/items/`
- Modify: `lib/src/ast/tree.rs`
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/infer/constraints.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/products.rs`
- Modify: `lib/src/products/type_table.rs`

### RED Tests

Add tests for:

- arbitrary typed predicate subjects;
- `where F _: Functor` declaring F's kind and trait obligation;
- trait declaration predicates acting as supertraits;
- `Applicative F` implying the selected `Functor F` obligation;
- missing and cyclic supertrait diagnostics;
- solver termination on malformed recursive obligations;
- same-spelled traits distinguished by DefId;
- constructor arguments in predicate artifact roundtrips.

Run:

```bash
cargo test -p rock-lib supertrait -- --nocapture
cargo test -p rock-lib typed_predicate -- --nocapture
```

Expected RED: `WhereClause` stores a string subject and trait declarations have
no predicates.

### Implementation

Replace `WhereClause { type_param: String, ... }` after parsing with a typed,
ID-backed HIR predicate. Keep a parsed predicate form with spans for diagnostics.

Store trait predicates/supertraits on `HirTrait`. Extend obligation solving with
cycle detection and deterministic diagnostic ordering. Constructor obligations
use the same trait IDs and candidate stores as ordinary type obligations.

Reject supertrait graph cycles by canonical trait ID during local collection and
again after artifact remapping. Keep a canonical obligation stack and memo table
inside solving as a defensive termination guarantee.

Associated type declarations receive their generic descriptors and resulting
kind in this task if Task 5 only scaffolded them.

### GREEN Gates

```bash
cargo test -p rock-lib parser::items::tests::trait -- --nocapture
cargo test -p rock-lib lower::traits::conformance -- --nocapture
cargo test -p rock-lib infer::solve -- --nocapture
cargo test -p rock-lib selection::service -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 8: Add Constructor Impl Targets, Orphan Rules, And Overlap Rejection

### Purpose

Represent constructor impls honestly and make coherence a declaration/artifact
property rather than a late selection accident.

### Files

- Create: `lib/src/traits/coherence.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/selection/matching.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: current impl indexing/storage modules
- Modify: `lib/src/lower/pipeline.rs`
- Modify: `lib/src/crate_artifact/load.rs`

### RED Tests

Add tests for:

- constructor target kind checking;
- local trait for foreign constructor accepted;
- foreign trait for local constructor accepted;
- foreign trait for foreign constructor rejected;
- alias-expanded outer-constructor locality;
- overlap between alpha-renamed targets;
- overlap between `Result _, E` and `\T -> Result T, E`;
- overlap between local and dependency artifact impls;
- constructor-variable blanket target rejection;
- deterministic diagnostics independent of map insertion order.

Run:

```bash
cargo test -p rock-lib traits::coherence -- --nocapture
```

Expected RED: impls use runtime receiver-oriented metadata and ambiguity is
primarily detected during selection/mono.

### Implementation

Introduce an explicit constructor impl target in HIR. Normalize all impl heads
before indexing. Enforce the design's restricted header grammar: one concrete
nominal outer head, optional outer lambda, no unresolved projection, no generic
outer constructor, and first-order body matching after rigid skolemization.
Implement orphan checks and pairwise overlap with that grammar. Predicates do
not prove disjointness.

During overlap, align lambda binders as shared rigid skolems and treat each
impl's generic parameters as fresh flexible first-order variables. Constructor
generic variables may appear atomically as nominal arguments but not as
application heads. Add the explicit witness case
`\T -> Wrap E, T` versus `\T -> Wrap I64, T` with `E = I64`.

Run coherence after local/dependency interfaces are merged but before accepted
HIR. Add an explicit `CoherencePhase::run` in `LoweringPipeline::prepare_traits`
after trait header/default preparation and conformance, but before dependency or
current-crate body lowering. Its context must include every transitive explicit
dependency impl interface from `CrateContext`. Reuse the same checker after
artifact remapping/normalization. Keep defensive ambiguity
checks in selection and mono; update their diagnostics to identify a violated
upstream invariant if reached.

Do not add specialization or candidate ranking.

### GREEN Gates

```bash
cargo test -p rock-lib traits::coherence -- --nocapture
cargo test -p rock-lib lower::traits::conformance -- --nocapture
cargo test -p rock-lib selection_service_rejects_equally_specific_trait_impls -- --exact --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 9: Select Constructor Trait Members And Persist Exact Authority

### Purpose

Support constructor-level static trait calls locally and generically without
name-based fallback.

### Files

- Modify: `lib/src/selection/types.rs`
- Modify: `lib/src/selection/matching.rs`
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/infer/authority.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/hir/accepted.rs`
- Modify: `lib/src/hir/type_ids.rs`

### RED Tests

Add tests for:

- `Option::pure` selecting an Applicative constructor impl;
- `F::pure` selecting through a constructor bound;
- `F::Functor::fmap` explicit qualification;
- `(Result _, IoError)::pure` selecting the unary fixed-error constructor;
- bare `Result::pure` rejected for target-kind mismatch;
- inherent short-name precedence and exact qualified trait selection;
- Result-section impl substitution preserving `E`;
- same-spelled constructor traits selected by ID;
- ambiguity reported during lowering;
- accepted HIR rejecting targetless constructor calls.

Run:

```bash
cargo test -p rock-lib constructor_trait_selection -- --nocapture
```

Expected RED: selection expects kind-`Type` receivers/owners and has no
constructor authority representation.

### Implementation

Add a constructor selection request/result path to `SelectionService`. Reuse
canonical impl IDs, member IDs, typed substitutions, and predicates. Do not
reuse runtime receiver-adjustment code.

Extend static path lowering for constructor variables and fully qualified
`Constructor::Trait::member` syntax. Lower successful calls with exact selected
authority and complete owner/method substitutions. Materialize unresolved
generic authority after inference using the same constructor selection path.

For `F::member` under a constructor trait bound, accepted HIR records the exact
trait/member IDs and deferred constructor substitution but no impl ID. After
mono substitutes `F`, run concrete constructor selection and attach the exact
impl/member instance before MIR. Concrete calls continue to record impl
authority during lowering.

Add distinct HIR authority variants for concrete impl selection and deferred
trait-bound selection. Accepted HIR validation permits either when internally
complete; monomorphized-HIR validation requires the concrete/resolved variant.
Do not encode absence of an impl with sentinel IDs or optional fields whose
meaning depends on phase.

### GREEN Gates

```bash
cargo test -p rock-lib constructor_trait_selection -- --nocapture
cargo test -p rock-lib selection::service -- --nocapture
cargo test -p rock-lib infer::authority -- --nocapture
cargo test -p rock-lib hir::accepted -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 10: Carry Constructor Substitutions Through TypeContext, Mono, And MIR Barriers

### Purpose

Make higher-kinded generic code specialize to existing concrete runtime types
with canonical instance identity and no backend HKT representation.

### Files

- Modify: `lib/src/type_context/mod.rs`
- Modify: `lib/src/type_context/view.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/specialize.rs`
- Modify: `lib/src/mono/substitute.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/hir/mod.rs` concrete-type checks
- Modify: `lib/src/mir/agreement.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/backend_contract.rs`
- Modify: `lib/src/codegen/types.rs`

### RED Tests

Add tests proving:

- constructor `TypeId`s carry kinds but layout APIs reject non-`Type` kinds;
- instance keys distinguish different constructors;
- backend symbols/fingerprints are independent of TypeId allocation and
  dependency load order;
- eta-equivalent constructors reuse one instance;
- specialization substitutes `F` through nested `F A` applications;
- Result sections beta-reduce after substituting `E`;
- constructor trait methods become ordinary instance calls;
- MIR rejects every unspecialized constructor/lambda/bound/generic term;
- closed constructor-kind arguments retained inside nominal instance identity
  never receive layouts, while all resolved field/variant/ABI facts are
  concrete;
- LLVM sees the same concrete nominal layouts as direct Option/Result/Vec code.

Run:

```bash
cargo test -p rock-lib mono::specialize -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
```

Expected RED: specialization extracts only leaf generics and nominal arguments.

### Implementation

Allow TypeContext to intern terms of any kind. Require explicit kind `Type` in
layout APIs and runtime type queries.

Extend generic extraction and substitution through applications, lambdas, and
constructor-valued projections. Normalize immediately after substitution and
before `InstanceKey` construction. Keep the existing `Vec<TypeId>` instance
substitution identity.

Update concrete-HIR and MIR validation. Do not add LLVM lowering for any HKT
variant. A surviving term is a compiler error at the MIR agreement boundary.

Preserve the current phase order exactly:

```text
accepted HIR -> monomorphized HIR -> MIR -> borrow checking -> agreement -> LLVM
```

A saturated nominal may retain closed constructor-kind arguments in its
TypeContext/instance identity. Before MIR construction, resolve and persist all
kind-`Type` field, variant, ABI, drop, and projection facts in the backend
contract. MIR/codegen may use the opaque nominal TypeId to query those facts but
must not normalize or request a layout for a constructor argument.

Keep `TypeId` only as an in-session instance-key component. Add a stable
canonical structural fingerprint for backend symbols using product crate
identity, local DefIds, kinds, normalized terms, and binder indices. Prove it is
independent of interning and artifact load order.

Replace the current symbol construction in `mono/mod.rs`, including
`substitution_symbol_suffix`, `def_symbol_fragment`, and every caller of
`backend_symbol_for_origin`. Pass canonical current/dependency product-crate
identity data into `Monomorphizer`; a remapped session `CrateId` or raw TypeId
must never enter the stable fingerprint.

Compute one `CompilationIdentityContext` in `lib.rs::compile_impl` before
optional product emission and before `mono::monomorphize_with_crates`. It owns
the current `ProductCrateIdentity` plus dependency product identities from
`CrateContext`. Compute the current source fingerprint once regardless of
`emit_products`, reuse the same identity for `CompilerProducts`, and pass the
context explicitly into monomorphization. Product emission must not be the
phase that creates semantic/backend symbol identity.

### GREEN Gates

```bash
cargo test -p rock-lib type_context -- --nocapture
cargo test -p rock-lib mono -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib codegen -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 11: Complete Format-44 Artifacts And Cross-Crate HKT

### Purpose

Persist every HKT interface/body fact portably and prove downstream selection
and specialization from artifacts.

### Files

- Modify: `lib/src/products.rs`
- Modify: `lib/src/products/type_table.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_artifact/types.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `rock-shared/src/sysroot.rs`
- Modify: `rockc/src/main.rs`
- Modify: `lib/tests/integration.rs`

### RED Tests

Add format-44 tests for:

- generic descriptor kinds;
- type alias declarations, bodies, visibility, exports, and generic descriptors;
- nominal constructor rows;
- application rows;
- lambda and bound-variable rows;
- higher-kinded projection rows;
- constructor trait target and method signatures;
- method-level generic binders, static-member status, and deferred bound-call
  authority;
- typed predicates and supertraits;
- constructor impl targets and substitutions;
- ProductDefId remapping inside every new row;
- alpha-equivalent roundtrip normalization;
- malformed kind, binder scope, cycle, and impl rejection;
- oversized preamble/header/payload, collection count, string, type depth, and
  normalization-output rejection before unbounded allocation;
- format 43 rejection before full decode;
- dependency Functor/Monad selection and generic-body specialization.

Run:

```bash
cargo test -p rock-lib products::type_table -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
```

Expected RED: format 44 metadata cannot represent the new semantic terms or
impl headers completely.

### Implementation

Extend `ProductTypeRow` and serialized interfaces exactly once. Every child type
uses `ProductTypeId`; every declaration owner uses product-local IDs. Serialize
de Bruijn bound variables by scope/index and validate scope while decoding.

Replace the monolithic bincode artifact shell with the format-44 preamble,
bounded identity/dependency header, and payload envelope from the design. The
header must precede the type table so dependency ordering and freshness checks
never deserialize semantic payloads.

Decode in this order:

1. Version check.
2. Bounded portable DTO decode plus table-reference, finite-structure, and
   lambda-scope checks while IDs remain product-local.
3. Product ID remapping into the consumer identity environment.
4. Alias/declaration environment construction and type normalization.
5. Kind, canonical-form, generic descriptor, predicate, and supertrait
   validation.
6. Orphan and coherence validation against the complete loaded interface set.
7. Consumer TypeContext interning and semantic product materialization.

Reject malformed data without panics or unbounded recursion. Do not serialize
producer TypeIds.

Refactor the current eager
`CompilerProducts::from_artifact_bytes -> artifact_to_products` path so it can
return or internally retain a portable decoded DTO until the loader has the
consumer remap environment. Add a bounded header reader for freshness and
dependency-order inspection; those callers must not force premature semantic
Type construction. `crate_artifact/load.rs` owns remap followed by semantic
normalization/validation.

Update `lib.rs::load_extern_artifacts` to read all bounded headers first, verify
declared dependencies, establish deterministic load/remap identities, and only
then decode/remap/validate each payload in topological order. Implement every
format-44 resource limit before allocation from an untrusted declared length;
plain `bincode::deserialize` into unrestricted vectors is forbidden.

Replace production `read_artifact_from_path` use with explicit staged APIs,
equivalent to:

```rust
ProductArtifactHeader::read_from_path_bounded(path)
PortableProductArtifact::read_payload_from_path_bounded(path, &header)
```

The header reader checks filesystem length against 512 MiB and validates the
fixed preamble before allocating a file-sized buffer. The payload reader streams
or bounded-reads only the declared payload. Migrate all production callers,
including `lib.rs::load_extern_artifacts`, `crate_artifact/load.rs`, rockc
artifact-dependency printing, rockc artifact validation, and every freshness or
sysroot reader found by CodeGraph. Header-only callers must not materialize
semantic products. Remove or make test-only the eager whole-file semantic
reader once no production caller remains.

### GREEN Gates

```bash
cargo test -p rock-lib products::type_table -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib crate_system -- --nocapture
cargo test -p rock-lib mono::external -- --nocapture
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact
cargo fmt --all --check
git diff --check
```

## Task 12: Add Lawful Stdlib Functional Type Classes

### Purpose

Exercise HKT through ordinary source code and provide the reusable abstraction
surface requested by the feature.

### Files

- Create: `stdlib/functor.rk`
- Create: `stdlib/applicative.rk`
- Create: `stdlib/monad.rk`
- Create: `stdlib/foldable.rk`
- Create: `stdlib/traversable.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`
- Modify: `stdlib/option.rk`
- Modify: `stdlib/result.rk`
- Modify: `stdlib/vec.rk`
- Modify: `stdlib/callable.rk` only if tuple callback conformance needs a
  general source-level correction
- Modify: `lib/tests/integration.rs`

### RED Tests

Add stdlib-backed integration tests for:

- generic `fmap` over `Option`, `Result _, E`, and `Vec`;
- `pure`/`ap` and `bind` through constructor-generic functions;
- constructor-qualified static calls;
- `(Result _, IoError)::pure` and a constructor-alias equivalent;
- nested constructor composition;
- `Foldable` order;
- `Traversable` and `traverse_m` with Option/Result effects;
- no compiler language-item registration for any functional trait;
- cross-crate calls through a prebuilt stdlib artifact.

Add finite executable law tests for each shipped impl.
Use pure, total callbacks for algebraic law tests; test mutable/effectful callback
ordering separately as an operational property.

Run:

```bash
cargo test -p rock-lib --test integration test_hkt_functor_option_result_vec -- --exact --nocapture
cargo test -p rock-lib --test integration test_hkt_applicative_and_monad_laws -- --exact --nocapture
cargo test -p rock-lib --test integration test_hkt_foldable_and_traversable -- --exact --nocapture
```

Expected RED: traits/modules/constructor impls are absent.

### Implementation

Implement the hierarchy from the design as ordinary Rock traits. Keep imports
acyclic; use one module per abstraction unless a smaller grouping remains clear.

Option and `Result _, E` implement all five traits. Vec implements Functor,
Foldable, and Traversable only. Document the ownership reason for omitting Vec
Applicative/Monad in source docs near the public exports.

Existing inherent Option/Result methods remain ergonomic APIs and delegate to
one semantic implementation where possible. Do not create mutually recursive
trait/inherent delegation.

Use `FnMut` bounds for callbacks that may run multiple times. Verify captured
mutable closures. Preserve left-to-right evaluation and Result error types.

### GREEN Gates

```bash
cargo test -p rock-lib --test integration test_hkt_functor_option_result_vec -- --exact --nocapture
cargo test -p rock-lib --test integration test_hkt_applicative_and_monad_laws -- --exact --nocapture
cargo test -p rock-lib --test integration test_hkt_foldable_and_traversable -- --exact --nocapture
cargo test -p rock-lib test_stdlib_option_methods -- --nocapture
cargo test -p rock-lib test_stdlib_result_methods -- --nocapture
cargo test -p rock-lib crate_artifact::tests::test_compile_with_stdlib_artifact -- --exact --nocapture
cargo fmt --all --check
git diff --check
```

## Task 13: Add Eager Ownership-Sensitive Vec Operations

### Purpose

Provide practical transformation and iteration without lazy adapters or
forcing borrowed operations through a consuming Functor contract.

### Files

- Modify: `stdlib/vec.rk`
- Modify: supporting stdlib allocation/drop helpers only if a reusable primitive
  is genuinely required
- Modify: `lib/tests/integration.rs`

### RED Tests

Add runtime and compiler tests for:

- consuming `map` preserves order;
- `map_ref` leaves the source usable;
- empty and single-element vectors;
- move-only String values;
- mutable captured callback state;
- source elements are dropped exactly once;
- callback outputs are dropped exactly once;
- `for_each`, `for_each_owned`, `retain`, `filter`, and `filter_map` order;
- `try_map` and `try_for_each` stop on the first Result error;
- large vectors do not recurse or overflow the stack;
- allocation failure and early process exit do not introduce a double drop.

Run:

```bash
cargo test -p rock-lib --test integration test_stdlib_vec_eager_map_move_only_preserves_order -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_map_ref_preserves_source -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_try_for_each_short_circuits -- --exact --nocapture
```

Expected RED: Vec has only imperative primitive methods.

### Implementation

Implement operations directly in `stdlib/vec.rk`. For consuming map/filter:

1. Save source length and pointer.
2. Establish a drop-safe ownership-transfer invariant before moving elements.
3. Move each element once in index order.
4. Push transformed/retained values into the destination.
5. Ensure consumed slots cannot be dropped by the source.
6. Release source storage through the normal RawBuffer/Vec drop path.

Rock has no unwind semantics, but ordinary error returns and callback behavior
must still leave ownership explicit. Do not use `swap_remove` for ordered map or
filter. Do not clone to simplify transfer.

`try_map` and `try_for_each` are Result-aware concrete operations and must avoid
invoking later callbacks after `Err`.

### GREEN Gates

```bash
cargo test -p rock-lib --test integration test_stdlib_vec_eager_map_move_only_preserves_order -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_map_ref_preserves_source -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_try_for_each_short_circuits -- --exact --nocapture
cargo test -p rock-lib vec -- --nocapture
cargo test -p rock-lib mir::borrowck -- --nocapture
cargo fmt --all --check
git diff --check
```

## Task 14: Refactor `new_new`, Run Closure Audits, And Finish Documentation

### Purpose

Use the new abstractions in the motivating program, prove end-to-end behavior,
and close every production boundary.

### Files

- Modify: `test_projects/new_new/main.rk`
- Modify: `lib/tests/integration.rs` or the project test harness
- Modify: `LANGUAGE_SPECIFICATION.md`
- Modify: public stdlib documentation if present
- Modify: this design/plan only to record final verified deviations or evidence
- Modify: `MEMORY.md` only for durable, verified architecture facts

### RED Tests

Add a deterministic artifact-backed compile regression for the project before
refactoring. Pair it with a non-network subprocess run using an invalid mode so
argument dispatch and executable linking are exercised without entering
`listen` or `connect`. Verify:

- the full project compiles against a freshly built stdlib artifact;
- the invalid-mode path returns and prints the expected Result error;
- lowered/HIR authority for the new finite collection calls is exact;
- Vec unit/integration tests separately cover snapshot-order, retain, and
  traversal semantics used by the refactor;
- argument dispatch result;
- no network port or timing dependency exists in the stable gate.

Do not claim live server/client behavior from this gate. If a live socket
regression is added, use the timeout-aware process harness, isolated loopback
resources, and a bounded cleanup path; run it separately from the stable gate.

### Implementation

Refactor:

- `ServerState::snapshot` to `map_ref`;
- `ServerState::remove` to `retain`;
- broadcast over the owned snapshot to eager `for_each` or `try_for_each`
  according to the existing error policy;
- argument dispatch to Option/Result combinators only where it is clearer;
- finite collection transformations to Functor/Foldable helpers where ownership
  matches.

Keep socket receive loops explicit. Do not introduce a lazy stream iterator or
hide cancellation/error state behind an unrelated abstraction.

Document:

- kind syntax and type sections;
- constructor traits and qualification;
- Result partial application;
- laws and strict evaluation;
- Vec ownership differences;
- why Vec has no Applicative/Monad impl.

### Residue Audit

Search tracked production source and classify every occurrence of:

- `generic_params: Vec<String>`;
- code that assumes every `GenericParamId` has kind `Type`;
- type lowering that drops generic-head arguments;
- name-based constructor trait or impl lookup;
- Result-specific HKT handling;
- ad hoc beta reduction outside the normalizer;
- executable constructor/lambda normalization, application, or layout handling
  in MIR/codegen, excluding opaque closed nominal-instance identity metadata;
- artifact format 43 acceptance;
- targetless constructor trait authority;
- implicit clone/boxing in functional operations.

Do not add permanent tests that only search for deleted source strings. Record
the audit evidence in the completion report and keep semantic regression tests.

### Focused GREEN Gates

```bash
cargo test -p rock-lib parser::items::tests -- --nocapture
cargo test -p rock-lib fmt:: -- --nocapture
cargo test -p rock-lib type_services -- --nocapture
cargo test -p rock-lib type_context -- --nocapture
cargo test -p rock-lib collect:: -- --nocapture
cargo test -p rock-lib lower:: -- --nocapture
cargo test -p rock-lib infer:: -- --nocapture
cargo test -p rock-lib selection:: -- --nocapture
cargo test -p rock-lib traits::coherence -- --nocapture
cargo test -p rock-lib mono:: -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib --test integration test_hkt_functor_option_result_vec -- --exact --nocapture
cargo test -p rock-lib --test integration test_hkt_applicative_and_monad_laws -- --exact --nocapture
cargo test -p rock-lib --test integration test_hkt_foldable_and_traversable -- --exact --nocapture
```

From `tree-sitter-rock/`:

```bash
tree-sitter generate
tree-sitter test
```

### Full Closure Gates

Run serially and save broad logs:

```bash
cargo test -p rock-lib > /tmp/rock-hkt-rock-lib.log 2>&1
cargo test --workspace > /tmp/rock-hkt-workspace.log 2>&1
cargo clippy --workspace --all-targets -- -D warnings > /tmp/rock-hkt-clippy.log 2>&1
cargo fmt --all --check
git diff --check
```

Compile the motivating project with `rockc` and the explicit stdlib artifact:

```bash
cargo run -p rockc -- \
  --crate-name stdlib \
  --entry-file stdlib/lib.rk \
  --output-dir /tmp/rock-hkt-stdlib \
  --no-std \
  --no-prelude \
  --no-link \
  --emit-object /tmp/rock-hkt-stdlib/stdlib.o \
  --emit-artifact /tmp/rock-hkt-stdlib/stdlib.rkca

cargo run -p rockc -- \
  --entry-file test_projects/new_new/main.rk \
  --output-dir /tmp/rock-hkt-new-new \
  --extern-artifact stdlib=/tmp/rock-hkt-stdlib/stdlib.rkca

/tmp/rock-hkt-new-new/main invalid
```

The first command explicitly prepares the format-44 stdlib artifact; the gate
must not depend on a pre-existing `build/stdlib.rkca`. `rockc` compiles and
links but does not run the output, so the final command is the deterministic
runtime check.

## Review Gates

After Tasks 4, 8, 11, and 14, request separate reviews with these focuses:

- Task 4: binder correctness, normalization termination, and canonical identity.
- Task 8: coherence soundness, orphan locality, and cross-crate overlap.
- Task 11: malformed artifact safety, remapping completeness, and schema
  authority.
- Task 14: lawfulness, ownership/drop behavior, MIR barrier, and no hidden lazy
  iteration.

Resolve all correctness findings before proceeding past each gate. Performance
or API-polish findings may become linked follow-up beads only when they do not
weaken the specification's acceptance criteria.

## Completion Criteria

Implementation is complete only when every acceptance criterion in the design
specification has evidence, all Task 14 gates pass, format 44 is the sole
accepted artifact schema, and no constructor operation reaches executable
MIR/codegen. Closed constructor arguments may exist only as opaque nominal
instance identity metadata with concrete backend-contract facts. Closing
the implementation epic requires a final report listing:

- every changed semantic representation;
- artifact migration evidence;
- focused RED/GREEN commands actually run;
- full log paths and exit statuses;
- coherence and residue audit results;
- stdlib law-test results;
- `new_new` behavior evidence;
- any explicitly deferred non-goal with its linked issue ID.

## Verification Evidence (2026-08-08)

Task 14 is complete. All implementation, audit, review, and HKT closure gates
are green. The repository-wide strict Clippy baseline is an explicitly approved
deviation tracked independently by `new_lang2-n2e`.

### End-to-End Behavior

- Explicit higher-order binders parse, format, and lower in declaration, trait
  target, type-lambda, and associated-type positions. Conflicting ordinary and
  constructor declarations produce kind diagnostics rather than silently
  re-kinding an existing generic.
- Trait signature generics use member-owned `GenericParamId`s, including
  bound-only parameters. Monomorphization maps selected trait-member bindings
  to the corresponding impl-method generics by canonical member identity.
- `ServerState::snapshot` uses eager borrowed `Vec::map_ref`.
- `ServerState::remove` uses stable-order `Vec::retain`.
- broadcast consumes its owned snapshot with `Vec::for_each_owned`.
- socket receive loops remain explicit.
- `test_new_new_fresh_artifact_invalid_mode_is_deterministic` builds a unique
  stdlib artifact, compiles and links `new_new` against it, and verifies the
  non-network `invalid` path prints `Err(IoError)`.
- The exact manual artifact commands in this task also exited `0`; the runtime
  output is recorded in `/tmp/rock-hkt-new-new-run.log`.
- Separate final specification-compliance and code-quality reviews reported no
  remaining correctness findings.

### Residue Classification

The tracked-source audit is saved at `/tmp/rock-hkt-residue-audit.log`.

- The production `Vec<String>` generic-name fields are temporary lexical
  collection/lowering contexts paired with owner IDs, kind vectors, and
  `GenericParamId` maps. HIR, products, artifacts, and executable phases use
  `GenericParamDecl` and canonical IDs. Remaining exact hits outside those
  contexts are tests and the semantic-identity audit itself.
- MIR/codegen constructor-term matches are rejection barriers, traversal of
  opaque closed nominal identity metadata, and tests. LLVM type/default-value
  handling rejects constructor/application/lambda/bound-variable terms.
- Format `43` occurs only in the negative preamble rejection regression; other
  numeric `43` hits are fixture IDs or values. Both artifact constants are `44`.
- `Result` constructor-section hits in compiler code are semantic regressions or
  fixtures. No production Result-specific HKT selection or inference branch was
  found.
- Beta/eta reduction is implemented by `type_services::normalize`; inference and
  monomorphization call that normalizer rather than carrying independent
  reducers.
- Constructor trait selection is ID-based. `target: None` hits in the audited
  paths are ordinary first-order traits or test fixtures, not targetless
  constructor-trait authority.
- No clone or boxing hit exists in the functional trait, Option, Result, or Vec
  implementation files. Generic-head arguments remain covered by type-lowering,
  normalization, artifact, selection, and monomorphization tests.

### Gate Logs

- All focused Task 14 compiler gates exited `0`; logs are
  `/tmp/rock-hkt-focused-*.log`.
- The Functor, Applicative/Monad, Foldable/Traversable, and derived `sequence`
  runtime gates exited `0`; logs are `/tmp/rock-hkt-{functor,applicative-monad,foldable-traversable,sequence}.log`.
- `cargo test -p rock-lib` exited `0` with 2,217 unit and 592 integration tests:
  `/tmp/rock-hkt-rock-lib.log`.
- `cargo test --workspace` exited `0`, including 2,217 `rock-lib` unit and 592
  integration tests: `/tmp/rock-hkt-workspace.log`.
- `tree-sitter generate` and `tree-sitter test` exited `0`:
  `/tmp/rock-hkt-tree-sitter-{generate,test}.log`.
- `cargo fmt --all --check` and `git diff --check` exited `0`.
- Fresh stdlib and `new_new` build logs are
  `/tmp/rock-hkt-{stdlib-build,new-new-build}.log`; both commands exited `0`.
  `/tmp/rock-hkt-new-new-run.log` contains the verified `Err(IoError)` output.

### Deferred Repository-Wide Gate

`cargo clippy --workspace --all-targets -- -D warnings` exits `101` with 149
repository-wide warnings across more than 40 lint classes, led by 27
`clippy::result_large_err` findings. The unchanged baseline spans compiler,
source-loader, codegen, and test code and is not safely addressable through HKT
changes or blanket lint suppression. Cleanup is deferred to priority-1 task
`new_lang2-n2e` and does not block the completed HKT implementation. The strict log is
`/tmp/rock-hkt-clippy.log`; ordinary `cargo clippy --workspace --all-targets`
completes successfully with warnings in `/tmp/rock-hkt-clippy-warnings.log`.
