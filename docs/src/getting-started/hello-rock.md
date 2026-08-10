# Hello, Rock!

Create a directory with this `rock.toml` manifest:

```toml
[crate]
name = "hello"
version = "0.1.0"

[lib]
path = "main.rk"
```

Create `main.rk` beside it with this complete program:

```rock
main = ->
    "Hello, Rock!".println!
    0
```

Run it from the project directory:

```console
$ rock run
Hello, Rock!
```

The output contains one line, and the process exits with status zero.

## Reading the program

`main = ->` declares a function named `main` with no parameters. The arrow separates its parameter list from its body. The extra indentation makes the next two lines part of that body.

The string literal is borrowed string data. `.println!` calls the prelude's printing method with no explicit arguments. The final `0` is the value returned by `main` and is conventionally the success status of a command-line program.

Calls with arguments use spaces and commas:

```rock
main = ->
    maximum = max 10, 20
    maximum.println!
    0
```

`max` is a prelude function. The first argument is `10`, the second is `20`, and the result is bound to `maximum` before it is printed.

## Expressions and statements

A one-expression function can stay on one line or use an indented body:

```rock
square = number ->
    number * number

main = ->
    (square 5).println!
    0
```

The final expression of `square` is its return value. Earlier expressions can perform effects before the final value:

```rock
announce_square = number ->
    "squaring a number".println!
    number * number

main = ->
    (announce_square 5).println!
    0
```

An explicit `return` exits before the end of the block:

```rock
absolute = number ->
    if number >= 0
        return number
    0 - number

main = ->
    (absolute (0 - 5)).println!
    0
```

For `-5`, the condition is false, so execution reaches `0 - number` and produces `5`. For a nonnegative value, `return number` skips the remaining expression.

## Comments

Rock supports line and block comments:

```rock
main = ->
    // This line explains why the next value is printed.
    /* A block comment can
       cover several source lines. */
    "comments do not produce values".println!
    0
```

Comments are ignored by the compiler. Use them for intent, constraints, or a non-obvious reason, not as a translation of every line.

## Common mistakes

- Leaving out the final `0` from `main` when the body otherwise returns `Unit`.
- Writing `println("text")`; the current call form is `"text".println!`.
- Indenting a statement at the same level as `main = ->`; that makes it a separate top-level item instead of part of `main`.
- Running `rock` outside the directory containing `rock.toml`.
