# Struct Field Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make struct fields private by default, keep `< field: Type` as the public-field marker, and enforce Rust-like privacy for struct field access, struct construction, and struct pattern matching.

**Architecture:** Keep the existing parser and HIR visibility flag, and add semantic enforcement in lowering only. Track the active struct impl owner in `Lowerer`, reuse shared helper methods for field lookup and privacy checks, and apply those checks in the three struct-only entry points: dot field access, struct literals, and struct patterns.

**Tech Stack:** Rust 2021, `rock-lib`, parser tests under `lib/src/parser/items/tests`, semantic lowering under `lib/src/lower`, integration tests under `lib/tests/integration.rs`.

---

### Task 1: Lock Struct Visibility Syntax And Access Rules With Failing Tests

**Files:**
- Modify: `lib/src/parser/items/tests/struct_decl/test_parse_struct_with_fields.rs`
- Modify: `lib/tests/integration.rs`
- Test: `cargo test -p rock-lib test_parse_struct_with_fields -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_rejected_outside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_private_struct_construction_is_rejected_outside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_rejected_outside_impl -- --exact --nocapture`

- [ ] **Step 1: Extend the parser regression to assert default-private and explicit-public fields**

```rust
#[test]
fn test_parse_struct_with_fields() {
    let input = "struct Test\n    field: Type\n    < field2: Type2\n";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, struct_decl) = struct_decl
        .process(ParseCtx::from(&tokens, &config))
        .unwrap();

    let field = struct_decl
        .fields
        .iter()
        .find(|field| field.name.name == "field")
        .unwrap();
    let field2 = struct_decl
        .fields
        .iter()
        .find(|field| field.name.name == "field2")
        .unwrap();

    assert_eq!(struct_decl.name.name, "Test");
    assert_eq!(struct_decl.fields.len(), 2);
    assert_eq!(field.ty.to_string(), "Type");
    assert!(!field.public);
    assert_eq!(field2.ty.to_string(), "Type2");
    assert!(field2.public);
    assert_eq!(rest.len(), 0);
}
```

- [ ] **Step 2: Add focused integration tests for access, construction, and patterns before changing lowering**

```rust
#[test]
fn test_public_struct_field_access_is_allowed_outside_impl() {
    let output = compile_and_run(
        r#"
struct Point
    < x: I64
    y: I64

impl Point
    new = ->
        Point
            x: 1
            y: 2

main = ->
    p = Point::new!
    p.x.println!
    0
"#,
    );

    assert_eq!(output.trim(), "1");
}

#[test]
fn test_private_struct_field_access_is_rejected_outside_impl() {
    compile_should_fail(
        r#"
struct Point
    < x: I64
    y: I64

impl Point
    new = ->
        Point
            x: 1
            y: 2

main = ->
    p = Point::new!
    p.y.println!
    0
"#,
        "Field 'y' of struct 'Point' is private",
    );
}

#[test]
fn test_private_struct_field_access_is_allowed_inside_impl() {
    let output = compile_and_run(
        r#"
struct Point
    x: I64
    y: I64

impl Point
    new = ->
        Point
            x: 3
            y: 4

    @sum = -> @x + @y

main = ->
    Point::new!.sum!.println!
    0
"#,
    );

    assert_eq!(output.trim(), "7");
}

#[test]
fn test_private_struct_construction_is_rejected_outside_impl() {
    compile_should_fail(
        r#"
struct Point
    < x: I64
    y: I64

main = ->
    p = Point
        x: 1
        y: 2
    p.x.println!
    0
"#,
        "Cannot construct struct 'Point' because it has private fields",
    );
}

#[test]
fn test_private_struct_construction_is_allowed_inside_impl() {
    let output = compile_and_run(
        r#"
struct Point
    < x: I64
    y: I64

impl Point
    new = ->
        Point
            x: 5
            y: 6

    @x_value = -> @x

main = ->
    Point::new!.x_value!.println!
    0
"#,
    );

    assert_eq!(output.trim(), "5");
}

#[test]
fn test_private_struct_pattern_is_rejected_outside_impl() {
    compile_should_fail(
        r#"
struct Point
    < x: I64
    y: I64

impl Point
    new = ->
        Point
            x: 7
            y: 8

main = ->
    p = Point::new!
    match p
        Point y: value => value
"#,
        "Field 'y' of struct 'Point' is private",
    );
}

#[test]
fn test_private_struct_pattern_is_allowed_inside_impl() {
    let output = compile_and_run(
        r#"
struct Point
    < x: I64
    y: I64

impl Point
    new = ->
        Point
            x: 9
            y: 10

    @y_value = ->
        match self
            Point y: value => value

main = ->
    Point::new!.y_value!.println!
    0
"#,
    );

    assert_eq!(output.trim(), "10");
}

#[test]
fn test_all_public_struct_construction_is_allowed_outside_impl() {
    let output = compile_and_run(
        r#"
struct Point
    < x: I64
    < y: I64

main = ->
    p = Point
        x: 11
        y: 12
    p.x.println!
    p.y.println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines[0], "11");
    assert_eq!(lines[1], "12");
}
```

- [ ] **Step 3: Run the parser regression and confirm it starts red only if the visibility assertion is wrong**

Run: `cargo test -p rock-lib test_parse_struct_with_fields -- --exact --nocapture`
Expected: PASS after the assertion update, proving the parser already supports the syntax.

- [ ] **Step 4: Run the focused compile-fail/private-visibility regressions and watch them fail before implementation**

Run: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_rejected_outside_impl -- --exact --nocapture`
Expected: FAIL because field privacy is not yet enforced in lowering.

Run: `cargo test -p rock-lib --test integration test_private_struct_construction_is_rejected_outside_impl -- --exact --nocapture`
Expected: FAIL because struct literal privacy is not yet enforced.

Run: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_rejected_outside_impl -- --exact --nocapture`
Expected: FAIL because struct pattern privacy is not yet enforced.

### Task 2: Add Struct-Owner Context And Enforce Private Field Access

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/bodies.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`
- Test: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_rejected_outside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_allowed_inside_impl -- --exact --nocapture`

- [ ] **Step 1: Add current-struct-impl tracking and shared visibility helpers to `Lowerer`**

```rust
pub struct Lowerer {
    pub(crate) engine: InferenceEngine,
    pub(crate) scope: Scope,
    pub(crate) structs: HashMap<String, HirStruct>,
    pub(crate) enums: HashMap<String, HirEnum>,
    pub(crate) traits: HashMap<String, HirTrait>,
    pub(crate) impls: Vec<HirImpl>,
    pub(crate) externs: Vec<HirExtern>,
    pub(crate) functions: HashMap<String, HirFunction>,
    pub(crate) function_sigs: HashMap<String, HirFunctionSig>,
    pub(crate) methods: HashMap<(String, String), HirFunction>,
    pub(crate) errors: Vec<ResolveError>,
    pub(crate) file_path: PathBuf,
    pub(crate) current_span: Option<Span>,
    pub(crate) function_type_vars: HashMap<String, HashSet<u32>>,
    pub(crate) current_function: Option<String>,
    pub(crate) current_impl_struct: Option<String>,
    pub(crate) current_crate_name: Option<String>,
    // ...
}

pub(crate) fn is_inside_struct_impl(&self, struct_name: &str) -> bool {
    self.current_impl_struct.as_deref() == Some(struct_name)
}

pub(crate) fn lookup_struct_field(&self, struct_name: &str, field_name: &str) -> Option<&HirField> {
    self.structs
        .get(struct_name)
        .and_then(|hir_struct| hir_struct.fields.iter().find(|field| field.name == field_name))
}

pub(crate) fn can_access_struct_field(&self, struct_name: &str, field_name: &str) -> bool {
    self.lookup_struct_field(struct_name, field_name)
        .map(|field| field.public || self.is_inside_struct_impl(struct_name))
        .unwrap_or(false)
}

pub(crate) fn report_private_struct_field(
    &mut self,
    struct_name: &str,
    field_name: &str,
    span: Span,
) {
    self.push_error_with_span(
        format!("Field '{}' of struct '{}' is private", field_name, struct_name),
        span,
    );
}
```

- [ ] **Step 2: Set and restore the active owner struct while lowering impl bodies**

```rust
let previous_impl_struct = self.current_impl_struct.clone();
if self.structs.contains_key(&type_name) {
    self.current_impl_struct = Some(type_name.clone());
} else {
    self.current_impl_struct = None;
}

let body = self.lower_function_body_block(fd);

self.current_impl_struct = previous_impl_struct;
```

- [ ] **Step 3: Gate dot field access in `secondary.rs` before building `HirExprKind::FieldAccess`**

```rust
if let Type::Struct(name, type_args) = &inner_ty {
    if let Some(field) = self.lookup_struct_field(name, field_name) {
        if !field.public && !self.is_inside_struct_impl(name) {
            self.report_private_struct_field(name, field_name, span.clone());
            return HirExpr {
                ty: Type::Error,
                kind: HirExprKind::FieldAccess(Box::new(base_expr), ident.name.clone()),
                span,
            };
        }

        let mut subst: HashMap<String, Type> = HashMap::new();
        if let Some(hir_struct) = self.structs.get(name) {
            for (param, arg) in hir_struct.generic_params.iter().zip(type_args.iter()) {
                subst.insert(param.clone(), arg.clone());
            }
        }
        field_ty = field.ty.substitute_generics(&subst);
    }
}
```

- [ ] **Step 4: Route `@field` desugaring through the same privacy rule in `expression.rs`**

```rust
if let Type::Struct(ref struct_name, ref type_args) = resolved_self {
    if let Some(field) = self.lookup_struct_field(struct_name, &ident.name) {
        if !field.public && !self.is_inside_struct_impl(struct_name) {
            self.report_private_struct_field(struct_name, &ident.name, ident.span.clone());
            return HirExpr {
                ty: Type::Error,
                kind: HirExprKind::FieldAccess(Box::new(self_var), ident.name.clone()),
                span: ident.span.clone(),
            };
        }

        let mut subst: HashMap<String, Type> = HashMap::new();
        if let Some(type_def) = self.structs.get(struct_name) {
            for (param, arg) in type_def.generic_params.iter().zip(type_args.iter()) {
                subst.insert(param.clone(), arg.clone());
            }
        }
        let resolved_field_ty = field.ty.substitute_generics(&subst);
        let _ = self.engine.unify(&field_ty, &resolved_field_ty);
    }
}
```

- [ ] **Step 5: Run the focused access regressions and confirm the new field-access rule is green**

Run: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_rejected_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_allowed_inside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_public_struct_field_access_is_allowed_outside_impl -- --exact --nocapture`
Expected: PASS

### Task 3: Enforce Private Field Rules For Struct Construction

**Files:**
- Modify: `lib/src/lower/paths.rs`
- Test: `cargo test -p rock-lib --test integration test_private_struct_construction_is_rejected_outside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_private_struct_construction_is_allowed_inside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_all_public_struct_construction_is_allowed_outside_impl -- --exact --nocapture`

- [ ] **Step 1: Add a whole-struct privacy gate before lowering a struct literal outside the owner impl**

```rust
if hir_struct.fields.iter().any(|field| !field.public) && !self.is_inside_struct_impl(&type_name) {
    self.push_error_with_span(
        format!("Cannot construct struct '{}' because it has private fields", type_name),
        span.clone(),
    );
}
```

- [ ] **Step 2: Keep the current field-type inference path intact when lowering the struct literal fields**

```rust
for (ident, expr) in &inst.fields {
    let hir_expr = self.lower_expression(expr);

    if let Some(field_info) = hir_struct.fields.iter().find(|f| f.name == ident.name) {
        let expected_ty = field_info.ty.substitute_generics(&type_var_mapping);
        let _ = self.engine.unify(&expected_ty, &hir_expr.ty);
    }

    fields.push((ident.name.clone(), hir_expr));
}
```

- [ ] **Step 3: Run the focused construction regressions**

Run: `cargo test -p rock-lib --test integration test_private_struct_construction_is_rejected_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_construction_is_allowed_inside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_all_public_struct_construction_is_allowed_outside_impl -- --exact --nocapture`
Expected: PASS

### Task 4: Enforce Private Field Rules For Struct Patterns

**Files:**
- Modify: `lib/src/lower/control_flow/pattern.rs`
- Test: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_rejected_outside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_allowed_inside_impl -- --exact --nocapture`

- [ ] **Step 1: Add field-visibility checks in the struct-pattern path before building `HirPattern::Struct`**

```rust
let field_patterns: Vec<(String, HirPattern)> = fields
    .iter()
    .map(|f| {
        if let Some(field_info) = self.lookup_struct_field(&type_name, &f.name.name) {
            if !field_info.public && !self.is_inside_struct_impl(&type_name) {
                self.report_private_struct_field(&type_name, &f.name.name, f.name.span.clone());
            }
        }

        let t = self.engine.fresh_type_var();
        let pat = self.lower_pattern(&f.pattern, &t);
        (f.name.name.clone(), pat)
    })
    .collect();
```

- [ ] **Step 2: Leave enum pattern lowering untouched**

```rust
if let Some(enum_info) = self.enums.get(&enum_name).cloned() {
    // keep existing enum behavior unchanged
}
```

- [ ] **Step 3: Run the focused pattern regressions**

Run: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_rejected_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_allowed_inside_impl -- --exact --nocapture`
Expected: PASS

### Task 5: Verify The Full Struct-Only Slice And Broader Regressions

**Files:**
- Modify: only files from Tasks 1-4 if a failing regression requires a minimal fix
- Test: `cargo test -p rock-lib --test integration test_public_struct_field_access_is_allowed_outside_impl -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_struct_field_assignment -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_nested_structs -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration test_impl_methods -- --exact --nocapture`
- Test: `cargo test -p rock-lib --test integration`

- [ ] **Step 1: Re-run the full focused struct-visibility slice serially**

Run: `cargo test -p rock-lib --test integration test_public_struct_field_access_is_allowed_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_rejected_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_field_access_is_allowed_inside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_construction_is_rejected_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_construction_is_allowed_inside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_rejected_outside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_private_struct_pattern_is_allowed_inside_impl -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_all_public_struct_construction_is_allowed_outside_impl -- --exact --nocapture`
Expected: PASS

- [ ] **Step 2: Run nearby existing struct regressions to catch collateral damage**

Run: `cargo test -p rock-lib --test integration test_struct_field_assignment -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_nested_structs -- --exact --nocapture`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_impl_methods -- --exact --nocapture`
Expected: PASS

- [ ] **Step 3: Run the full integration suite if the focused slice is green**

Run: `cargo test -p rock-lib --test integration`
Expected: PASS or a short list of remaining regressions directly caused by struct visibility changes.

- [ ] **Step 4: If the integration suite passes, run the broader library suite once**

Run: `cargo test -p rock-lib`
Expected: PASS

- [ ] **Step 5: Inspect the worktree state before handing off**

Run: `git status --short`
Expected: only the intended parser, lowering, test, spec, and plan files are modified.
