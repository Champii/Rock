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

```coffee
class Foo              # Class declaration
    bar :: Int         # Type Integer
    def: 32            # Default value (type inference)
    def2 :: Int: 32    # Default value (explicit type)
    f -> @bar + @def   # Method

add a, b -> a + b      # Polymorphism by default

main ->                # Main function
    x = 1
    y = 2

    add x, y           # Call to function

    a = Foo            # Class instance
        bar: 10        # 'def' property is ommited

    a.f()              # Returns 42
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
    rock [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    build      Build the current project directory
    compile    Compile given files
    help       Prints this message or the help of the given subcommand(s)
    run        Run the current project directory
```

### Compile

Compile each given files individualy, does not link them afterward.

```
Compile given files

USAGE:
    rock compile [FLAGS] [files]...

FLAGS:
    -a               Show ast
    -h, --help       Prints help information
    -i               Show the generated IR
    -V, --version    Prints version information

ARGS:
    <files>...    Files to compile
```

### Build (TODO)

Treat the current working directory as a project, and will descend recursively into it to compile each ".rk" file it encounters.
It will then link them together.

```
Build the current project directory

USAGE:
    rock build

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

### Run (TODO)

Same as 'build' but it runs the created binary afterwards.

```
Run the current project directory

USAGE:
    rock run

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

## TODO (by order):

- v0.1.0
    - Replace llvm-sys with Inkwell
    - escaped chars
    - immutable by default
    - mut keyword
    - operator precedence
    - floats
    - sub, mul div and mod
    - while/for_in
    - modules/multi-file
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
  - Ultra basic HIR-MIR
  - Typing + inference
  - Crate/Mod
  - Basic LLVM compile

## 2

### Todo Dependency graph
  - Modules
    - Path
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
