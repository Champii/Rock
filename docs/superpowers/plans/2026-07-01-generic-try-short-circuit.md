# Generic Try Short-Circuit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement postfix `?` as a generic Try-like short-circuit operator backed by stdlib traits.

**Architecture:** Add stdlib protocol types (`ControlFlow`, `Try`, `FromResidual`, `From`) and implement them for `Result` and `Option`. Preserve parser syntax with call-result trailing `?`. The implemented compiler slice lowers `Result` and `Option` `?` into existing HIR `match` plus `return` control flow so MIR cleanup follows explicit return paths; fully generic custom-carrier/static `FromResidual` dispatch remains follow-up work.

**Tech Stack:** Rust compiler implementation in `lib/`, Rock stdlib sources in `stdlib/`, parser unit tests under `lib/src/parser/items/tests/`, integration tests in `lib/tests/integration.rs`, verified with `cargo test -p rock-lib` focused commands.

---

## File Structure

- Modify `stdlib/lib.rk`: register the new `ops` module before `option` and `result`.
- Create `stdlib/ops.rk`: define `ControlFlow`, `Try`, and `FromResidual`.
- Modify `stdlib/convert.rk`: add `From T` trait and identity impl.
- Modify `stdlib/result.rk`: import `ops`/`convert`, add `ResultResidual E`, `Try` impl, and `FromResidual ResultResidual E for Result T, F where F: From E`.
- Modify `stdlib/option.rk`: import `ops`, add `OptionResidual`, `Try` impl, and `FromResidual OptionResidual for Option T`.
- Modify `lib/src/parser/items/expression.rs`: make trailing `?` after inline calls and no-arg bang calls apply to the whole call.
- Modify `lib/src/parser/items/tests/expression/interogation.rs`: cover call-result, no-arg bang call, receiver-chain, parenthesized argument, and parenthesized callee parsing.
- Modify `lib/src/lower/control_flow/secondary.rs`: replace the placeholder `Interogation` lowering with HIR `match`/`return` lowering for `Result` and `Option` carriers.
- Modify `lib/src/mir/builder/expr.rs`: mark value-producing `if`/`match` destinations initialized at merge blocks so generated short-circuit matches are accepted by borrow checking.
- Modify `lib/src/codegen/expr/mod.rs` only if legacy HIR codegen still requires exhaustive match support.
- Modify `lib/tests/integration.rs`: add user-visible tests for `Result`, `Option`, conversion, custom carrier, diagnostics, and early drop cleanup.

---

### Task 1: Stdlib Protocol

**Files:**
- Create: `stdlib/ops.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/convert.rk`
- Modify: `stdlib/result.rk`
- Modify: `stdlib/option.rk`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing stdlib protocol integration test**

Add this test to `lib/tests/integration.rs` near the existing `Result`/`Option` stdlib tests:

```rust
#[test]
fn test_stdlib_try_protocol_methods() {
    let output = compile_and_run(
        r#"
> stdlib::prelude::*

main: -> I64
main = ->
    res = (Result::Ok 41).branch!
    match res
        ControlFlow::Continue value => value + 1
        ControlFlow::Break _ => 0
"#,
    );
    assert_eq!(output.trim(), "42");
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p rock-lib --test integration test_stdlib_try_protocol_methods -- --exact`

Expected: FAIL because `ControlFlow`/`branch` are not defined.

- [ ] **Step 3: Add `stdlib/ops.rk`**

```rock
< enum ControlFlow B, C
    Break B
    Continue C

< trait Try
    type Output
    type Residual
    ~@branch: ControlFlow Self::Residual, Self::Output

< trait FromResidual R
    from_residual: R -> Self
```

- [ ] **Step 4: Register and expose protocol modules**

Add `< mod ops` to `stdlib/lib.rk` before `option` and `result`.

Add `< stdlib::ops::*` to `stdlib/prelude.rk` so tests and users can name `ControlFlow` directly.

- [ ] **Step 5: Add `From` and carrier impls**

In `stdlib/convert.rk`, add:

```rock
< trait From T
    from: T -> Self

impl From T for T
    from = value -> value
```

In `stdlib/result.rk`, import `stdlib::ops::{ControlFlow, Try, FromResidual}` and `stdlib::convert::From`, add `ResultResidual E`, then implement `Try` and `FromResidual`.

In `stdlib/option.rk`, import `stdlib::ops::{ControlFlow, Try, FromResidual}`, add `OptionResidual`, then implement `Try` and `FromResidual`.

- [ ] **Step 6: Verify stdlib protocol test passes**

Run: `cargo test -p rock-lib --test integration test_stdlib_try_protocol_methods -- --exact`

Expected: PASS.

---

### Task 2: Parser Precedence For Call-Result `?`

**Files:**
- Modify: `lib/src/parser/items/expression.rs`
- Modify: `lib/src/parser/items/tests/expression/interogation.rs`

- [ ] **Step 1: Add failing parser tests**

Add tests for these source snippets:

```rock
foo bar?
foo!?
maybe?.get 0?
foo (bar?)
(make_fn?) arg
```

Assert that `foo bar?` parses as `foo` with `Arguments([bar])` followed by `Interogation`, and that `foo (bar?)` keeps `Interogation` inside the parenthesized argument.

- [ ] **Step 2: Verify parser tests fail**

Run: `cargo test -p rock-lib parser::items::tests::expression::interogation -- --nocapture`

Expected: FAIL for call-result cases that currently attach `?` to the final argument.

- [ ] **Step 3: Adjust inline argument parsing**

Change `arguments()` so inline argument expressions parse a primary/application unit that stops before a trailing `?` intended for the call expression. Parenthesized arguments still parse full `expression`, so `foo (bar?)` remains available.

- [ ] **Step 4: Verify parser tests pass**

Run: `cargo test -p rock-lib parser::items::tests::expression::interogation -- --nocapture`

Expected: PASS.

---

### Task 3: HIR Match Lowering For Result/Option Try

**Files:**
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing `Result` success and early-return tests**

Add tests:

```rust
#[test]
fn test_try_result_success_path_unwraps() { /* returns 42 */ }

#[test]
fn test_try_result_error_path_returns_early() { /* returns 7 from Err path */ }
```

Rock snippets should use `value = maybe_value true?` and `value = maybe_value false?` inside functions returning `Result I64, I64`.

- [ ] **Step 2: Verify tests fail**

Run each test with `cargo test -p rock-lib --test integration <name> -- --exact`.

Expected: FAIL because `?` still has placeholder lowering.

- [ ] **Step 3: Lower `SecondaryExpr::Interogation`**

Resolve the carrier enum shape for `Result T, E` and `Option T`. Emit a HIR `Match` whose success arm yields the payload and whose break arm returns `Result::Err payload` or `Option::None` from the enclosing function.

---

### Task 4: MIR Initialization For Generated Try Matches

**Files:**
- Modify: `lib/src/mir/builder/expr.rs`
- Modify MIR helper modules if needed for enum payload extraction.

- [ ] **Step 1: Preserve initialization through value-producing matches**

Ensure MIR lowering marks `if` and `match` expression destinations initialized after their merge blocks. Generated `?` lowering relies on the success arm assigning the destination and the break arm returning before merge.

- [ ] **Step 2: Verify result tests pass**

Run:

```bash
cargo test -p rock-lib --test integration test_try_result_success_path_unwraps -- --exact
cargo test -p rock-lib --test integration test_try_result_error_path_returns_early -- --exact
```

Expected: PASS.

---

### Task 5: Option, Conversion, Custom Carrier, Diagnostics, Cleanup

**Files:**
- Modify: `lib/tests/integration.rs`
- Modify diagnostics/lowering as needed based on failures.

- [ ] **Step 1: Add failing integration tests**

Add tests for:

- `Option::Some` unwrap and `Option::None` early return.
- Diagnostic when `?` is used on `I64`.

Follow-up tests still needed for the fully generic design:

- `Result<T, E>` converting to `Result<U, F>` through `impl From E for F`.
- A custom carrier implementing `Try` and `FromResidual`.
- Diagnostic when returning `Option` from a `Result` function without a custom conversion.
- Drop cleanup on the early-return path.

- [ ] **Step 2: Verify tests fail or expose missing cases**

Run each test with `cargo test -p rock-lib --test integration <name> -- --exact`.

- [ ] **Step 3: Complete missing lowering/selection/diagnostic support**

Add only the compiler code required by the failing tests. Prefer existing trait-selection diagnostics and preserve spans on the `?` expression.

- [ ] **Step 4: Verify focused tests pass**

Run each new test exactly.

---

### Task 6: Full Verification And Commit

**Files:**
- All modified files.

- [ ] **Step 1: Format**

Run: `cargo fmt --all --check`

Expected: PASS. If it fails, run `cargo fmt --all`, then rerun the check.

- [ ] **Step 2: Diff whitespace check**

Run: `git diff --check`

Expected: no output.

- [ ] **Step 3: Focused full package tests**

Run: `cargo test -p rock-lib > /tmp/rock-lib-generic-try-tests.log 2>&1`

Expected: all tests pass.

- [ ] **Step 4: Commit and push**

Run:

```bash
git status --short
git add docs/superpowers/plans/2026-07-01-generic-try-short-circuit.md stdlib lib
git commit -m "feat: implement generic try short-circuit"
git push
```

Expected: branch `generic-try-short-circuit` is pushed and clean.

---

## Self-Review Notes

- Spec coverage: syntax, protocol, residual conversion, carrier impls, diagnostics, parser tests, integration tests, custom carrier, and cleanup are all mapped to tasks.
- Placeholder scan: no task is intentionally left as TBD; implementation steps name exact files and expected commands.
- Type consistency: plan uses `ControlFlow B, C`, `Try::Output`, `Try::Residual`, `FromResidual R`, and `From T` consistently with the design spec.
