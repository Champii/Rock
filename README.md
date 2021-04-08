# Rock 0.1.0

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with enforced safeness with a borrow checker and native performances thanks to LLVM.  
It's highly inspired from Livescript, and will borrow (pun intended) some features and syntaxes from Crystal, from functional languages like Haskell, or even from Rust itself.

## Features

- Strongly typed
- Custom operators
- Type inference
- Parametric Polymorphism
- Compile to LLVM IR

## Example

```haskell
mod other_file

infix + 0

+ a b = ~Add a b #Native Add

add a b = +(a, b)
add2 a = +(a, 2)
main = add(add2(2), 2) #6
```

## Usage

### General commands

```
#> cargo build
#> ./target/debug/rock -h
rock 0.1.0
Champii <contact@champii.io>
Simple toy language

USAGE:
    rock [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
        --help       Prints help information
    -a               Show ast
    -h               Show hir
    -i               Show the generated IR
    -t               Show tokens
    -V, --version    Prints version information

OPTIONS:
    -v <verbose>        Verbose level

SUBCOMMANDS:
    build    Build the current project directory
    help     Prints this message or the help of the given subcommand(s)
    run      Run the current project directory

```

## TODO (by order):

- v0.1.0
    - operator precedence
    - sub, mul div and mod
    - while/for_in
    - immutable by default
    - mut keyword
    - type aliasing
    - floats
    - enums
    - escaped chars
    - Technical
      - Return diagnostic list as error instead of a single one
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
- Type inference
- Parametric Polymophism
- Returnable statement
- Custom Operators

# Goal

```haskell
class Foo
    val :> Int

    @val ->

    bar -> 1

trait Num
    +: A -> A
    -: A -> A
    *: A -> A
    /: A -> A

impl Num for UInt_8: Int
    +: -> ~#compiler_add_uint_8 @, it

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

# NEW GLOBAL LONG TERM TODO

## 1
  - Use the simpliest syntax possible
  - Custom operators
  - If/Else

## 2

### Todo Dependency graph
  - Modules
    - Rename/mangle fns before codegen
  - Polymophism
    - Mark generic functions
    - Allow them to pass the typechecker if never called
  - Currying
    - Closure
        - LowLevel Structs
  - Pattern matching
  - Custom operators
    - Operator as Identifier (special syntax)
    - Infix notation 
    - Custom precedence definition
