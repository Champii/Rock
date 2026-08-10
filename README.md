# Rock

[![Book](https://github.com/Champii/Rock/actions/workflows/book.yml/badge.svg?branch=develop)](https://github.com/Champii/Rock/actions/workflows/book.yml)
[![Discord](https://img.shields.io/discord/990627124236939314.svg)](https://discord.gg/f6skPNB96J)

[Documentation](https://champii.github.io/Rock/)

A native, expression-oriented programming language built with Rust and LLVM.

Rock combines static typing and ownership with a compact functional syntax. It is inspired by [LiveScript](https://livescript.net/), [Haskell](https://www.haskell.org/), and [Rust](https://www.rust-lang.org/).

Rock is experimental. The language, compiler, and tooling can change or break at any time. Contributions and design discussions are welcome.

## Index

- [Features](#features)
- [Install](#install)
- [Quickstart](#quickstart)
- [Showcase](#showcase)
- [Tooling](#tooling)
- [Documentation](#documentation)

---

## Features

### Type system

- Strong static typing with local and generic type inference
- Algebraic data types, tuples, structs, enums, and exhaustive-style pattern matching
- Parametric polymorphism, trait bounds, associated types, and trait default methods
- Higher-kinded types and constructor bounds such as `F _: Functor`
- First-class functions, closures, callable structs, currying, and partial application
- User-defined infix operators with program-defined precedence and semantics

### Safety and systems programming

- Ownership, moves, shared references, mutable references, and borrow checking
- Explicit `unsafe` blocks for raw pointers, pointer arithmetic, and FFI boundaries
- Deterministic destruction through `Drop`
- Thread-safety contracts through `Send` and `Sync`
- Native code generation through LLVM 18
- C interoperability for building low-level libraries and platform bindings

### Functional programming

- `Option`, `Result`, and generic `?` short-circuiting through `Try`
- Standard `Functor`, `Bifunctor`, `Applicative`, `Monad`, `Foldable`, and `Traversable` traits
- Functional operators including `|>`, `<$>`, `<&>`, `<!>`, `<*>`, `>>=`, and `<|>`
- Effectful traversal and sequencing over `Option`, `Result`, and `Vec`
- Function-call holes and concise lambdas for point-free-style pipelines
- Expression-oriented `if`, `match`, loops, and blocks

### Programs and tooling

- Modules, explicit imports and exports, path dependencies, and reusable crate artifacts
- Declarative macros and macro expansion inspection
- A standard library with strings, vectors, hash maps, files, TCP networking, threads, atomics, `Arc`, and `Mutex`
- Project-aware build, run, format, expand, and artifact commands through `rock`

---

## Install

Rock currently targets `x86_64-unknown-linux-gnu`. Building from source requires Git, Rust with Cargo, LLVM 18 with shared libraries, and a C linker available as `cc`.

```console
$ git clone https://github.com/Champii/Rock.git
$ cd Rock
$ cargo build --release
$ target/release/rockup dev stdlib package \
    --path stdlib \
    --sysroot target/release
$ target/release/rockup toolchain install dev --path target/release
$ target/release/rockup default dev
```

Restart the shell when prompted so the installed `rock` command is available. Rock does not yet ship through a package registry or stable binary installer.

See the [installation guide](https://champii.github.io/Rock/getting-started/installation.html) for setup details and troubleshooting.

---

## Quickstart

Create a project with a manifest:

```console
$ mkdir hello-rock
$ cd hello-rock
```

`rock.toml`:

```toml
[crate]
name = "hello"
version = "0.1.0"

[lib]
path = "main.rk"
```

`main.rk`:

```haskell
main = ->
    "Hello, Rock!".println!
    0
```

Build and run it:

```console
$ rock run
Hello, Rock!
```

---

## Showcase

### Generic programming over type constructors

One function can map any unary constructor implementing `Functor`. The same implementation works for `Option`, `Result`, and `Vec`, including nested constructors.

```haskell
map_any: M -> F A -> F B where F _: Functor, M: FnMut A, B
map_any = mapper, value -> F::Functor::fmap mapper, value

increment: I64 -> I64
increment = value -> value + 1

double: I64 -> I64
double = value -> value * 2

make_values: () -> Vec I64
make_values = ->
    mut values = Vec::new!
    values.push 1
    values.push 2
    values.push 3
    values

map_values: Vec I64 -> Vec I64
map_values = values -> map_any double, values

main = ->
    option: Option I64 = map_any increment, (Option::Some 4)
    result: Result I64, I64 = map_any increment, (Result::Ok 5)
    vector: Vec I64 = map_any double, (make_values!)
    nested: Option (Vec I64) = map_any map_values, (Option::Some (make_values!))

    option.show!.println!
    result.show!.println!
    vector.show!.println!
    nested.show!.println!
    0
```

```text
Some(5)
Ok(6)
[2, 4, 6]
Some([2, 4, 6])
```

### Effectful validation and folding

`Traversable` turns a vector of fallible computations into one fallible vector. `traverse_m` short-circuits on the first error, while `Foldable` reduces the validated values without exposing storage details.

```haskell
validate_positive: I64 -> Result I64, I64
validate_positive = value ->
    if value > 0
        Result::Ok value
    else
        Result::Err value

sum_pair: (I64, I64) -> I64
sum_pair = pair -> pair.0 + pair.1

sum_values: Vec I64 -> I64
sum_values = values -> Vec::Foldable::foldl sum_pair, 0, values

validate_and_sum: Vec I64 -> Result I64, I64
validate_and_sum = values ->
    validated: Result (Vec I64), I64 =
        Vec::Traversable::traverse_m validate_positive, values
    validated <&> sum_values

make_values: () -> Vec I64
make_values = ->
    mut values = Vec::new!
    values.push 10
    values.push 20
    values.push 12
    values

main = ->
    total: Result I64, I64 = validate_and_sum (make_values!)
    total.show!.println!
    0
```

```text
Ok(42)
```

### Functional error pipelines

Operators are ordinary stdlib definitions rather than compiler special cases. Pipelines can map errors, sequence effects, transform successes, and propagate failures with `?`.

```haskell
parse_port: I64 -> Result I64, I64
parse_port = value ->
    if value > 0 && value <= 65535
        Result::Ok value
    else
        Result::Err value

normalize_port: I64 -> I64
normalize_port = port -> port + 1000

open_service: I64 -> Result I64, I64
open_service = requested ->
    port = parse_port requested?
    Result::Ok port

main = ->
    selected: Result I64, I64 =
        (open_service 8080 <&> normalize_port)
            <|> Result::Ok 9000

    selected
        .unwrap_or 0
        |> (port -> port.println!)
    0
```

```text
9080
```

More complete programs live in [`examples/`](examples/), [`test_projects/`](test_projects/), and the [language guide](https://champii.github.io/Rock/).

---

## Tooling

All application workflows use the project-aware `rock` command from a directory containing `rock.toml`:

```console
$ rock format
$ rock build
$ rock run -- first second
$ rock expand
$ rock artifact
```

- `format` rewrites the configured source file.
- `build` compiles the package and its path dependencies.
- `run` builds and executes the package; arguments after `--` go to the program.
- `expand` prints macro-expanded source.
- `artifact` materializes a reusable crate artifact.

There is currently no `rock test` command. Application tests are ordinary Rock programs or external harnesses; compiler contributors use the Rust integration suite.

---

## Documentation

The full book is published as [The Rock Programming Language](https://champii.github.io/Rock/). Its source is under [`docs/`](docs/) and can be built locally with:

```console
$ mdbook build docs
```

Compiler contributors can run the Rust test suite with:

```console
$ cargo test -p rock-lib
```
