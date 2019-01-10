# Plasma 0.0.1

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with safeness, no GC but a Borrow Checker instead, and of course native performances thanks to LLVM.  
Will highly inspired from Livescript, it will borrow some features and syntax from functional languages like Haskell or even Rust itself.

## Features

- Strongly typed
- Type infered
- Parametric Polymorphism
- Compile to LLVM IR

## Usage

```
#> cargo build
#> ./target/debug/plasma run
```

## Exemple

```haskell
add a, b -> a + b

main ->
    x = 1
    y = 2
    add x, y
```

## TODO (by order):

- returnable statement
- operator precedence
- sub, mul div and mod
- while/for
- comments
- structs
- methods
- arrays
- multi-file
- operator overload
- desugar
- pattern matching
- enums
- type aliasing
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

    (@val) ->

    bar: -> 1

impl Num for Foo
    +: (&self, other) -> self.val + other.val

main ->
  a = Foo 1
  b = Foo 2
  a + b
```
