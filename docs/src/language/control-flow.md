# Control Flow

Rock's control-flow forms are expressions. They decide which work runs and, except for ordinary loop control, can produce a value for the surrounding expression.

## `if` expressions

The multiline form uses indentation:

```rock
category: I64 -> &Str
category = temperature ->
    if temperature < 0
        "freezing"
    else if temperature < 20
        "cool"
    else
        "warm"

main = ->
    (category 12).println!
    0
```

The condition must be `Bool`. The compiler evaluates conditions from top to bottom and evaluates only the selected branch. Because `category` returns `&Str`, every reachable branch must produce a compatible string type.

For short alternatives, `then` and `else` fit on one line:

```rock
absolute: I64 -> I64
absolute = value ->
    if value >= 0 then value else 0 - value

main = ->
    (absolute (0 - 7)).println!
    0
```

## `while`

Use `while` when the number of iterations depends on a condition:

```rock
sum_to_ten: I64
sum_to_ten = ->
    index = 0
    total = 0
    while index < 10
        total = total + index
        index = index + 1
    total

main = ->
    (sum_to_ten!).println!
    0
```

The condition is checked before each iteration. The body adds the current `index`, then increments it. When `index` reaches 10, the body stops and the final `total` becomes the function result. Plain reassignment is currently permitted, so neither counter needs `mut` here.

Rock has no built-in `++` or `--`; write the state transition explicitly.

## `for`

`for pattern in expression` iterates over a range, fixed array, or slice:

```rock
main = ->
    for number in 0..10
        number.println!
    0
```

The upper bound is exclusive, so this prints `0` through `9`. The pattern is bound for each iteration:

```rock
main = ->
    values: [I64; 3] = [10, 20, 30]
    for value in values
        value.println!
    0
```

The current iteration facilities are smaller than Rust's iterator ecosystem. A growable `Vec` is not directly the same as a fixed array; use its `as_slice!` view, a library traversal helper, or a `while` loop when a `Vec` is the source.

This complete example borrows a vector as a slice and iterates over that fixed view:

```rock
main = ->
    mut values: Vec I64 = Vec::new!
    values.push 4
    values.push 5
    values.push 6

    view: &[I64] = values.as_slice!
    for value in *view
        value.println!
    0
```

`values` owns its growable allocation. `as_slice!` borrows its initialized elements, and `*view` supplies the slice value accepted by `for`. The loop prints `4`, `5`, and `6`; ownership returns to `values` after the borrow's final use. Indexed `while` loops and consuming traversal methods are covered in [Arrays, Slices, and Tuples](arrays-slices-tuples.md) and [Strings and Collections](../stdlib/collections.md).

## `loop`, `break`, and `continue`

`loop` repeats until control leaves it. `break` leaves the nearest loop, and `continue` starts its next iteration:

```rock
main = ->
    index = 0
    loop
        index = index + 1
        if index == 2
            continue
        if index == 4
            break
        index.println!
    0
```

The output is `1` and `3`: the second iteration uses `continue`, and the fourth uses `break` before printing. Expressions after `break` and `continue` are accepted syntactically, but ordinary control statements have the strongest end-to-end coverage today.

## `match`

`match` is the natural control flow for enums and structural patterns:

```rock
enum Status
    Ready
    Waiting I64

label: Status -> &Str
label = status ->
    match status
        Status::Ready => "ready"
        Status::Waiting seconds if seconds > 10 => "late"
        Status::Waiting _ => "waiting"

main = ->
    (label Status::Ready).println!
    (label (Status::Waiting 20)).println!
    0
```

Arms are tested from top to bottom. The first `Waiting` arm handles values over 10; the second handles the remaining `Waiting` values. The enums chapter explains payload bindings, guards, and exhaustiveness in detail.

## Common mistakes

- Returning incompatible branch types from an `if` expression.
- Forgetting the update in a `while`, producing a condition that never changes.
- Expecting the upper endpoint of a range to be included.
- Putting a broad `match` pattern before a more specific guarded pattern.
- Using `break` or `continue` outside the nearest loop.
