# A First Project: FizzBuzz

FizzBuzz is a useful first project because it combines a value-producing function, an enum, conditions, pattern matching, a loop, and output without needing a large library. The rules are:

1. Visit the integers from 1 through 30.
2. Print `FizzBuzz` for a multiple of both 3 and 5.
3. Otherwise print `Fizz` for a multiple of 3.
4. Otherwise print `Buzz` for a multiple of 5.
5. Print the number for every other value.

## The complete program

Create a project manifest as `rock.toml`:

```toml
[crate]
name = "fizzbuzz"
version = "0.1.0"

[lib]
path = "main.rk"
```

Save the program as `main.rk` beside the manifest:

```rock
enum FizzBuzzValue
    Text &Str
    Number I64

fizzbuzz_value: I64 -> FizzBuzzValue
fizzbuzz_value = number ->
    if number % 15 == 0
        FizzBuzzValue::Text "FizzBuzz"
    else if number % 3 == 0
        FizzBuzzValue::Text "Fizz"
    else if number % 5 == 0
        FizzBuzzValue::Text "Buzz"
    else
        FizzBuzzValue::Number number

print_value: FizzBuzzValue -> I32
print_value = value ->
    match value
        FizzBuzzValue::Text text => text.println!
        FizzBuzzValue::Number number => number.println!

main = ->
    number = 1
    while number <= 30
        value = fizzbuzz_value number
        print_value value
        number = number + 1
    0
```

Run it from the project directory:

```console
$ rock run
```

The complete output is:

```text
1
2
Fizz
4
Buzz
Fizz
7
8
Fizz
Buzz
11
Fizz
13
14
FizzBuzz
16
17
Fizz
19
Buzz
Fizz
22
23
Fizz
Buzz
26
Fizz
28
29
FizzBuzz
```

## Start with the result type

The calculation has two possible shapes: a word or the original number. The enum gives both shapes one static type:

```rock
enum FizzBuzzValue
    Text &Str
    Number I64
```

`FizzBuzzValue` is the type. `Text` carries borrowed string data, while `Number` carries an `I64`. A constructor is qualified with the enum name:

```rock
enum FizzBuzzValue
    Text &Str
    Number I64

main = ->
    word = FizzBuzzValue::Text "Fizz"
    number = FizzBuzzValue::Number 7
    word_text = match word
        FizzBuzzValue::Text text => text
        FizzBuzzValue::Number value => "number"
    word_text.println!
    number_text = match number
        FizzBuzzValue::Text text => text
        FizzBuzzValue::Number value => "number"
    number_text.println!
    0
```

Both local bindings have the same enum type even though their payloads differ. The matches make the payload types visible: `text` is `&Str` in its arm and `value` is `I64` in its arm.

## Separate calculation from output

The signature states the contract before the definition:

```rock
enum FizzBuzzValue
    Text &Str
    Number I64

fizzbuzz_value: I64 -> FizzBuzzValue
fizzbuzz_value = number ->
    if number % 15 == 0
        FizzBuzzValue::Text "FizzBuzz"
    else if number % 3 == 0
        FizzBuzzValue::Text "Fizz"
    else if number % 5 == 0
        FizzBuzzValue::Text "Buzz"
    else
        FizzBuzzValue::Number number
```

The signature says that one `I64` becomes one `FizzBuzzValue`. The body does not print, so the calculation can be reused by another output function or a test. Each branch constructs the same enclosing enum type even though the selected variant differs.

The operator `%` computes a remainder and `==` produces a `Bool`. Checking 15 first is essential: 15 is divisible by both 3 and 5, so a 3-only branch placed first would hide the combined case.

## Recover the payload with `match`

The output function consumes one enum value and handles both variants:

```rock
enum FizzBuzzValue
    Text &Str
    Number I64

print_value: FizzBuzzValue -> I32
print_value = value ->
    match value
        FizzBuzzValue::Text text => text.println!
        FizzBuzzValue::Number number => number.println!
```

An arm pattern checks the variant and binds its payload at the same time. The arm body is evaluated only after its pattern succeeds. Listing both variants makes the output decision visible in one place.

## Repeat with `while`

The loop in the complete program has four data-flow steps on every iteration:

1. `number <= 30` is evaluated.
2. If it is true, `fizzbuzz_value number` creates an enum value.
3. `print_value value` matches and prints that value.
4. `number = number + 1` advances the state before the next condition check.

The current compiler permits ordinary reassignment of an existing local, so this counter does not need `mut`. `mut` is still meaningful for mutable borrows and mutable receiver calls; the bindings chapter describes that distinction precisely.

## A `for` version

Ranges exclude their upper bound. The same calculation can therefore use `1..31`:

```rock
enum FizzBuzzValue
    Text &Str
    Number I64

fizzbuzz_value: I64 -> FizzBuzzValue
fizzbuzz_value = number ->
    if number % 15 == 0
        FizzBuzzValue::Text "FizzBuzz"
    else if number % 3 == 0
        FizzBuzzValue::Text "Fizz"
    else if number % 5 == 0
        FizzBuzzValue::Text "Buzz"
    else
        FizzBuzzValue::Number number

print_value: FizzBuzzValue -> I32
print_value = value ->
    match value
        FizzBuzzValue::Text text => text.println!
        FizzBuzzValue::Number number => number.println!

main = ->
    for number in 1..31
        value = fizzbuzz_value number
        print_value value
    0
```

The loop pattern `number` receives each range element. There is no explicit increment because the range iterator advances it.

## Extract a repeated idea

The divisibility test is a reusable function with two arguments:

```rock
is_divisible: I64 -> I64 -> Bool
is_divisible = number, divisor ->
    number % divisor == 0
```

The curried signature reads as “take an `I64`, then another `I64`, and return `Bool`.” In a larger version of the project, a branch could say `if is_divisible number, 15` without changing the underlying calculation.

## Common mistakes

- Checking `% 3` before `% 15`, which changes the result for 15.
- Forgetting that `1..31` excludes 31 and therefore includes 30 as its final element.
- Returning a raw string from one branch and an integer from another; all branches still need one enclosing type.
- Forgetting the counter update in the `while` version, which makes the condition stay true forever.
- Assuming an enum payload is available outside its matching arm; the binding is introduced by the pattern.

The next chapters isolate each of these language features so that you can reason about the type and value flowing through every line.
