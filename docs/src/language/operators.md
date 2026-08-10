# Operators

Rock does not assign permanent meanings to punctuation. The current source program and its explicit dependencies declare operator precedence and provide the function or trait implementation selected for the operand types. The standard library supplies familiar arithmetic, comparison, Boolean, and functional operators through the prelude.

## Standard Operators

With the standard library prelude, ordinary operators are selected from the operand types.

```rock
main = ->
    sum: I64 = 2 + 3
    same: Bool = sum == 5
    low: I64 = 2
    high: I64 = 8
    inside: Bool = sum >= low && sum <= high
    sum.println!
    same.println!
    inside.println!
    0
```

The declared result types are `I64`, `Bool`, and `Bool`; the output is `5`, `true`, and `true`. Removing those local annotations would let the same operand and operator constraints infer the types. If no implementation matches the operand types, compilation fails instead of applying a hidden built-in conversion.

## Declaring Precedence and Defining a Function

An `infix` declaration gives a symbol a precedence from `0` through `255`. The function with the same symbol supplies its meaning.

```rock
infix 9 %%

%%: I64 -> I64 -> I64
%% = left, right -> left - right

main = ->
    result: I64 = 40 %% 2
    result.println!
    0
```

The declaration makes `%%` parse as an infix operator at precedence `9`; the function returns `left - right`, so the output is `38`. A dependency can transport its declarations, which is why two dependencies should not assign conflicting precedence to the same symbol.

## Trait-Selected Operators

An operator can be a trait method. This example defines a new operator and selects an implementation from the left operand's type and the right operand's type.

```rock
infix 9 %%

struct Boxed
    < value: I64

trait Combine Rhs
    @%%: Rhs -> I64

impl Combine Boxed for Boxed
    @%% = other -> @value + other.value

main = ->
    left = Boxed
        value: 40
    right = Boxed
        value: 2
    result: I64 = left %% right
    result.println!
    0
```

The receiver is `Boxed`, the right operand is `Boxed`, and the selected implementation returns `42`. The compiler does not need to know a special meaning for `%%`; the trait declaration and implementation provide it.

## Unary Operators

Prefix operators use the same type-directed model. The standard library provides negation for numeric values and logical not for booleans where an implementation exists.

```rock
main = ->
    value: I64 = 7
    condition: Bool = true
    negative: I64 = -value
    opposite: Bool = !condition
    negative.println!
    opposite.println!
    0
```

The output is `-7` and `false`. Custom unary-operator authoring is less complete than ordinary infix function definitions; use the shipped unary implementations for portable code today.

## Operator Sections

Parenthesized operator sections create functions. `(+ 2)` waits for its left operand and then adds `2`.

```rock
increment: I64 -> I64
increment = (+ 2)

main = ->
    result: I64 = increment 5
    result.println!
    0
```

`increment` has type `I64 -> I64`, and the output is `7`. Use an explicit lambda when the missing side or ownership is not obvious.

## Functional Operators

The prelude also exports standard-library operators for function application through a value, mapping, binding, and fallback. Each example declares its functions and concrete carrier types.

```rock
increment: I64 -> I64
increment = value -> value + 1

keep_even: I64 -> Option I64
keep_even = value ->
    if value % 2 == 0
        Option::Some value
    else
        Option::None

main = ->
    mapped: Option I64 = Option::Some 4 <&> increment
    bound: Option I64 = (Option::Some 4) >>= keep_even
    fallback: Option I64 = (Option::None <|> Option::Some 9)
    mapped.unwrap_or 0 |> (value -> value.println!)
    bound.unwrap_or 0 |> (value -> value.println!)
    fallback.unwrap_or 0 |> (value -> value.println!)
    0
```

The output is `5`, `4`, and `9`. `<&>` maps a function over `Option`, `>>=` calls a function that returns another `Option`, `<|>` chooses the first present value, and `|>` passes a value to a function. These meanings are standard-library definitions, not compiler fallbacks.

## Parsing and Common Mistakes

Function application binds before infix operators. Parenthesize a complete operator expression when it is an argument or when doing so makes ownership and order clearer.

```rock
double: I64 -> I64
double = value -> value * 2

main = ->
    first: I64 = double 2 + 2
    second: I64 = double (2 + 2)
    first.println!
    second.println!
    0
```

The first expression is `(double 2) + 2` and prints `6`; the second is `double (2 + 2)` and prints `8`. The parser does not infer the grouping a reader may have intended.

Operators are library design. Use a symbol for a compact operation with a stable, documented reading direction; use a named function for a domain action whose meaning is not obvious from punctuation.
