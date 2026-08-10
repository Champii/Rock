# Curried `~>` Functions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit curried function and method definitions with `~>`, support partial application and bound method values, keep normal `->` functions unchanged, and require explicit `!` for zero-arg invocation.

**Architecture:** Parse and preserve `~>` on lambda/function definitions, then lower curried definitions to nested unary `Type::Function` shapes while leaving source type syntax written with `->`. Method selectors lower to bound callable values instead of auto-invoking, and runtime function values become real first-class callables backed by a closure-like `{code_ptr, env_ptr}` representation.

**Tech Stack:** Rust 2021, custom lexer/parser/AST/HIR lowering, inference/generalization, monomorphization, LLVM codegen via `inkwell`/LLVM 18.

---

## Scope Notes

- `extern` functions stay unchanged and cannot be curried in v1.
- Methods are included.
- Trait impl methods may be curried or not per impl.
- Because trait signatures do not encode curriedness, generic code typed only against a trait signature cannot rely on trait-method partial application in v1. Concrete impl-resolved trait method values are still in scope.
- This plan does not itself add a stdlib `map`; a proof-point task is listed at the end.

## Primary Files

- Parser and AST:
  - `lib/src/lexer/token.rs`
  - `lib/src/lexer/lexer.rs`
  - `lib/src/parser/engine/token_type.rs`
  - `lib/src/parser/items/primitives.rs`
  - `lib/src/ast/tree.rs`
  - `lib/src/parser/items/function_decl.rs`
  - `lib/src/fmt/decl.rs`
- Lowering and typing:
  - `lib/src/lower/function.rs`
  - `lib/src/lower/paths.rs`
  - `lib/src/lower/bodies.rs`
  - `lib/src/lower/control_flow/secondary.rs`
  - `lib/src/lower/collect/declarations.rs`
  - `lib/src/lower/collect/traits.rs`
  - `lib/src/lower/traits/defaults.rs`
  - `lib/src/lower/traits/conformance.rs`
  - `lib/src/lower/types_helpers/generalize.rs`
  - `lib/src/lower/types_helpers/type_vars.rs`
- Codegen and runtime callable representation:
  - `lib/src/codegen/types.rs`
  - `lib/src/codegen/mod.rs`
  - `lib/src/codegen/closures.rs`
  - `lib/src/codegen/expr/mod.rs`
  - `lib/src/codegen/expr/call.rs`
  - `lib/src/codegen/expr/access.rs`
  - `lib/src/codegen/stmt.rs`
- Cross-crate generic support:
  - `lib/src/crate_artifact/helpers.rs`
  - `lib/src/mono/external.rs`
  - `lib/src/mono/process.rs`
- Tests:
  - `lib/src/parser/items/tests/**`
  - `lib/tests/integration.rs`
  - `lib/src/crate_artifact/tests.rs`

### Task 1: Parse `~>` And Preserve Arrow Kind

**Files:**
- Modify: `lib/src/lexer/token.rs`
- Modify: `lib/src/lexer/lexer.rs`
- Modify: `lib/src/parser/engine/token_type.rs`
- Modify: `lib/src/parser/items/primitives.rs`
- Modify: `lib/src/ast/tree.rs`
- Modify: `lib/src/parser/items/function_decl.rs`
- Modify: `lib/src/fmt/decl.rs`
- Test: `lib/src/parser/items/tests/function_decl/test_parse_function_decl.rs`
- Test: `lib/src/parser/items/tests/ast_validation.rs`
- Test: lexer/parser token tests near `test_token_type_arrow.rs`

- [ ] **Step 1: Add a dedicated `~>` token**

```rust
pub enum TokenType {
    Arrow,
    CurriedArrow,
    FatArrow,
    // ...
}
```

- [ ] **Step 2: Lex `~>` before native operator parsing**

```rust
'~' if self.peek(1) == '>' => self.token(TokenType::CurriedArrow, 2),
'~' if self.peek(1).is_alphabetic() && self.peek(1).is_uppercase() => {
    self.native_operator()
}
```

- [ ] **Step 3: Store arrow kind on `LambdaDecl`**

```rust
pub enum LambdaArrowKind {
    Normal,
    Curried,
}

pub struct LambdaDecl {
    pub parameters: Vec<Pattern>,
    pub body: Block,
    pub arrow_kind: LambdaArrowKind,
}
```

- [ ] **Step 4: Parse both arrows**

```rust
(parameters, TokenType::Arrow.or(TokenType::CurriedArrow), lambda_block)
    .map(|(parameters, arrow, body)| LambdaDecl {
        parameters,
        body,
        arrow_kind: match arrow {
            TokenType::Arrow => LambdaArrowKind::Normal,
            TokenType::CurriedArrow => LambdaArrowKind::Curried,
            _ => unreachable!(),
        },
    })
```

- [ ] **Step 5: Keep shorthand lambdas normal and format the original arrow**

```rust
let arrow = match self.arrow_kind {
    LambdaArrowKind::Normal => "->",
    LambdaArrowKind::Curried => "~>",
};
```

- [ ] **Step 6: Add parser coverage**

Examples to add:

```rock
add = a, b ~> a + b
main = ->
    inc = x ~> x + 1
    0
```

- [ ] **Step 7: Run focused parser tests**

Run: `cargo test -p rock-lib test_parse_function_decl -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib inline_lambda -- --exact`
Expected: PASS

### Task 2: Make Definition Arrow Control Internal Function Type Shape

**Files:**
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/collect/declarations.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/traits/defaults.rs`
- Modify: `lib/src/lower/traits/conformance.rs`

- [ ] **Step 1: Add one helper for function type construction**

```rust
fn build_function_type(param_tys: &[Type], ret_ty: Type, arrow: LambdaArrowKind) -> Type {
    match arrow {
        LambdaArrowKind::Normal => Type::Function(param_tys.to_vec(), Box::new(ret_ty)),
        LambdaArrowKind::Curried => param_tys.iter().rev().fold(ret_ty, |acc, param| {
            Type::Function(vec![param.clone()], Box::new(acc))
        }),
    }
}
```

- [ ] **Step 2: Stop flattening every `A -> B -> C` signature unconditionally**

Replace the unconditional flattening in `lower_function_sig` with contextual interpretation when a definition exists.

- [ ] **Step 3: When a standalone signature is merged with a definition, interpret it using the definition arrow kind**

```rust
let func_type = build_function_type(&param_types, func.ret_type.clone(), fd.lambda.arrow_kind);
```

- [ ] **Step 4: Keep trait signatures flat**

Trait signatures stay `->`-only and describe the normal method shape. Do not try to infer curriedness from them.

- [ ] **Step 5: Register functions in scope with the computed type shape, not always `Function(params, ret)`**

Affected places include:
`collect_function_sig`
`collect_function_signature_only`
`collect_crate_declarations`
export alias registration

- [ ] **Step 6: Run focused lowering/signature tests**

Run: `cargo test -p rock-lib test_parse_fn_type -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib test_inline_program -- --exact`
Expected: PASS

### Task 3: Lower Curried Definitions To Nested Lambdas

**Files:**
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/bodies.rs`

- [ ] **Step 1: Change inline lambda lowering to branch on `arrow_kind`**

Normal:

```rust
Type::Function(param_types, Box::new(ret_type))
```

Curried:

```rust
x, y ~> body
// lowers to:
Lambda { params: [x], body: Block { expr: Lambda { params: [y], body } } }
```

- [ ] **Step 2: For top-level and impl functions, synthesize nested lambda-returning bodies for curried defs**

Shape:

```rust
add = a, b ~> a + b
// outer function params: [a]
// outer body returns lambda [b] -> a + b
```

- [ ] **Step 3: Preserve `self` correctly in curried methods**

Shape:

```rust
@add = x, y ~> ...
// outer method params: [self, x]
// outer body returns lambda [y] -> ...
```

- [ ] **Step 4: Leave zero-user-arg functions as zero-arg callables**

Do not auto-invoke anything during lowering.

- [ ] **Step 5: Do not add a new HIR node**

Use existing `HirExprKind::Lambda`, `Call`, and `MethodCall`.

- [ ] **Step 6: Run focused lambda and closure tests**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_lambdas -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_closure_capture -- --exact`
Expected: PASS

### Task 4: Lower Calls And Method Selectors With New Semantics

**Files:**
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs` if secondaries need cleaner lookahead

- [ ] **Step 1: For curried callees, fold comma arguments into repeated unary application**

```rust
f a, b, c
// lower to:
Call(
    Box::new(Call(
        Box::new(Call(Box::new(f), vec![a])),
        vec![b],
    )),
    vec![c],
)
```

- [ ] **Step 2: Keep normal functions on the existing single-call path**

```rust
f a, b
// stays:
Call(Box::new(f), vec![a, b])
```

- [ ] **Step 3: Remove the current zero-user-arg method auto-call behavior**

Delete the lowering rule that turns `obj.area` directly into `MethodCall(..., vec![])`.

- [ ] **Step 4: Lower method access to a bound callable value**

For a normal method:

```rust
obj.add
// lower to lambda capturing obj:
// x, y -> obj.add x, y
```

For a curried method:

```rust
obj.add
// lower to lambda capturing obj:
// x ~> y ~> obj.add x, y
```

For a zero-arg method:

```rust
obj.area
// lower to zero-arg callable value
// -> obj.area!
```

- [ ] **Step 5: Reuse the existing method lookup and auto-ref insertion logic**

The helper that finds methods for concrete receivers can stay authoritative.
The helper that resolves trait methods on typevars/generics should remain flat-only in v1.

- [ ] **Step 6: Add integration tests for method semantics**

Examples:

```rock
struct Counter
    value: I64

impl Counter
    @get = -> self.value
    @add = x, y ~> self.value + x + y
```

- [ ] **Step 7: Run focused method tests**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_impl_methods -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_method_chaining -- --exact`
Expected: PASS

### Task 5: Replace Raw Function Pointers With Real First-Class Callables

**Files:**
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/closures.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/expr/call.rs`
- Modify: `lib/src/codegen/expr/access.rs`
- Modify: `lib/src/codegen/stmt.rs`

- [ ] **Step 1: Represent `Type::Function` as a callable object in LLVM**

```rust
// conceptual layout
struct Callable {
    code_ptr: *u8,
    env_ptr: *u8,
}
```

- [ ] **Step 2: Compile lambdas into `(code_ptr, env_ptr)` instead of raw function pointers**

Captures move into heap-allocated env structs.
Remove the `closure_captures` side-table path.

- [ ] **Step 3: Make named functions as values produce empty-env callables**

`HirExprKind::Var(name)` should still fast-path direct calls when used as a direct known callee, but as a value it must materialize a callable object.

- [ ] **Step 4: Change indirect call lowering to unpack the callable**

```rust
let callable = compile_expr(func_expr)?;
let code_ptr = extract(callable, 0);
let env_ptr = extract(callable, 1);
// indirect ABI: fn(env_ptr, args...) -> ret
```

- [ ] **Step 5: Make `compile_field_access` field-only**

If lowering already turns methods into lambdas/bound callables, codegen field access should stop trying to invoke methods as a fallback.

- [ ] **Step 6: Keep direct known normal-function calls as an optimization**

`Call(Var(name), args)` for a non-curried, exact-arity known function can stay a direct LLVM call.

- [ ] **Step 7: Run focused closure tests**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_closures -- --exact`
Expected: PASS

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_higher_order_functions -- --exact`
Expected: PASS

### Task 6: Fix Cross-Crate Generic Callable Support

**Files:**
- Modify: `lib/src/crate_artifact/helpers.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mono/process.rs`
- Test: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Remove the hardcoded generic-function whitelist**

Current code only exports/loads:

```rust
matches!(name, "identity" | "const" | "apply" | "twice")
```

Replace it with actual generic detection.

- [ ] **Step 2: Export any cross-crate generic function that needs HIR for specialization**

This is required for eventual stdlib higher-order helpers like `map`.

- [ ] **Step 3: Ensure returned function types survive specialization**

Nested `Type::Function` values must be preserved through:
`extract_type_args_from_expr_type`
`monomorphize_with_type_args`
`process_expr`

- [ ] **Step 4: Add a cross-crate test with a generic higher-order helper**

Example shape:

```rock
// dep crate
apply = f, x -> f x
< apply
```

- [ ] **Step 5: Run focused artifact and crate tests**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib crate_artifact -- --nocapture`
Expected: PASS

### Task 7: Add User-Facing Integration Coverage

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add free-function currying tests**

Examples:

```rock
add = a, b ~> a + b
main = ->
    inc = add 1
    (inc 2).println!
    (add 1, 2).println!
    0
```

- [ ] **Step 2: Add zero-arg invocation tests**

Examples:

```rock
make = ~> 42
main = ->
    f = make
    f!.println!
    0
```

- [ ] **Step 3: Add bound method value tests**

Examples:

```rock
c = Counter
    value: 10
get = c.get
(get!).println!
```

- [ ] **Step 4: Add curried method tests**

Examples:

```rock
step = c.add 1
(step 2).println!
(c.add 1, 2).println!
```

- [ ] **Step 5: Add trait-impl concrete resolution tests**

Test only concrete receivers, not generic trait-bound partial application.

- [ ] **Step 6: Run the full library suite**

Run: `LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib`
Expected: PASS

- [ ] **Step 7: Format and final verification**

Run: `cargo fmt --all`
Expected: no diff after re-run

### Optional Follow-Up: Prove The `map (+2)` Story

**Files:**
- Optional modify: `stdlib/*.rk`
- Optional modify: `examples/*.rk`
- Optional test: `lib/tests/integration.rs`

- [ ] **Step 1: Decide whether `map` targets arrays or `Vec`**

Arrays are currently minimal; `Vec` may be the easier first target.

- [ ] **Step 2: Add one proof-point example**

Example target:

```rock
infix 1 |>
|> = x, f -> f x
```

- [ ] **Step 3: Add an integration test that exercises partial application through a higher-order helper**

This can be crate-local first if you want to avoid broad stdlib work in the same branch.

## Key Risks

- The biggest implementation risk is the callable runtime representation. Without replacing raw function pointers, partial application and bound methods will stay fragile.
- The biggest semantic compromise is trait methods: with no curriedness in trait signatures, generic trait-bound partial application stays out of v1.
- The biggest regression risk is method access, because current code auto-calls zero-arg methods in lowering and codegen.

## Recommended Execution Order

1. Task 1
2. Task 2
3. Task 3
4. Task 4
5. Task 5
6. Task 6
7. Task 7
8. Optional `map` follow-up
