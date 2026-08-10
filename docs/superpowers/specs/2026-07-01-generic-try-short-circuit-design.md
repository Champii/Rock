# Generic Try Short-Circuit Design

## Context

Rock has explicit `Result T, E` and `Option T` stdlib carriers, result/option combinators, associated types, trait bounds, and MIR/codegen support for early `return`. Fallible IO code currently nests `match Result::Ok/Err` blocks or chains `>>=`, which becomes hard to read when multiple resources need sequential setup.

The lexer already tokenizes `?` as `TokenType::Interogation`, the parser stores it as `SecondaryExpr::Interogation`, the formatter can print it, and AST visitors know about it. The current lowerer branch is only a placeholder: it drops the secondary by preserving the underlying expression kind and assigning a fresh type variable. This design gives that existing syntax real generic carrier short-circuit semantics backed by stdlib traits, not by hardcoded `Result` or `Option` names.

## Goals

- Add `expr?` as a postfix expression that unwraps successful carrier values and short-circuits residual values.
- Support generic carriers through a compiler-known stdlib protocol instead of special-casing `Result` and `Option` variants.
- Include residual conversion in the first version through a `FromResidual`-style trait.
- Add common `Result` error conversion through a small `From` trait.
- Keep the compiler responsible for control-flow effects and the stdlib responsible for carrier meanings.
- Provide diagnostics when `?` is used outside a compatible return context or without required trait implementations.
- Cover parser behavior and user-visible `Result`/`Option` behavior with tests.

## Non-Goals

- Do not add `try` blocks in this slice.
- Do not add async, generators, exceptions, or implicit function return-type inference changes.
- Do not hardcode stdlib operator symbols or fallback meanings in the compiler.
- Do not add a default `OptionResidual -> Result T, E` conversion without an explicit error value source.
- Do not make stdlib loading implicit beyond the existing explicit-stdlib/prelude rules.

## Surface Syntax

`?` is a postfix expression form:

```rock
value = fallible_expr?
```

It evaluates `fallible_expr` exactly once. On the carrier's continue path, the whole `expr?` evaluates to the carrier output. On the carrier's break path, the enclosing function or lambda returns immediately after converting the residual into its declared return carrier.

The operator should support clean call-chain error propagation without forcing parentheses around every whitespace call. In a call expression, a trailing `?` after the final argument applies to the whole call, not just the final argument:

```rock
file = File::open path?
count = file.read &mut buf?
item = iterator.next!?
value = maybe_vec?.get 0?
```

These examples are equivalent to:

```rock
file = (File::open path)?
count = (file.read (&mut buf))?
item = (iterator.next!)?
value = ((maybe_vec?).get 0)?
```

No-argument bang calls are call expressions too: `fn!?` unwraps the result of `fn!`. If the caller wants to unwrap a function value and then call it with no arguments, use `(fn?)!`.

This rule optimizes for the common fallible-call case. When the caller wants to unwrap an argument before passing it, the argument should be parenthesized:

```rock
value = parse_pair (read_left?), (read_right?)
```

When the caller wants to unwrap a function value before applying arguments, the callee should be parenthesized:

```rock
value = (make_parser?) input
```

The intended precedence is: receiver/primary secondaries, then whitespace call application, then call-result `?`, then binary operators and casts. Parentheses override this order. Unparenthesized `make_parser? input` should not be the preferred spelling for unwrapping a function value before a call; use `(make_parser?) input` for that case.

Receiver-chain unwrapping remains direct and readable:

```rock
vec = maybe_vec?
value = (vec.get 0)?
```

Parentheses can still be used when a larger non-call expression should be short-circuited:

```rock
value = (choose_result a, b)?
```

`?` is only valid where the compiler has an enclosing function or lambda return type. It is rejected in contexts where there is no return target.

## Stdlib Protocol

Add a public `stdlib::ops` module for control-flow carrier protocols:

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

Add small conversion traits to `stdlib::convert`:

```rock
< trait From T
    from: T -> Self

< trait Into T
    into: T
```

`From` is the canonical conversion implementation point. `Into` is available through a blanket implementation:

```rock
impl Into U for T where U: From T
    into = -> U::from *self
```

Users should normally implement `From Source for Target`; `source.into!` then works automatically. Direct explicit `Into` impls are not part of this design because they duplicate conversion paths and can conflict with the blanket impl.

`Try::branch` consumes the carrier. This matches `Result` and `Option` combinators that move the success payload out of the carrier.

`FromResidual` is an associated/static trait function. The compiler uses trait selection to find an implementation for the enclosing return type and the residual produced by the `?` expression.

## Carrier Residuals

Residuals are carrier-specific wrapper types, not raw payloads. This keeps conversions explicit and prevents accidental cross-carrier behavior.

In `stdlib::result`:

```rock
< enum ResultResidual E
    Err E
```

In `stdlib::option`:

```rock
< enum OptionResidual
    None
```

`Result T, E` implements `Try` with:

- `Output = T`
- `Residual = ResultResidual E`
- `Ok value -> ControlFlow::Continue value`
- `Err err -> ControlFlow::Break (ResultResidual::Err err)`

`Option T` implements `Try` with:

- `Output = T`
- `Residual = OptionResidual`
- `Some value -> ControlFlow::Continue value`
- `None -> ControlFlow::Break OptionResidual::None`

## Initial Conversions

Ship these stdlib conversions in the first implementation:

```rock
impl From T for T
    from = value -> value

impl Into U for T where U: From T
    into = -> U::from *self

impl FromResidual ResultResidual E for Result T, F where F: From E
    from_residual = residual ->
        match residual
            ResultResidual::Err err => Result::Err (F::from err)

impl FromResidual OptionResidual for Option T
    from_residual = residual ->
        match residual
            OptionResidual::None => Option::None
```

This supports identity `Result<T, E>? -> Result<U, E>` through `From E for E`, and supports error conversion `Result<T, E>? -> Result<U, F>` when `F: From E`.

No default `OptionResidual -> Result T, E` conversion is added. There is no payload to convert into `E`; users can define explicit `FromResidual OptionResidual for Result T, MyError` impls when their error type has a chosen `None` meaning.

## Compiler Semantics

For an enclosing function returning `R`, this source:

```rock
value = expr?
```

is conceptually equivalent to:

```rock
match expr.branch!
    ControlFlow::Continue value => value
    ControlFlow::Break residual => return R::from_residual residual
```

The compiler should not literally depend on user-written module imports for this expansion. It should resolve the protocol through language-item IDs recorded from the explicitly passed stdlib artifact, similar in spirit to existing drop language-item tracking.

If the stdlib is not available, or the required protocol items cannot be resolved, `?` should report a structured diagnostic instead of guessing a fallback meaning.

## Typing Rules

For an expression `expr?` inside a function or lambda with return type `R`:

- Infer the type of `expr` as `C`.
- Require `C: Try`.
- The type of `expr?` is `<C as Try>::Output`.
- Let the residual type be `<C as Try>::Residual`.
- Require `R: FromResidual <C as Try>::Residual`.
- Use normal trait selection and associated-type projection normalization for all constraints.

If `R` is not known at the point of lowering, the compiler should carry enough information to solve the constraints after inference rather than hardcoding the return type early.

## Lowering Strategy

Reuse the existing parsed `SecondaryExpr::Interogation` syntax, but replace the current lowerer placeholder with explicit HIR/MIR representation for try-short-circuit expressions. `?` has control-flow behavior, so treating it as an ordinary user-defined operator would be misleading and insufficient.

HIR should preserve the source span for diagnostics and store the selected protocol methods after resolution. The compiler must not special-case `Result` or `Option` during final `?` lowering; those carriers work because their stdlib impls satisfy the same `Try` and `FromResidual` protocol as custom carriers.

MIR lowering should create branch control flow equivalent to the conceptual `match`:

- Evaluate the carrier expression into a temporary.
- Call the selected `branch` method.
- Switch on `ControlFlow` variant.
- On `Continue`, bind/extract the output payload and continue in the current expression flow.
- On `Break`, call the selected `from_residual` conversion for the enclosing return type, store the function return value, run normal cleanup/drop paths, and return.

The early-return path must use the same cleanup/drop behavior as explicit `return` statements.

## Diagnostics

Diagnostics should be span-aware and point at the `?` token when possible.

Required cases:

- `?` outside a function or lambda body.
- Carrier type does not implement `Try`.
- Enclosing return type does not implement `FromResidual` for the carrier residual.
- Required stdlib protocol items are unavailable.
- `?` on a carrier whose `Try::branch` or associated types cannot be resolved unambiguously.

Messages should mention the carrier type, the required trait, and the enclosing return type when known.

## Examples

Result identity propagation:

```rock
parse_id: &Str -> Result I64, ParseError
load_id: &Str -> Result I64, ParseError
load_id = path ->
    text = read_text path?
    parse_id text
```

Result error conversion:

```rock
< enum AppError
    Io IoError

impl From IoError for AppError
    from = err -> AppError::Io err

load: &Str -> Result String, AppError
load = path ->
    file = File::open path?
    read_all file
```

Option propagation:

```rock
first_positive: [I64] -> Option I64
first_positive = values ->
    value = values.get 0?
    if value > 0
        Option::Some value
    else
        Option::None
```

Explicit Option-to-Result conversion remains user-defined:

```rock
< enum LookupError
    Missing

impl FromResidual OptionResidual for Result T, LookupError
    from_residual = residual ->
        match residual
            OptionResidual::None => Result::Err LookupError::Missing
```

## Testing

Parser tests should cover:

- `expr?` parses as a postfix expression.
- A trailing `?` after a whitespace call applies to the whole call: `foo bar?` parses as `(foo bar)?`.
- A trailing `?` after a no-arg bang call applies to the call result: `foo!?` parses as `(foo!)?`.
- Receiver-chain `?` composes before later secondaries: `maybe?.get 0` unwraps `maybe` before `.get`.
- Combined receiver and call `?` parses cleanly: `maybe?.get 0?` means `((maybe?).get 0)?`.
- Parenthesized argument `?` remains available: `foo (bar?)` unwraps `bar` before the call.
- Parenthesized callee `?` remains available: `(make_fn?) arg` unwraps the function value before the call.
- Unparenthesized callee unwrap before arguments is rejected or diagnosed: prefer `(make_fn?) arg` over `make_fn? arg`.
- `?` binds tighter than binary operators.
- `?` composes after method calls, field access, indexing, and parentheses.
- Invalid standalone `?` syntax produces a parse error.

Integration tests should cover:

- `Result` success path unwraps and continues.
- `Result` error path returns early.
- `Result` error conversion through `From`.
- `Option` `Some` path unwraps and continues.
- `Option` `None` path returns early.
- Custom carrier implementing `Try` and `FromResidual` works.
- Diagnostics for non-`Try` carrier and incompatible return carrier.
- Cleanup/drop behavior on the early-return path.

## Open Implementation Risks

- Static trait functions may need additional selection/lowering tests if current trait conformance primarily exercises receiver methods.
- Associated-type projection normalization must work in the return-carrier constraint `R: FromResidual <C as Try>::Residual`.
- Whitespace-call parsing must distinguish whole-call trailing `?` from parenthesized argument `?` without breaking existing call and method-chain parsing.
- MIR expression lowering must support an expression that can terminate the current function while still producing a value on the continue path.
- Formatting support for postfix `?` should be updated if the formatter relies on AST expression variants explicitly.
