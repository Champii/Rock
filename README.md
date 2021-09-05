# Rock 0.1.2

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with enforced safeness with a borrow checker and native performances thanks to LLVM.  
It's highly inspired from Livescript, and will borrow (pun intended) some features and syntaxes from Crystal, from functional languages like Haskell, or even from Rust itself.

## Features

- Strongly typed
- Parametric Polymorphism(Soon(tm))
- Type inference
- Custom operators
- Compile to LLVM IR

## Ongoing development

This project, its syntax and its APIs are subject to change at any moment. This is a personal project, please bear with me :)

## Quickstart and Example

Creating a new project folder

``` sh
mkdir -P new_project/src && cd new_project
```

Add some files like this:

`./src/std.rk`

``` haskell
infix + 4
infix * 5

trait Num a
    + a -> a -> a
    * a -> a -> a

impl Num Int64
    + a b = ~IAdd a b
    * a b = ~IMul a b

impl Num Float64
    + c d = ~FAdd c d
    * c d = ~FMul c d

infix == 3

trait Eq a
    == a -> a -> a

impl Eq Int64
    == e f = ~IEq e f

impl Eq Float64
    == g h = ~FEq g h

impl Eq Bool
    == i j = ~BEq i j

placeholder = 2.2 == 2.2

infix >> 2
>> x y = y

infix << 2
<< x y = x
```

`./src/main.rk`

```haskell
extern printf String -> Float64 -> Int64

mod std

use std::+
use std::*
use std::>>
use std::<<
use std::==

main =
    if true == (1 == 1)
    then 1 + 2 << printf "%f", (299.2 + 33.2)
    else printf "%f", (12.3 * 12.2) >> 23 * 2
```

Assuming you built Rock and put its binary in your PATH:

``` sh
rock run
```

## Usage

### General commands

```
#> cargo build
#> ./target/debug/rock -h
Rock 0.1.1
Simple toy language

USAGE:
    rock [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -a               Show ast
        --help       Prints help information
    -h               Show hir
    -i               Show the generated IR
    -t               Show tokens
    -V, --version    Prints version information

OPTIONS:
    -o <output-folder>        Choose a different output folder [default: ./build]
    -v <verbose>              Verbose level

SUBCOMMANDS:
    build    Build the current project directory
    help     Prints this message or the help of the given subcommand(s)
    run      Run the current project directory
```

