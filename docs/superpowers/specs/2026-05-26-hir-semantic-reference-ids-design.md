# HIR Semantic Reference IDs Design

**Date:** 2026-05-26
**Status:** Approved for implementation planning
**Roadmap task:** Task 3, `Convert HIR Semantic References To IDs`

## Purpose

Complete roadmap Task 3 by making every HIR semantic reference owned by Task 3 carry an authoritative compiler identity. Source names remain in HIR as diagnostics, display, source reconstruction, and compatibility metadata, but they must not be the only representation of a resolved semantic target.

This design intentionally does not remove all downstream string consumers. Removing persistent alias maps, moving path resolution out of `Lowerer`, replacing structural `Type`, selection-service cleanup, monomorphization instance cleanup, and MIR/codegen metadata extraction belong to later roadmap tasks.

## Source Requirements

- `docs/superpowers/specs/2026-04-24-compiler-architecture-audit-design.md` requires stable semantic identity through compiler-owned IDs, with names and backend symbols kept as display/output data.
- `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` defines Task 3 as replacing remaining string-valued HIR semantic references with explicit IDs or scoped local IDs where practical.
- `docs/superpowers/plans/master-audit-checklist.md` keeps Identity And Arenas open for local/variable references, field references, call edges, and unmigrated method/aggregate/pattern references.

## Completion Definition

Task 3 is complete when all of these are true:

- HIR locals, parameters, pattern bindings, tuple-destructuring temporaries, `for` loop variables, and closure captures have scoped local IDs.
- Local variable reads and assignments carry local IDs when lowering resolved them.
- Direct top-level function, extern, and already-instantiated callables continue to carry `DefId` or `InstanceId` through `HirVarRef`.
- Direct call expressions carry an explicit resolved callee sidecar when the callee is a known function, extern, instance, local function value, or intrinsic.
- Struct literals and struct patterns carry struct `DefId` plus field IDs for every resolved field.
- Enum variant expressions and enum variant patterns carry enum `DefId` plus variant IDs for every resolved variant.
- Field access and field assignment carry field owner and field ID when lowering resolved the field.
- Method calls carry selected method identity through `HirMethodCallTarget` whenever lowering or selection knows it.
- Error recovery never invents semantic IDs. Unresolved references keep display names and `Type::Error`-style recovery, not fake IDs.
- Product emission and product artifact loading serialize, remap, and validate the new sidecars before accepting artifact HIR.
- The master audit checklist and ordered roadmap mark Task 3 complete, with any remaining string compatibility clearly assigned to Tasks 4-5, 11-15, 18, 21, or 23.

## Non-Goals

- Do not remove compatibility string fields or maps if downstream phases still need them.
- Do not complete Task 4 alias-table migration.
- Do not complete Task 5 source/path resolution extraction from `Lowerer`.
- Do not migrate structural `Type` to `TypeId` beyond preserving existing sidecars.
- Do not make selection service fully ID-only beyond preserving selected `HirMethodCallTarget` values already available.
- Do not replace monomorphization, DCE, MIR, or codegen consumers wholesale. They may read new sidecars opportunistically only when that is the smallest safe way to preserve behavior.
- Do not change product artifact schema policy beyond adding/remapping/validating Task 3 sidecars.

## Current Gaps

Current HIR already has several ID sidecars:

- `HirVarRef` targets direct functions, externs, and instances.
- `HirStructLiteralField` and `HirStructPatternField` can carry `HirFieldLocation`.
- `HirExprKind::StructLiteral` can carry a struct `DefId`.
- `HirExprKind::EnumVariant` and `HirPattern::Enum` can carry `HirVariantLocation`.
- `HirMethodCallTarget` carries impl, trait, method, trait-argument, and index-operator identity.

Remaining gaps are Task 3-owned when HIR has only a string after lowering resolved a semantic target:

- `HirStmt::Let { name, ... }` has no binding ID.
- `HirParam { name, ... }` has no local ID.
- `HirPattern::Binding(String, bool)` has no binding ID.
- `HirExprKind::Var(String)` represents both local references and unresolved/error references.
- `HirExprKind::For { var, ... }` has no loop variable ID.
- `HirClosureCapture { name, ... }` has no source local ID.
- `HirExprKind::Call` has no explicit callee sidecar, even when the callee is resolved.
- Some struct/enum/field/pattern paths still allow `None` sidecars for resolved references and should instead use `None` only for error recovery or intentionally unresolved compatibility cases.

## HIR Identity Model

### Local IDs

Add a scoped local identity type to `ids.rs`:

```rust
define_id!(HirLocalId);
```

`HirLocalId` is local to one HIR function or closure body. It is not a `DefId`, is not cross-crate, and is not serialized as a globally meaningful identity. It is stable within a lowered body and provides a precise target for local reads, assignments, captures, and pattern bindings.

### Local Binding Metadata

Add a HIR-local binding reference shape:

```rust
pub struct HirLocalRef {
    pub id: HirLocalId,
    pub name: String,
}
```

Use it in:

- `HirParam { local: HirLocalRef, ... }` or equivalent fields preserving `name`.
- `HirStmt::Let { local: HirLocalRef, ... }`.
- `HirPattern::Binding { local: HirLocalRef, mutable: bool }`.
- `HirExprKind::For { local: HirLocalRef, ... }`.
- `HirClosureCapture { local: HirLocalRef, ... }`.
- A resolved local expression variant or sidecar for local reads.

The exact Rust shape may keep existing `name` fields and add `local_id: HirLocalId` to minimize diff size. The semantic rule is that `local_id` is authoritative and `name` is display metadata.

### Variable References

Split resolved local references from unresolved/display-only variable references.

Recommended representation:

```rust
pub enum HirVarTarget {
    Local(HirLocalId),
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
}
```

`HirExprKind::ResolvedVar(HirVarRef)` then becomes the common resolved variable-reference form for locals and top-level callable values. `HirExprKind::Var(String)` remains only for unresolved/error recovery and compatibility fixtures that are explicitly not semantically resolved.

## Lowering Data Flow

### Scope Entries

Extend lowering scope entries from `(Type, mutable)` to binding metadata:

```rust
pub struct ScopeBinding {
    pub ty: Type,
    pub mutable: bool,
    pub local_id: Option<HirLocalId>,
    pub is_alias: bool,
}
```

Top-level aliases and functions use `local_id: None` because their semantic identity is a `DefId`/`InstanceId`, not a local. Parameters, local lets, destructuring temps, pattern bindings, and loop variables use `Some(local_id)`.

### Local Allocation

Each function/method/default/closure body lowering context owns an `IdGen<HirLocalId>`. It allocates IDs in source/lowering order for:

- Function parameters, including implicit `self`.
- Let bindings.
- Pattern bindings.
- Tuple destructuring temporaries.
- `for` loop variables.
- Closure parameters.

Lowering must preserve IDs when it rewrites syntax into HIR. For example, tuple destructuring introduces a compiler temporary with its own local ID, then each user binding gets its own ID.

### Scope Push And Pop

`Scope` remains responsible for lexical lookup, shadowing, and mutability. It must also return the active `HirLocalId` for local bindings. Shadowed names produce different IDs. Reads resolve to the innermost binding ID.

### Patterns

Pattern lowering allocates binding IDs and returns HIR patterns with those IDs. For struct and enum patterns, resolved owner/field/variant IDs remain mandatory when the scrutinee type is known. Wildcards and literal patterns do not allocate local IDs.

### Closures

Capture collection records captured local IDs instead of only names. A closure capture is valid only when it refers to an outer local binding. Top-level functions are not captures. Captures keep names for diagnostics and debug output.

### Calls

Add a call-target sidecar for `HirExprKind::Call` when the callee is known:

```rust
pub enum HirCallTarget {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
    Local(HirLocalId),
    Intrinsic(String),
}
```

The call node can keep its callee expression for compatibility and source shape. The sidecar says what semantic callable was resolved at lowering time. Calls through a local function value use `Local(HirLocalId)`. Calls where the callee is unresolved keep no target and must already carry diagnostics or type errors.

### Aggregates And Fields

Struct literals and patterns already have locations. Task 3 should make resolved forms consistently populate them. `None` means unresolved/error recovery, not "resolved but omitted." Field access and assignment must carry `HirFieldLocation` when the receiver type is a known struct/enum payload field.

### Methods

Method calls already carry `HirMethodCallTarget`. Task 3 should tighten invariants and tests so selected calls keep that target for:

- Concrete impl methods.
- Trait methods selected through bounds.
- Trait defaults.
- Builtin/index operator dispatch where the service has selected identity.

Task 3 should not rewrite the selection service architecture. Missing targets that require broader targetless compatibility remain later selection/instance tasks only if lowering truly does not know the target.

## Product And Artifact Data

Product emission must include all new sidecars in the serialized HIR payload. Product artifact loading must remap and validate:

- Local IDs for structural consistency within each function body.
- `HirVarTarget::Function` / `Extern` `DefId`s.
- `HirVarTarget::Instance` values only where product data already has stable instance metadata, otherwise preserve existing compatibility rules.
- Struct owner IDs, field owner IDs, enum owner IDs, variant IDs, and method call targets.
- New call targets.

Artifact loading should reject sidecars that claim resolved identity but do not match the corresponding loaded definition kind. It should not reject `HirExprKind::Var` recovery nodes solely because they lack IDs.

## Error Handling

Missing identity is handled at the phase that attempted resolution:

- Missing local binding: diagnostic plus `Type::Error` and unresolved `Var` recovery.
- Missing field/variant/struct owner: diagnostic plus `Type::Error` or wildcard/error recovery.
- Missing call target for a syntactically callable expression: preserve the callee expression and emit the existing type diagnostic; do not invent IDs.
- Product artifact mismatch: structured artifact load error before remapping is accepted.

No Task 3 path may use `DefId(0, 0)`, `CrateId(u32::MAX)`, or fresh generated current-crate IDs as a reference fallback.

## Testing Strategy

Use TDD for each implementation slice. Required coverage:

- Same-name locals in nested scopes resolve to distinct local IDs.
- Reassignment targets the original binding ID, not a same-name outer binding.
- Function parameters and implicit `self` have local IDs and reads point to them.
- Tuple destructuring temporaries and user bindings get distinct IDs.
- Pattern bindings in `match`, `if let`, and struct/enum patterns get local IDs.
- `for` loop variables get local IDs and loop-body reads target them.
- Closure captures record captured local IDs and distinguish same-name shadowed locals.
- Direct function and extern calls carry call-target sidecars.
- Calls through local function values carry local call targets.
- Struct literals and struct patterns populate field owner/field IDs for every resolved field.
- Enum expressions and enum patterns populate enum owner/variant IDs for every resolved variant.
- Field access and field assignment preserve field IDs.
- Artifact round trips preserve and validate new HIR sidecars.
- Error recovery does not fabricate IDs and keeps diagnostics useful.

Full verification before marking Task 3 complete:

```bash
cargo test -p rock-lib hir_semantic_reference
cargo test -p rock-lib local_id
cargo test -p rock-lib resolved_reference
cargo test -p rock-lib artifact
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Exact focused test names may differ by slice, but the full `cargo test -p rock-lib`, formatting check, diff check, final grep/reference audit, and code review are required before updating roadmap status.

## Documentation Updates At Completion

At implementation completion, update:

- `docs/superpowers/plans/master-audit-checklist.md`: mark Task 3's Identity And Arenas reference-ID work done, and move remaining string compatibility to the owning later tracks.
- `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`: mark Task 3 complete and remove the residual locals/field/call/reference caveat from Task 3.
- Any implementation plan created from this spec with exact verification evidence.

Task 3 may be marked fully complete only if no remaining HIR semantic reference lacks an ID/local ID except explicit unresolved/error recovery or later-task compatibility strings.
