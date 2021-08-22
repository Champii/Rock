# Rock 0.1.1

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

`./src/std.rk`

``` haskell
infix + 4
+ a b = ~Add a b

infix * 5
* a b = ~Mul a b

infix |> 1
|> c h = h c
```

`./src/main.rk`

```haskell
mod std

use std::*
use std::+
use std::|>

f a = a + 2
g a = a * 2

main =
    if true
    then g (1 + 2) |> f 
    else 2
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

