# Struct Field Visibility Design

## Goal

Make Rock struct fields private by default, with Rust-like opt-in public fields using `< field: Type` syntax.

This change applies only to structs and only to struct field access semantics:
- direct field access
- struct construction
- struct pattern matching

Enums and all other language features remain unchanged.

## Language Semantics

### Field Declarations

Struct fields are private by default.

- `field: T` means the field is private
- `< field: T` means the field is public

Example:

```rock
struct Point
    < x: I64
    y: I64
```

The parser already recognizes this syntax and records field visibility on struct fields.

### Direct Field Access

Direct field access on structs follows Rust-like privacy.

- public fields may be accessed anywhere the struct type itself is accessible
- private fields may only be accessed from methods or associated functions inside the same struct's `impl`

Valid:

```rock
struct Point
    < x: I64
    y: I64

impl Point
    @sum = -> @x + @y

main = ->
    p = Point::new!
    p.x
```

Invalid:

```rock
main = ->
    p.y
```

### Struct Construction

Struct construction must also respect field privacy.

If a struct has any private field, code outside that struct's own `impl` cannot construct it using a struct literal.

This matches Rust's rule in practice: once construction would require naming a private field, outside code cannot build the struct directly.

Allowed:

```rock
impl Point
    new = x, y ->
        Point
            x: x
            y: y
```

Rejected outside the owner impl:

```rock
main = ->
    Point
        x: 1
        y: 2
```

For structs whose fields are all public, construction remains allowed anywhere.

### Struct Pattern Matching

Struct pattern matching obeys the same visibility rules.

- patterns may destructure public fields anywhere
- patterns may destructure private fields only inside the same struct's `impl`

Valid inside owner impl:

```rock
impl Point
    @y_value = ->
        match self
            Point y: value => value
```

Invalid outside owner impl:

```rock
match p
    Point y: value => value
```

### Owner Rule

The owner of a private struct field is the struct's own `impl` block.

Private fields are accessible only while lowering code inside an `impl` whose target type is the same struct.

This includes:
- instance methods
- associated functions

This does not include:
- free functions
- impls for other types
- trait impls for other types
- unrelated modules

For generic structs, ownership is determined by the base struct name, not by specific type arguments.

## Non-Goals

- changing enum field visibility or enum payload access
- introducing module-scoped privacy beyond the struct-owner rule above
- changing method visibility
- changing export/import behavior
- changing tuple field or array index access

## Compiler Design

### Existing State

The parser and AST already support field visibility for struct fields:
- `lib/src/parser/items/struct_decl.rs` parses optional leading `<`
- `lib/src/ast/tree.rs` stores `StructDeclField.public`
- `lib/src/hir/mod.rs` stores `HirField.public`
- `lib/src/lower/collect/types.rs` carries the flag into HIR structs

The missing work is semantic enforcement.

### Enforcement Phase

Enforce struct field visibility during lowering.

This is the minimal place that already has:
- resolved struct definitions
- impl-lowering context
- field access lowering
- struct literal lowering
- struct pattern lowering

No separate visibility pass is needed.

### Lowerer Context

Add a small amount of lowering context to track the active owner struct while lowering impl bodies.

Required capability:
- know whether the current code is being lowered inside `impl StructName ...`
- compare that struct name to the struct whose field is being accessed

This should be represented by a dedicated current-impl-struct field on `Lowerer`, set while lowering impl method bodies and cleared afterward.

The comparison should be based on the struct name recorded in the impl, ignoring type arguments.

### Shared Visibility Helper

Add one shared helper for struct-field access control in lowering.

The helper should answer:
- whether a named field exists on a given struct
- whether that field is public
- whether the current lowering context is allowed to use it

It should produce consistent diagnostics for:
- field access
- struct construction
- struct pattern matching

This avoids duplicating the same private-field rule in three different code paths.

### Direct Field Access Lowering

Apply visibility checks when lowering struct field access in:
- `lib/src/lower/control_flow/secondary.rs`
- `lib/src/lower/expression.rs` for `@field` desugaring via `self.field`

Behavior:
- if the field is public, allow access
- if the field is private and the active impl owner matches the struct, allow access
- otherwise emit a visibility diagnostic and continue with an error/fallback type

Method resolution behavior should stay unchanged. This feature is only about field access.

### Struct Literal Lowering

Apply visibility checks in `lib/src/lower/paths.rs` when lowering struct literals.

Behavior:
- for each named field in the literal, validate that the field exists as today
- if the field is private and the current impl owner is not the same struct, emit an error
- if the struct contains any private fields and construction is outside the owner impl, reject construction

The simplest Rust-like rule here is:
- outside the owner impl, a struct literal is allowed only if all fields of the struct are public

That keeps the behavior easy to reason about and matches the user-visible Rust rule.

### Struct Pattern Lowering

Apply visibility checks in `lib/src/lower/control_flow/pattern.rs` for struct field patterns.

Behavior:
- keep existing struct-pattern lowering shape
- for each named field pattern, validate visibility before producing `HirPattern::Struct`
- private field patterns are allowed only inside the same struct's own impl

### Diagnostics

Use clear, struct-specific diagnostics.

Examples:

```text
Field 'y' of struct 'Point' is private
```

```text
Cannot construct struct 'Point' because it has private fields
```

Prefer span-aware errors at the field access site, field pattern site, or struct literal site.

## Affected Areas

Primary files:
- `lib/src/lower/mod.rs`
- `lib/src/lower/bodies.rs`
- `lib/src/lower/control_flow/secondary.rs`
- `lib/src/lower/expression.rs`
- `lib/src/lower/paths.rs`
- `lib/src/lower/control_flow/pattern.rs`
- `lib/src/parser/items/tests/struct_decl/test_parse_struct_with_fields.rs`
- `lib/tests/integration.rs`

Likely unchanged:
- `lib/src/codegen/**`
- `lib/src/mir/**`
- enum lowering paths

## Test Plan

### Parser

Add or extend a parser test to confirm:
- `field: Type` parses as private
- `< field: Type` parses as public

### Integration Tests

Add focused integration tests for:

1. Public field access from outside the struct impl succeeds.
2. Private field access from outside the struct impl fails.
3. Private field access inside the struct's own impl succeeds.
4. Struct construction with a private field fails outside the struct impl.
5. Struct construction with a private field succeeds inside an associated constructor on the same impl.
6. Struct pattern matching on a private field fails outside the struct impl.
7. Struct pattern matching on a private field succeeds inside the struct's own impl.
8. A struct with all-public fields remains constructible from outside.

### Regression Scope

Verify that existing enum tests remain unaffected.

This work should not change:
- enum construction
- enum pattern matching
- enum method calls
- tuple access
- array indexing

## Recommended Implementation Order

1. Extend parser coverage for `< field: Type` and default-private fields.
2. Add failing integration tests for private field access, construction, and pattern matching.
3. Add lowerer owner-context tracking for the current struct impl.
4. Implement shared struct-field visibility helpers.
5. Enforce visibility in direct field access lowering.
6. Enforce visibility in struct literal lowering.
7. Enforce visibility in struct pattern lowering.
8. Run focused tests, then the relevant broader `rock-lib` suite if needed.

## Resolved Decisions

- Struct fields are private by default: yes
- `< field: Type` means public field: yes
- Private fields are accessible only from the same struct's own impl: yes
- Struct construction must respect private fields: yes
- Struct pattern matching must respect private fields: yes
- Enums are included: no
