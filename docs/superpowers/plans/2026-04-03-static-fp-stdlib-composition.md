# Static FP Stdlib Composition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add static, specialized FP-style composition for `Option` and `Result`, with a Haskell-leaning operator surface and a pure methods surface that share the same concrete implementation.

**Architecture:** Keep semantics in concrete monomorphized methods on `Option` and a new `Result` enum. Use the current left-operand operator model where it fits (`>>=`, `<|>`, flipped map), and only add compiler work where it unlocks clearly useful symbolic composition without runtime cost. Do not add HKT, runtime typeclass dictionaries, or fake universal `Functor`/`Monad` traits in v1.

**Tech Stack:** Rust 2021, Rock stdlib under `stdlib/`, compiler changes in `lib/src/**`, integration tests in `lib/tests/integration.rs`, LLVM 18 via `rock-lib`.

---

## Scope Notes

- `Option` and `Result` are the only effect containers in scope for this plan.
- `Vec` composition is explicitly deferred to a follow-up plan.
- The method APIs are the semantic source of truth.
- The operator APIs are a thin ergonomic layer over those methods.
- This plan intentionally stops short of universal constructor-polymorphic typeclasses.

## File Map

- `stdlib/option.rk`
  - Extend `Option` with compositional method APIs and symbolic operator methods.
- `stdlib/result.rk`
  - Add a new stdlib `Result T, E` type with methods, `Show` impl, and operator methods.
- `stdlib/fp.rk`
  - Add reusable function helpers like `|>` once custom infix function lowering exists.
- `stdlib/lib.rk`
  - Register new modules and declare the new operator precedence entries.
- `stdlib/prelude.rk`
  - Re-export `Result`, FP helpers, and any public symbols meant for the prelude.
- `lib/src/lower/expression.rs`
  - Teach custom infix operators to resolve to ordinary functions before method fallback.
- `lib/tests/integration.rs`
  - Add stdlib-backed integration coverage for methods and operators.
- `README.md`
  - Document the new method and operator APIs.
- `LANGUAGE_SPECIFICATION.md`
  - Update operator examples to match shipped behavior.

### Task 1: Add `Option` And `Result` Method APIs

**Files:**
- Modify: `stdlib/option.rk`
- Create: `stdlib/result.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write a failing integration test for `Option` methods**

```rust
#[test]
fn test_stdlib_option_methods() {
    let output = compile_and_run(
        r#"
inc = x -> x + 1

to_even = x ->
    if x % 2 == 0
        Option::Some x
    else
        Option::None

main = ->
    some = Option::Some 41
    none = Option::None
    nested = Option::Some (Option::Some 9)

    (some.map inc).unwrap_or 0 .println!
    (none.map inc).unwrap_or 0 .println!
    (some.and_then to_even).unwrap_or 0 .println!
    ((Option::Some 8).and_then to_even).unwrap_or 0.println!
    nested.flatten!.unwrap_or 0 .println!
    (some.fold 0, x -> x + 1) .println!
    (none.fold 7, x -> x + 1) .println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["42", "0", "0", "8", "9", "42", "7"]);
}
```

- [ ] **Step 2: Run the focused test and verify it fails for missing methods**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_option_methods -- --exact`
Expected: FAIL with missing method/type errors for `map`, `and_then`, `flatten`, or `fold`

- [ ] **Step 3: Write a failing integration test for `Result` methods**

```rust
#[test]
fn test_stdlib_result_methods() {
    let output = compile_and_run(
        r#"
inc = x -> x + 1
tag = err -> String::from_str "tagged"

parse = x ->
    if x > 0
        Result::Ok x
    else
        Result::Err (String::from_str "bad")

main = ->
    ok = Result::Ok 41
    err = Result::Err (String::from_str "boom")
    nested = Result::Ok (Result::Ok 9)

    (ok.map inc).unwrap_or 0 .println!
    (err.map inc).unwrap_or 0 .println!
    (ok.and_then parse).unwrap_or 0 .println!
    ((Result::Ok -1).and_then parse).unwrap_or 0.println!
    (err.map_err tag).fold 0, e -> e.len .println!
    nested.flatten!.unwrap_or 0 .println!
    (ok.fold e -> 0, x -> x + 1) .println!
    (err.fold e -> e.len, x -> x) .println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["42", "0", "41", "0", "6", "9", "42", "4"]);
}
```

- [ ] **Step 4: Run the focused test and verify it fails for missing `Result` stdlib support**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_result_methods -- --exact`
Expected: FAIL with missing `Result` crate item or missing method errors

- [ ] **Step 5: Implement the minimal `Option` method set**

```rock
impl Option T
    @map = f ->
        match self
            Option::Some val => Option::Some (f val)
            Option::None => Option::None

    @and_then = f ->
        match self
            Option::Some val => f val
            Option::None => Option::None

    @or = other ->
        match self
            Option::Some _ => self
            Option::None => other

    @flatten = ->
        match self
            Option::Some inner => inner
            Option::None => Option::None

    @fold = default, f ->
        match self
            Option::Some val => f val
            Option::None => default

    @inspect = f ->
        match self
            Option::Some val =>
                f val
                self
            Option::None => self
```

- [ ] **Step 6: Implement the minimal `Result` type and method set**

```rock
< enum Result T, E
    Ok T
    Err E

impl Result T, E
    @map = f ->
        match self
            Result::Ok val => Result::Ok (f val)
            Result::Err err => Result::Err err

    @map_err = f ->
        match self
            Result::Ok val => Result::Ok val
            Result::Err err => Result::Err (f err)

    @and_then = f ->
        match self
            Result::Ok val => f val
            Result::Err err => Result::Err err

    @or = other ->
        match self
            Result::Ok _ => self
            Result::Err _ => other

    @unwrap_or = default ->
        match self
            Result::Ok val => val
            Result::Err _ => default

    @flatten = ->
        match self
            Result::Ok inner => inner
            Result::Err err => Result::Err err

    @fold = on_err, on_ok ->
        match self
            Result::Ok val => on_ok val
            Result::Err err => on_err err
```

- [ ] **Step 7: Export `Result` and wire modules into stdlib**

```rock
// stdlib/lib.rk
< mod result
< mod fp

// stdlib/prelude.rk
< stdlib::result::*
< stdlib::fp::*
```

- [ ] **Step 8: Add a minimal `Show` impl for `Result` when values are showable**

```rock
impl Show for Result T, E where T: Show, E: Show
    @show = ->
        match self
            Result::Ok val =>
                (String::from_str "Ok(").concat (val.show!).concat (String::from_str ")")
            Result::Err err =>
                (String::from_str "Err(").concat (err.show!).concat (String::from_str ")")
```

- [ ] **Step 9: Run the focused method tests and verify they pass**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_option_methods -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_result_methods -- --exact`
Expected: PASS

### Task 2: Add Operator Surface For `Option` And `Result`

**Files:**
- Modify: `stdlib/option.rk`
- Modify: `stdlib/result.rk`
- Modify: `stdlib/lib.rk`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write a failing integration test for `>>=`, `<|>`, and `<&>`**

```rust
#[test]
fn test_stdlib_option_result_fp_operators() {
    let output = compile_and_run(
        r#"
inc = x -> x + 1

keep_even_opt = x ->
    if x % 2 == 0
        Option::Some x
    else
        Option::None

keep_even_res = x ->
    if x % 2 == 0
        Result::Ok x
    else
        Result::Err (String::from_str "odd")

main = ->
    some = Option::Some 4
    none = Option::None
    ok = Result::Ok 4
    err = Result::Err (String::from_str "bad")

    (some >>= keep_even_opt).unwrap_or 0 .println!
    (none >>= keep_even_opt).unwrap_or 0 .println!
    ((Option::Some 4) <&> inc).unwrap_or 0 .println!
    ((Option::None <|> Option::Some 9)).unwrap_or 0 .println!

    (ok >>= keep_even_res).unwrap_or 0 .println!
    (err >>= keep_even_res).unwrap_or 0 .println!
    ((Result::Ok 4) <&> inc).unwrap_or 0 .println!
    ((Result::Err (String::from_str "x")) <|> Result::Ok 9).unwrap_or 0 .println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["4", "0", "5", "9", "4", "0", "5", "9"]);
}
```

- [ ] **Step 2: Run the focused operator test and verify it fails for missing operator methods**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_option_result_fp_operators -- --exact`
Expected: FAIL with no implementation errors for one or more of `>>=`, `<|>`, `<&>`

- [ ] **Step 3: Declare operator precedences in `stdlib/lib.rk`**

```rock
infix 2 >>=
infix 3 <|>
infix 4 <&>
```

- [ ] **Step 4: Add operator methods on `Option`**

```rock
impl Option T
    @>>= = f -> self.and_then f
    @<|> = other -> self.or other
    @<&> = f -> self.map f
```

- [ ] **Step 5: Add operator methods on `Result`**

```rock
impl Result T, E
    @>>= = f -> self.and_then f
    @<|> = other -> self.or other
    @<&> = f -> self.map f
```

- [ ] **Step 6: Run the focused operator test and verify it passes**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_option_result_fp_operators -- --exact`
Expected: PASS

### Task 3: Lower Custom Infix Operators To Functions Before Method Fallback

**Files:**
- Modify: `lib/src/lower/expression.rs`
- Create: `stdlib/fp.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write a failing integration test for `|>` as a normal infix function**

```rust
#[test]
fn test_stdlib_pipe_operator_function() {
    let output = compile_and_run(
        r#"
inc = x -> x + 1

main = ->
    41 |> inc .println!
    ((Option::Some 41) <&> inc).unwrap_or 0 .println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["42", "42"]);
}
```

- [ ] **Step 2: Run the focused operator-function test and verify it fails because custom operators do not resolve to functions yet**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_pipe_operator_function -- --exact`
Expected: FAIL with no implementation found for operator `|>`

- [ ] **Step 3: Add a new stdlib FP helper module with `|>`**

```rock
infix 1 |>

< |> = x, f -> f x
```

- [ ] **Step 4: Wire the FP helper module into stdlib exports**

```rock
// stdlib/lib.rk
< mod fp

// stdlib/prelude.rk
< stdlib::fp::*
```

- [ ] **Step 5: Change custom infix lowering to resolve a same-named function before method fallback**

```rust
if trait_name.is_empty() {
    if let Some((func_ty, _)) = self.scope.lookup(op_str) {
        return HirExpr {
            ty: result_ty,
            kind: HirExprKind::Call(
                Box::new(HirExpr {
                    ty: func_ty.clone(),
                    kind: HirExprKind::Variable(op_str.clone()),
                    span: self.current_span.clone().unwrap_or_default(),
                }),
                vec![left, right],
            ),
            span: self.current_span.clone().unwrap_or_default(),
        };
    }
}
```

- [ ] **Step 6: Run the focused pipe test and verify it passes**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_pipe_operator_function -- --exact`
Expected: PASS

### Task 4: Document Operator-First And Method-Only Usage

**Files:**
- Modify: `README.md`
- Modify: `LANGUAGE_SPECIFICATION.md`

- [ ] **Step 1: Add a README section showing the operator-first style**

```markdown
## FP-style composition

```haskell
inc = x -> x + 1
keep_even = x ->
    if x % 2 == 0
        Option::Some x
    else
        Option::None

main = ->
    result = (Option::Some 4) <&> inc >>= keep_even
    result.unwrap_or 0 .println!
```
```

- [ ] **Step 2: Add a README section showing the pure-method equivalent**

```markdown
```haskell
result = (Option::Some 4).map inc .and_then keep_even
```
```

- [ ] **Step 3: Update the language specification to describe `|>` as a custom infix function example rather than aspirational syntax**

```markdown
infix 1 |>
|> = x, f -> f x
```

- [ ] **Step 4: Sanity-check docs for consistency with shipped operators**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_pipe_operator_function -- --exact`
Expected: PASS

### Task 5: Final Verification

**Files:**
- Verify only

- [ ] **Step 1: Run the focused new tests together**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_option_methods -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_result_methods -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_option_result_fp_operators -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_pipe_operator_function -- --exact`
Expected: PASS

- [ ] **Step 2: Run a broader stdlib integration sweep**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_stdlib_math -- --exact`
Expected: PASS

- [ ] **Step 3: Run formatting if needed and confirm no new failures appear**

Run: `cargo fmt --all`
Expected: PASS

- [ ] **Step 4: Run the full relevant Rust test suite for confidence**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib`
Expected: PASS
