# PR 12 Review Comments Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address every unresolved inline review comment on GitHub PR #12, then reply to and resolve each review thread after verification.

**Architecture:** Treat the PR feedback as a set of small, verifiable cleanup tasks. Prefer stdlib-only fixes where the compiler already supports the requested syntax or semantics; only touch compiler code where a comment identifies compiler diagnostics or trait/default-method behavior. Comments that are already fixed or technically non-actionable still require verification, a factual GitHub thread reply, and thread resolution.

**Tech Stack:** Rock stdlib and examples, Rust compiler tests in `rock-lib`, GitHub CLI REST and GraphQL APIs for review replies and thread resolution.

---

## Required GitHub Review Thread Protocol

Every task below must end by replying to the relevant inline review comments and resolving their review threads after the code and tests for that task pass.

Use REST replies for inline comments:

```bash
gh api \
  --method POST \
  repos/Rock-lang-org/Rock/pulls/12/comments/<comment-id>/replies \
  -f body='<short factual fix summary>'
```

Use GraphQL to resolve the matching review thread:

```bash
gh api graphql \
  -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' \
  -f id='<thread-id>'
```

Do not resolve a thread before all task-specific verification passes. If no code change is made because the feedback is already addressed or should be declined for technical reasons, still run the verification listed in the task, reply with the reason, then resolve the thread.

## Review Thread Registry

| Comment | Thread | File | Required handling |
| --- | --- | --- | --- |
| `3453226069` | `PRRT_kwDOKw_Lbc6LTTb8` | `examples/mir_tests/shared_borrow_then_move.rk` | Verify the Holder workaround is gone and the test still fails for borrow-then-move. |
| `3453239060` | `PRRT_kwDOKw_Lbc6LTVqo` | `examples/matrix.rk` | Add string `+` ergonomics and update matrix output construction. |
| `3453245117` | `PRRT_kwDOKw_Lbc6LTWrc` | `examples/matrix.rk` | Covered by Task 2: replace verbose matrix `.concat` output with `+`. |
| `3453245857` | `PRRT_kwDOKw_Lbc6LTWz3` | `examples/showcase.rk` | Covered by Task 2: replace verbose showcase `.concat` output with `+`. |
| `3453247200` | `PRRT_kwDOKw_Lbc6LTXDJ` | `examples/string_utils.rk` | Covered by Task 2: replace verbose string-utils `.concat` output with `+`. |
| `3453312095` | `PRRT_kwDOKw_Lbc6LTiXV` | `lib/src/lower/traits/conformance.rs` | Verify conformance diagnostics now use `method_span`. |
| `3453313508` | `PRRT_kwDOKw_Lbc6LTim-` | `lib/src/lower/traits/conformance.rs` | Verify conformance diagnostics now use `method_span`. |
| `3453314391` | `PRRT_kwDOKw_Lbc6LTixP` | `lib/src/lower/traits/conformance.rs` | Verify conformance diagnostics now use `method_span`. |
| `3453435287` | `PRRT_kwDOKw_Lbc6LT3y9` | `stdlib/convert.rk` | Verify `string_to_int` directly returns `atol owned.as_ptr!`. |
| `3453436695` | `PRRT_kwDOKw_Lbc6LT4Cs` | `stdlib/convert.rk` | Verify `string_to_float` directly returns `atof owned.as_ptr!`. |
| `3453448829` | `PRRT_kwDOKw_Lbc6LT6H7` | `stdlib/eq.rk` | Remove unnecessary `(*self)` and `(*other)` parentheses in stdlib. |
| `3453462092` | `PRRT_kwDOKw_Lbc6LT8b-` | `stdlib/eq.rk` | Verify `&Str` equality no longer casts compared bytes to `I64`. |
| `3453464109` | `PRRT_kwDOKw_Lbc6LT8zk` | `stdlib/eq.rk` | Verify `&Str` inequality is handled by the default `!=`. |
| `3453479123` | `PRRT_kwDOKw_Lbc6LT_dJ` | `stdlib/eq.rk` | Add default `Eq.@!=` and remove duplicate impl bodies. |
| `3453489214` | `PRRT_kwDOKw_Lbc6LUBLs` | `stdlib/eq.rk` | Verify duplicate `Eq for &I64` style reference impls are absent. |
| `3453491888` | `PRRT_kwDOKw_Lbc6LUBpn` | `stdlib/hash.rk` | Remove unnecessary deref parentheses. |
| `3453508305` | `PRRT_kwDOKw_Lbc6LUEn9` | `stdlib/num.rk` | Remove unnecessary deref parentheses where not required by parser precedence. |
| `3453513416` | `PRRT_kwDOKw_Lbc6LUFih` | `stdlib/option.rk` | Replace `f (&val)` with `f &val` in `Option.inspect`. |
| `3453515072` | `PRRT_kwDOKw_Lbc6LUF2M` | `stdlib/option.rk` | Remove duplicate `Show.println` override. |
| `3453517430` | `PRRT_kwDOKw_Lbc6LUGSd` | `stdlib/ord.rk` | Remove unnecessary deref parentheses in the file. |
| `3453521440` | `PRRT_kwDOKw_Lbc6LUG_V` | `stdlib/raw_buffer.rk` | Remove unreachable post-`exit` pointer fallback in `with_capacity`. |
| `3453528091` | `PRRT_kwDOKw_Lbc6LUIJT` | `stdlib/result.rk` | Remove duplicate `Show.println` override. |
| `3453543534` | `PRRT_kwDOKw_Lbc6LUK5B` | `stdlib/show.rk` | Define default `Show.println` and remove redundant impl overrides. |
| `3453553760` | `PRRT_kwDOKw_Lbc6LUMuW` | `stdlib/string_type.rk` | Verify/answer that no portable non-printf libc integer-to-string function is available; keep manual `from_i64`. |
| `3492401617` | `PRRT_kwDOKw_Lbc6M_5AQ` | `stdlib/bitwise.rk` | Remove unnecessary deref parentheses across stdlib. |
| `3492407632` | `PRRT_kwDOKw_Lbc6M_6HN` | `stdlib/convert.rk` | Remove immediately-returned `out` temporary in `int_to_string`. |
| `3492408322` | `PRRT_kwDOKw_Lbc6M_6PV` | `stdlib/convert.rk` | Remove immediately-returned `out` temporary in `float_to_string`. |
| `3492427120` | `PRRT_kwDOKw_Lbc6M_9q_` | `stdlib/hash_map.rk` | Replace `~PtrOffset` call sites with pointer `+` syntax where not implementing pointer `+` itself. |
| `3492445021` | `PRRT_kwDOKw_Lbc6NAA2S` | `stdlib/show.rk` | Covered by Task 4: define `Show.println` once as a trait default. |

## File Map

- Modify `stdlib/string_type.rk`: string concatenation `+` overloads; remove `~PtrOffset` in `concat`; keep manual `from_i64` with technical rationale.
- Modify `stdlib/string.rk`: simplify `string_concat` through the new `+` overloads if useful.
- Modify `examples/matrix.rk`, `examples/showcase.rk`, `examples/string_utils.rk`: replace verbose `.concat` chains with `+` syntax.
- Modify `stdlib/convert.rk`: make `int_to_string` and `float_to_string` direct returns.
- Modify `stdlib/eq.rk`: add default `@!=`, remove duplicate `@!=` impls, remove deref parentheses, keep byte comparisons as `U8`.
- Modify `stdlib/hash.rk`, `stdlib/num.rk`, `stdlib/ord.rk`, `stdlib/bitwise.rk`, `stdlib/neg.rk`, `stdlib/not.rk`, `stdlib/show.rk`, `stdlib/option.rk`, `stdlib/result.rk`, `stdlib/net.rk`, `stdlib/vec.rk`, `stdlib/hash_map.rk`, `stdlib/raw_buffer.rk`: remove unnecessary deref parentheses, redundant println impls, and `~PtrOffset` call sites.
- Modify `lib/tests/integration.rs`: add or adjust focused integration coverage for string `+`, default `Show.println`, `Eq.@!=`, pointer-offset syntax call sites, and examples touched by review comments.
- Read-only verification for `lib/src/lower/traits/conformance.rs` and `examples/mir_tests/shared_borrow_then_move.rk`: already fixed; only add tests if existing coverage is insufficient.

---

### Task 1: Baseline Review State And Already-Fixed Threads

**Files:**
- Read: `examples/mir_tests/shared_borrow_then_move.rk`
- Read: `lib/src/lower/traits/conformance.rs:1392-1455`
- Read: `stdlib/convert.rk:29-39`
- Read: `stdlib/eq.rk:31-52`

- [ ] **Step 1: Verify current state for comments that appear already fixed**

Check these facts:

```bash
rg -n "struct Holder|mut x = Holder" examples/mir_tests/shared_borrow_then_move.rk
rg -n "span: method_span\.clone\(\)|span: method_span," lib/src/lower/traits/conformance.rs
rg -n "result = atol|result = atof|atol \(|atof \(" stdlib/convert.rk
rg -n "as I64" stdlib/eq.rk
```

Expected:
- No `Holder` match in `shared_borrow_then_move.rk`.
- `conformance.rs` has `span: method_span.clone()` for parameter mismatch diagnostics and `span: method_span` for return mismatch diagnostics.
- No `result = atol` or `result = atof`; direct `atol owned.as_ptr!` and `atof owned.as_ptr!` are present.
- No `as I64` casts in `stdlib/eq.rk` string byte comparisons.

- [ ] **Step 2: Run existing focused tests**

```bash
cargo test -p rock-lib --test integration test_stdlib_string_helpers_return_owned_values -- --exact
cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_move_rust_parity -- --exact
cargo test -p rock-lib selection::service -- --nocapture
```

- [ ] **Step 3: Reply and resolve already-fixed threads**

Reply with short factual messages and resolve:

```bash
# Holder workaround removed.
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453226069/replies -f body='Verified: the Holder wrapper is gone; this regression now uses a moved String directly and remains covered by the borrow-after-move failure test.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTTb8'

# Conformance spans.
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453312095/replies -f body='Verified: this diagnostic now uses the impl method span via method_span.clone().'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTiXV'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453313508/replies -f body='Verified: this diagnostic now uses the impl method span via method_span.clone().'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTim-'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453314391/replies -f body='Verified: the return-type mismatch diagnostic now uses the impl method span.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTixP'

# Direct parse return helpers.
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453435287/replies -f body='Verified: string_to_int now directly returns atol owned.as_ptr!.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LT3y9'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453436695/replies -f body='Verified: string_to_float now directly returns atof owned.as_ptr!.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LT4Cs'

# Eq byte casts and reference impl duplication.
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453462092/replies -f body='Verified: &Str equality compares U8 bytes directly; the I64 casts are gone.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LT8b-'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453489214/replies -f body='Verified: the duplicate primitive reference Eq impls are gone; autoref handles these calls through the base impls.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUBLs'
```

Do not commit in this task unless code or tests were changed.

---

### Task 2: String Concatenation Ergonomics

**Files:**
- Modify: `stdlib/string_type.rk:116-143`
- Modify: `stdlib/string.rk:24-27`
- Modify: `examples/matrix.rk:17-38`
- Modify: `examples/showcase.rk:83-85`
- Modify: `examples/string_utils.rk:25-33,64-67`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add a failing integration test for string `+` combinations**

Add this test near `test_stdlib_string_helpers_return_owned_values` in `lib/tests/integration.rs`:

```rust
#[test]
fn test_stdlib_string_add_operators() {
    let src = r#"
main = ->
    owned = String::from_str "Rock"
    suffix = String::from_str "!"
    a = "Hello, " + owned
    b = a + " language"
    c = b + suffix
    c.println!
    0
"#;

    let output = compile_and_run(src);
    assert_eq!(output.stdout, "Hello, Rock language!\n");
}
```

- [ ] **Step 2: Run test to verify it fails before implementation**

```bash
cargo test -p rock-lib --test integration test_stdlib_string_add_operators -- --exact --nocapture
```

Expected: FAIL because `&Str + String` and/or `String + String` are not both implemented.

- [ ] **Step 3: Implement missing `+` overloads and simplify `string_concat`**

Update `stdlib/string_type.rk` so the string addition impls are:

```rock
impl Add String for String
    type Output = String
    @+ = other -> self.concat other

impl Add &Str for String
    type Output = String
    @+ = other -> self.concat (String::from_str other)

impl Add String for &Str
    type Output = String
    @+ = other -> (String::from_str *self).concat other

impl Add &Str for &Str
    type Output = String
    @+ = other -> (String::from_str *self).concat (String::from_str other)
```

Update `stdlib/string.rk`:

```rock
string_concat: &Str -> &Str -> String
< string_concat = a, b -> a + b
```

- [ ] **Step 4: Update examples to use `+`**

Use this shape in `examples/matrix.rk`:

```rock
show_matrix = m ->
    "| " + (int_to_string m.a) + " " + (int_to_string m.b) + " |" .println!
    "| " + (int_to_string m.c) + " " + (int_to_string m.d) + " |" .println!

main = ->
    ...
    "det(A) = " + (int_to_string (m1.det!)) .println!
    "trace(A) = " + (int_to_string (m1.trace!)) .println!
```

Use this shape in `examples/showcase.rk`:

```rock
num_str = int_to_string dot
msg = "Result: " + num_str
msg.println!
```

Use this shape in `examples/string_utils.rk`:

```rock
reverse_ascii = s ->
    bytes = as_bytes s
    len = string_len s
    result = String::from_str ""
    i = len - 1
    while i >= 0
        result = result + (byte_to_ascii_str ((byte_at bytes, i) as I64))
        i = i - 1
    result

...

result = prefix + suffix
```

- [ ] **Step 5: Run focused tests and examples**

```bash
cargo test -p rock-lib --test integration test_stdlib_string_add_operators -- --exact --nocapture
cargo test -p rock-lib --test integration test_matrix_multiply -- --exact --nocapture
cargo test -p rock-lib --test integration test_showcase_example -- --exact --nocapture
```

If there is no existing `string_utils` integration test, compile it directly:

```bash
cargo run -p rockc -- --entry-file examples/string_utils.rk --extern-artifact stdlib=build/stdlib.rkca
```

- [ ] **Step 6: Reply and resolve string-concat threads**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453239060/replies -f body='Implemented: String concatenation now supports + for &Str/String combinations, and matrix output uses the concise + form.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTVqo'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453245117/replies -f body='Implemented with the same string + cleanup in matrix.rk.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTWrc'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453245857/replies -f body='Implemented: showcase.rk now uses &Str + String instead of the verbose concat chain.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTWz3'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453247200/replies -f body='Implemented: string_utils.rk now uses + for &Str/String concatenation.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LTXDJ'
```

- [ ] **Step 7: Commit**

```bash
git add stdlib/string_type.rk stdlib/string.rk examples/matrix.rk examples/showcase.rk examples/string_utils.rk lib/tests/integration.rs
git commit -m "fix: simplify string concatenation review feedback"
```

---

### Task 3: Default `Eq.@!=` And Eq Cleanup

**Files:**
- Modify: `stdlib/eq.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add a failing test for default `!=` dispatch**

Add this near other stdlib operator tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_stdlib_eq_default_not_equal() {
    let src = r#"
main = ->
    1 != 2 .println!
    1 != 1 .println!
    "abc" != "abd" .println!
    "abc" != "abc" .println!
    0
"#;

    let output = compile_and_run(src);
    assert_eq!(output.stdout, "true\nfalse\ntrue\nfalse\n");
}
```

- [ ] **Step 2: Run test to verify current behavior before cleanup**

```bash
cargo test -p rock-lib --test integration test_stdlib_eq_default_not_equal -- --exact --nocapture
```

Expected before code cleanup: it may pass because `@!=` is duplicated today. Keep the test anyway because the implementation step removes those duplicate impl bodies.

- [ ] **Step 3: Implement default `@!=` and remove duplicate bodies**

Update `stdlib/eq.rk`:

```rock
< trait Eq
    @==: &Self -> Bool
    @!=: &Self -> Bool
    @!= = other -> (self == other) == false

impl Eq for I64
    @== = other -> ~I64Eq *self, *other

impl Eq for I32
    @== = other -> ~I32Eq *self, *other

impl Eq for U8
    @== = other -> ~U8Eq *self, *other

impl Eq for F64
    @== = other -> ~F64Eq *self, *other

impl Eq for F32
    @== = other -> ~F32Eq *self, *other

impl Eq for Bool
    @== = other -> ~BoolEq *self, *other
```

Keep the existing `impl Eq for &Str @==` logic, but remove its explicit `@!=` body. Leave byte comparisons as `U8`:

```rock
while equal && i < len
    left = unsafe lhs[i]
    right = unsafe rhs[i]
    if left != right
        equal = false
    i = i + 1
```

- [ ] **Step 4: Verify no duplicate reference impls or byte casts remain**

```bash
rg -n "impl Eq for &[A-Za-z0-9]|as I64|@!=" stdlib/eq.rk
```

Expected:
- No duplicate `impl Eq for &I64`-style primitive reference impls.
- No `as I64` byte casts.
- Only the trait-level `@!=` remains.

- [ ] **Step 5: Run focused tests**

```bash
cargo test -p rock-lib --test integration test_stdlib_eq_default_not_equal -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_get_and_contains_borrow_probe_key_without_consuming -- --exact --nocapture
```

- [ ] **Step 6: Reply and resolve Eq threads**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453464109/replies -f body='Implemented: &Str now uses the trait default !=, and its equality path keeps direct U8 byte comparisons.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LT8zk'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453479123/replies -f body='Implemented: Eq now defines a default != in terms of ==, so impls only provide == unless they need specialization.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LT_dJ'
```

- [ ] **Step 7: Commit**

```bash
git add stdlib/eq.rk lib/tests/integration.rs
git commit -m "fix: default Eq inequality implementation"
```

---

### Task 4: Default `Show.println` And Remove Redundant Overrides

**Files:**
- Modify: `stdlib/show.rk`
- Modify: `stdlib/option.rk`
- Modify: `stdlib/result.rk`
- Modify: `stdlib/net.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add a focused default-println integration test**

Add this near the existing `Show` integration tests:

```rust
#[test]
fn test_stdlib_show_default_println() {
    let src = r#"
struct Named
    < value: I64

impl Show for Named
    @show = -> "Named(" + (int_to_string self.value) + ")"

main = ->
    n = Named
        value: 7
    n.println!
    Option::Some 3 .println!
    Result::Ok 4 .println!
    0
"#;

    let output = compile_and_run(src);
    assert_eq!(output.stdout, "Named(7)\nSome(3)\nOk(4)\n");
}
```

- [ ] **Step 2: Run test to verify it fails before adding the default**

```bash
cargo test -p rock-lib --test integration test_stdlib_show_default_println -- --exact --nocapture
```

Expected: FAIL because `Named` implements `show` but not `println`.

- [ ] **Step 3: Add the default method to `Show`**

Update the trait in `stdlib/show.rk`:

```rock
< trait Show
    @show: String
    @println: I32
    @println = ->
        out = self.show!
        puts out.as_ptr!
```

- [ ] **Step 4: Remove redundant `@println` methods**

Remove `@println` overrides from these impls unless a focused test proves a specialization is required:

- `impl Show for I64`
- `impl Show for I32`
- `impl Show for U8`
- `impl Show for F64`
- `impl Show for F32`
- `impl Show for Bool`
- `impl Show for String`
- `impl Show for Char`
- `impl Show for &[T] where T: Show`
- `impl Show for &Str`
- `impl Show for Option T where T: Show`
- `impl Show for Result T, E where T: Show, E: Show`
- `impl Show for SocketError`

After cleanup, `stdlib/show.rk` should have only the trait-level `@println` definition:

```bash
rg -n "@println" stdlib/show.rk stdlib/option.rk stdlib/result.rk stdlib/net.rk
```

Expected: only `stdlib/show.rk` contains `@println`, and it appears in the trait default.

- [ ] **Step 5: Remove now-unused imports**

Remove unused `stdlib::libc::puts` imports from `stdlib/option.rk`, `stdlib/result.rk`, and `stdlib/net.rk`. Remove `stdlib::libc::putchar` from `stdlib/show.rk` if no longer used.

- [ ] **Step 6: Run focused tests**

```bash
cargo test -p rock-lib --test integration test_stdlib_show_default_println -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_option_result_show_prints_payloads -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_option_show_owned_string_is_non_consuming -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_result_show_owned_string_is_non_consuming -- --exact --nocapture
```

- [ ] **Step 7: Reply and resolve Show/println threads**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453515072/replies -f body='Implemented: Option no longer overrides println; it uses the Show trait default.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUF2M'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453528091/replies -f body='Implemented: Result no longer overrides println; it uses the Show trait default.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUIJT'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453543534/replies -f body='Implemented: Show now defines the default println method, and redundant impl-level println methods were removed.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUK5B'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3492445021/replies -f body='Implemented: Show.println is now a trait default instead of being redefined by each impl.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6NAA2S'
```

- [ ] **Step 8: Commit**

```bash
git add stdlib/show.rk stdlib/option.rk stdlib/result.rk stdlib/net.rk lib/tests/integration.rs
git commit -m "fix: use default Show println"
```

---

### Task 5: Remove Unnecessary Deref Parentheses Across Stdlib

**Files:**
- Modify: `stdlib/bitwise.rk`
- Modify: `stdlib/eq.rk`
- Modify: `stdlib/hash.rk`
- Modify: `stdlib/num.rk`
- Modify: `stdlib/ord.rk`
- Modify: `stdlib/neg.rk`
- Modify: `stdlib/not.rk`
- Modify: `stdlib/show.rk`
- Modify: `stdlib/option.rk`
- Modify: `stdlib/hash_map.rk`
- Modify: `stdlib/string_type.rk`
- Modify: any other `stdlib/**/*.rk` file matching the verification grep

- [ ] **Step 1: Record current matches**

```bash
rg -n "\(\*[^)]*\)" stdlib --glob '*.rk'
```

Expected before implementation: matches in operator impls and show/hash files.

- [ ] **Step 2: Replace simple deref parentheses**

Apply these mechanical transformations where syntax remains unambiguous:

```text
(*self)      -> *self
(*other)     -> *other
(*key)       -> *key
(*stored_key)-> *stored_key
((*self) as T) -> (*self as T) only if parser requires grouping for cast; otherwise `*self as T`
String::from_str (*self) -> String::from_str *self
f (&val) -> f &val
```

Concrete target examples:

```rock
@| = other -> ~I64Or *self, other
@hash = -> *self as I64
@< = other -> ~I64Lt *self, *other
@show = -> int_to_string *self
```

For cast forms, if `*self as I64` fails to parse, use the least-cluttered necessary grouping `(*self as I64)` and document the exception in the GitHub reply.

- [ ] **Step 3: Run parser/stdlib focused tests**

```bash
cargo test -p rock-lib --test integration test_stdlib_show_default_println -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_eq_default_not_equal -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_option_methods -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_result_methods -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_get_and_contains_borrow_probe_key_without_consuming -- --exact --nocapture
```

- [ ] **Step 4: Verify remaining matches are justified**

```bash
rg -n "\(\*[^)]*\)" stdlib --glob '*.rk'
```

Expected: no matches. Any remaining match means this task is incomplete and must be fixed before replying to or resolving the deref-parentheses threads.

- [ ] **Step 5: Reply and resolve deref-parentheses threads**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453448829/replies -f body='Implemented: removed unnecessary dereference parentheses across the stdlib where the parser accepts the cleaner form.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LT6H7'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453491888/replies -f body='Implemented: removed the unnecessary dereference parentheses in hash.rk.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUBpn'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453508305/replies -f body='Implemented: removed unnecessary dereference parentheses in num.rk.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUEn9'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453513416/replies -f body='Implemented: removed the unnecessary parentheses around the borrowed inspect payload where parser syntax allows it.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUFih'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453517430/replies -f body='Implemented: removed unnecessary dereference parentheses in ord.rk.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUGSd'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3492401617/replies -f body='Implemented: removed unnecessary dereference parentheses across stdlib, including bitwise.rk.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6M_5AQ'
```

- [ ] **Step 6: Commit**

```bash
git add stdlib
git commit -m "style: remove redundant stdlib deref parentheses"
```

---

### Task 6: Remove Temporary Return Variables In Convert Helpers

**Files:**
- Modify: `stdlib/convert.rk:17-27`
- Verify: `lib/tests/integration.rs` existing `test_stdlib_string_helpers_return_owned_values` covers these helpers

- [ ] **Step 1: Update direct-return helpers**

Change `stdlib/convert.rk` to:

```rock
// Convert integer to string, returns an owned String
int_to_string: I64 -> String
< int_to_string = x -> String::from_i64 x

// Convert float to string, returns an owned String
float_to_string: F64 -> String
< float_to_string = x -> String::from_f64 x
```

- [ ] **Step 2: Verify no temporary `out` remains**

```bash
rg -n "out = String::from_(i64|f64)|< (int_to_string|float_to_string)" stdlib/convert.rk
```

Expected: direct function bodies only.

- [ ] **Step 3: Run focused test**

```bash
cargo test -p rock-lib --test integration test_stdlib_string_helpers_return_owned_values -- --exact --nocapture
```

- [ ] **Step 4: Reply and resolve convert temporary threads**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3492407632/replies -f body='Implemented: int_to_string now directly returns String::from_i64 x.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6M_6HN'
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3492408322/replies -f body='Implemented: float_to_string now directly returns String::from_f64 x.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6M_6PV'
```

- [ ] **Step 5: Commit**

```bash
git add stdlib/convert.rk
git commit -m "style: simplify convert return helpers"
```

---

### Task 7: RawBuffer Negative Capacity Cleanup

**Files:**
- Modify: `stdlib/raw_buffer.rk:25-43`
- Verify: `lib/tests/integration.rs` existing Vec and HashMap allocation tests cover this path

- [ ] **Step 1: Restructure `RawBuffer::with_capacity`**

Replace the typed `if` expression that exits then returns a fake pointer with an early guard statement:

```rock
unsafe with_capacity: I64 -> I64 -> RawBuffer T
unsafe with_capacity = elem_size, cap ->
    if cap < 0
        exit (1 as I32)

    bytes = if cap == 0
        1
    else
        checked_mul_i64 elem_size, cap

    layout = Layout
        size: bytes
        align: 1
    ptr = (checked_alloc layout) as *T

    RawBuffer T
        raw_ptr: ptr
        raw_cap: cap
        raw_bytes: bytes
```

- [ ] **Step 2: Verify the fallback cast is gone**

```bash
rg -n "0 as \*T|0 as \*U8|exit \(1 as I32\)" stdlib/raw_buffer.rk
```

Expected: `exit (1 as I32)` remains as the guard; fake pointer casts do not.

- [ ] **Step 3: Run focused allocation-backed tests**

```bash
cargo test -p rock-lib --test integration test_stdlib_vec_new_as_slice_uses_non_null_buffer -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_push_uses_non_null_buffer -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_rejects_zero_sized_values_on_insert -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_rejects_zero_sized_keys_on_insert -- --exact --nocapture
```

- [ ] **Step 4: Reply and resolve raw-buffer thread**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453521440/replies -f body='Implemented: RawBuffer::with_capacity now guards negative capacity before allocation and no longer has a fake pointer expression after exit.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUG_V'
```

- [ ] **Step 5: Commit**

```bash
git add stdlib/raw_buffer.rk
git commit -m "fix: clean up RawBuffer negative capacity path"
```

---

### Task 8: Replace `~PtrOffset` With Pointer `+` Syntax

**Files:**
- Modify: `stdlib/hash_map.rk`
- Modify: `stdlib/vec.rk`
- Modify: `stdlib/string_type.rk`
- Modify: `stdlib/net.rk`
- Inspect: `stdlib/num.rk`

- [ ] **Step 1: Record current `~PtrOffset` call sites**

```bash
rg -n "~PtrOffset" stdlib --glob '*.rk'
```

Expected before implementation: matches in `hash_map.rk`, `vec.rk`, `string_type.rk`, `net.rk`, and pointer operator implementation in `num.rk`.

- [ ] **Step 2: Replace non-operator-implementation call sites**

Use pointer `+` syntax anywhere outside the pointer `Add/Sub` implementation itself:

```rock
// string_type.rk
dest = buf + lhs_len

// hash_map.rk
unsafe drop_in_place (values_ptr + target_idx)
unsafe drop_in_place (values_ptr + i)
unsafe drop_in_place (keys_ptr + i)

// vec.rk
unsafe drop_in_place (ptr + i)
unsafe drop_in_place (self.raw.ptr! + i)

// net.rk
*((buf + 0) as *U16) = af_inet! as U16
*((buf + 2) as *U16) = htons (addr.port as U16)
*((buf + 4) as *U32) = htonl (ipv4_to_u32 addr.ip)
port = ntohs (unsafe *((buf + 2) as *U16))
ip = ntohl (unsafe *((buf + 4) as *U32))
```

Keep `~PtrOffset` inside `stdlib/num.rk` for the implementation of pointer `+`/`-`, because replacing that implementation with `+` would recurse back into itself.

- [ ] **Step 3: Verify only the pointer operator implementation still uses `~PtrOffset`**

```bash
rg -n "~PtrOffset" stdlib --glob '*.rk'
```

Expected: only `stdlib/num.rk` remains.

- [ ] **Step 4: Run focused pointer-backed tests**

```bash
cargo test -p rock-lib --test integration test_stdlib_hash_map_drops_keys_and_values -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_overwrite_drops_old_value_and_unused_key_once -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_growth_drops_moved_keys_and_values_once -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_drops_initialized_elements -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_vec_set_drops_replaced_element_once -- --exact --nocapture
```

- [ ] **Step 5: Reply and resolve PtrOffset thread**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3492427120/replies -f body='Implemented: stdlib call sites now use pointer + syntax instead of ~PtrOffset. The intrinsic remains only inside the pointer + operator implementation to avoid recursion.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6M_9q_'
```

- [ ] **Step 6: Commit**

```bash
git add stdlib/hash_map.rk stdlib/vec.rk stdlib/string_type.rk stdlib/net.rk
git commit -m "style: use pointer addition syntax in stdlib"
```

---

### Task 9: Integer Formatting Libc Question

**Files:**
- Read: `stdlib/string_type.rk:44-95`
- Optionally modify: `stdlib/string_type.rk:44-45` only if adding a short explanatory comment is desired

- [ ] **Step 1: Verify current libc use and portability constraint**

Check that `from_f64` already uses `gcvt`, and that `from_i64` is the only manual formatter:

```bash
rg -n "from_i64|from_f64|gcvt|sprintf|snprintf|itoa|ltoa" stdlib/string_type.rk stdlib/libc.rk
```

Expected:
- `from_f64` uses `gcvt`.
- No portable non-printf `itoa`/`ltoa` libc binding exists.
- `sprintf`/`snprintf` are printf-family functions and are excluded by the review comment.

- [ ] **Step 2: Decide whether to add an explanatory code comment**

Prefer no code comment unless this keeps getting revisited. If adding one, keep it short:

```rock
// There is no portable non-printf libc integer formatter, so keep I64 formatting here.
from_i64: I64 -> String
```

- [ ] **Step 3: Run focused string helper test**

```bash
cargo test -p rock-lib --test integration test_stdlib_string_helpers_return_owned_values -- --exact --nocapture
```

- [ ] **Step 4: Reply and resolve string formatter thread**

```bash
gh api --method POST repos/Rock-lang-org/Rock/pulls/12/comments/3453553760/replies -f body='Verified: from_f64 already uses gcvt. For I64 there is no portable non-printf libc formatter like itoa/ltoa, so the manual formatter stays to honor the no-printf-family constraint.'
gh api graphql -f query='mutation($id:ID!) { resolveReviewThread(input:{threadId:$id}) { thread { id isResolved } } }' -f id='PRRT_kwDOKw_Lbc6LUMuW'
```

- [ ] **Step 5: Commit if a code comment was added**

```bash
git add stdlib/string_type.rk
git commit -m "docs: note integer formatter portability"
```

Skip this commit if no file changed.

---

### Task 10: Final Verification And Unresolved Thread Audit

**Files:**
- Verify: all touched files

- [ ] **Step 1: Run formatting and diff checks**

```bash
cargo fmt --all --check
git diff --check
```

- [ ] **Step 2: Run focused stdlib suites touched by the review comments**

```bash
cargo test -p rock-lib --test integration test_stdlib_string_helpers_return_owned_values -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_string_add_operators -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_show_default_println -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_eq_default_not_equal -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_option_methods -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_result_methods -- --exact --nocapture
cargo test -p rock-lib --test integration test_stdlib_hash_map_get_and_contains_borrow_probe_key_without_consuming -- --exact --nocapture
cargo test -p rock-lib --test integration test_matrix_multiply -- --exact --nocapture
cargo test -p rock-lib --test integration test_showcase_example -- --exact --nocapture
```

- [ ] **Step 3: Run full library tests once focused tests pass**

```bash
cargo test -p rock-lib
```

- [ ] **Step 4: Audit remaining unresolved PR review threads**

```bash
gh api graphql \
  -f query='query($owner:String!, $repo:String!, $number:Int!) { repository(owner:$owner, name:$repo) { pullRequest(number:$number) { reviewThreads(first:100) { nodes { id isResolved comments(first:1) { nodes { databaseId path url body } } } } } } }' \
  -F owner=Rock-lang-org \
  -F repo=Rock \
  -F number=12 \
  --jq '.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved == false) | {thread_id:.id, comment_id:.comments.nodes[0].databaseId, path:.comments.nodes[0].path, url:.comments.nodes[0].url, body:.comments.nodes[0].body}'
```

Expected: no output for the 29 known threads. If any known thread remains unresolved, reply and resolve it before completion.

- [ ] **Step 5: Check final git status**

```bash
git status --short --branch
git log --oneline -10
```

Expected: only intended commits from this plan are ahead of the remote; unrelated pre-existing untracked docs, if any, remain untouched.
