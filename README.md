# Plasma 0.0.1

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with safeness, no GC but a Borrow Checker instead, and of course native performances thanks to LLVM.  
It is highly inspired from Livescript, and will borrow some features and syntax from Crystal, from functional languages like Haskell or even Rust itself.

## Features

- Strongly typed
- Type inference
- Parametric Polymorphism
- Compile to LLVM IR

## Usage

```
#> cargo build
#> ./target/debug/plasma run
```

## Exemple

```haskell
add: a, b -> a + b

main: ->
    x = 1
    y = 2
    add x, y
```

## TODO (by order):

- v0.1.0
    - comments
    - unmutable by default
    - mut keywork
    - returnable statement
    - operator precedence
    - floats
    - sub, mul div and mod
    - arrays
    - while/for
    - structs
    - methods
    - multi-file
    - enums
    - type aliasing
- v1.0.0
    - desugar
    - operator overload
    - pattern matching
    - macro
    - borrow checker
    - traits and impl
    - stdlib

# Done
- Functions
- If/ElseIf/Else
- Assignation
- Type inference
- Parametric Polymophism

# Goal

```haskell
class Foo
    val: Int

    @val ->

    bar: -> 1

trait Num
    +: a -> a -> a
    -: a -> a -> a
    *: a -> a -> a
    /: a -> a -> a

impl Num for Int
    +: (&self, other) -> ~#compiler_add self, other

impl Num for Foo
    +: (&self, other) -> self.val + other.val

main ->
  a = Foo 1
  b = Foo 2
  a + b
```
