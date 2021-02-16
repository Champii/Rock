# Rock 0.0.1

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with safeness, no GC but a Borrow Checker instead, and of course native performances thanks to LLVM.  
It is highly inspired from Livescript, and will borrow some features and syntax from Crystal, from functional languages like Haskell, or even Rust itself.

## Features

- Strongly typed
- Type inference
- Parametric Polymorphism
- Compile to LLVM IR

## Example

```haskell
mod other_file

main = foo 42
foo a = bar a
bar a = a
```

## Usage

### General commands

```
#> cargo build
#> ./target/debug/rock -h
rock 0.0.1
Champii <contact@champii.io>
Simple toy language

USAGE:
    fock [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -a               Show ast
        --help       Prints help information
    -h               Show hir
    -i               Show the generated IR
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
    - escaped chars
    - immutable by default
    - mut keyword
    - operator precedence
    - floats
    - sub, mul div and mod
    - while/for_in
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
- Class
- Methods
- Simple 'for'
- Returnable statement

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
