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

## Example

```coffee
class Foo
    bar :: Int
    def: 32
    f -> this.bar + this.def

add a, b -> a + b

main ->
    x = 1
    y = 2

    add x, y

    a = Foo
        bar: 10

    a.f()
```

## TODO (by order):

- v0.1.0
    - escaped chars
    - immutable by default
    - mut keywork
    - returnable statement
    - operator precedence
    - floats
    - sub, mul div and modg
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
- Arrays
- Assignation
- Type inference
- Parametric Polymophism
- Class (attributes only)

# Goal

```haskell
class Foo
    val: Int

    @val ->

    bar -> 1

trait Num
    +: A -> A
    -: A -> A
    *: A -> A
    /: A -> A

impl Num for Int
    +: -> ~#compiler_add @, it

impl Num for Foo
    +: -> Foo @val + it.val

class Iterator
    collec: [A]
    item: A
    idx: 0

    @collec ->

    next: -> 
        @item = @collec[@idx]
        @idx++
        @item

trait IntoIterator
    iter: Iterator

impl IntoIterator for Foo
    iter: -> Iterator @

main ->
  a = Foo 1
  b = Foo 2
  a + b
```
