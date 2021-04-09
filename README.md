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

## Ongoing development

This project, its syntax and its APIs are subject to change at any moment. This is a personal project, please bear with me :)

## Example

```haskell
mod other_file

infix + 5 #Operator declaration with precedence

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
    -o <output-folder>        Choose a different output folder [default: ./build]
    -v <verbose>              Verbose level

SUBCOMMANDS:
    build    Build the current project directory
    help     Prints this message or the help of the given subcommand(s)
    run      Run the current project directory

```

