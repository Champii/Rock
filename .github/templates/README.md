# Rock {version}

[![Rust](https://github.com/Champii/Rock/actions/workflows/rust.yml/badge.svg?branch={branch})](https://github.com/Champii/Rock/actions/workflows/rust.yml)

Little language made with Rust and LLVM.

Aim to follow the enforced safeness of the Rust model with a borrow checker (Soon™) and achieve high native performances thanks to LLVM.  
Rock is highly inspired from Livescript and Rust, and will also borrow (pun intended) some features from Crystal, from functional languages like Haskell, and even from Rust itself.

No to be taken seriously (yet)


## Index

- [Rock {version}](#rock-{version})
  - [Index](#index)
  - [Features](#features)
  - [Install](#install)
    - [Using Released Binary](#using-released-binary)
    - [From source](#from-source)
      - [Adding Rust Nightly](#adding-rust-nightly)
      - [With Cargo from Git](#with-cargo-from-git)
      - [Manual Clone and Build from Git](#manual-clone-and-build-from-git)
  - [Quickstart](#quickstart)
  - [Showcases](#showcases)
    - [Polymorphic function](#polymorphic-function)
    - [Custom infix operator](#custom-infix-operator)
    - [Trait Definition](#trait-definition)
    - [Struct instance and methods]( #struct-instance-and-methods )
    - [Show implementation]( #show-implementation )
    - [Modules and Code Separation](#modules-and-code-separation)
  - [REPL](#repl)
  - [Development notes](#development-notes)

---

## Features

- Strongly typed
- Type inference
- Custom operators
- Typeclass (Traits)
- Polymorphism by default
- Compile to LLVM IR
- REPL (ALPHA)

---

## Install

Warning: This project has only been tested on Linux x86_64.

How to install and run the compiler:

### Using Released Binary 

You will need `clang` somewhere in your $PATH

Linux x86_64 only

[Rock {version}](https://github.com/Champii/Rock/releases/download/{version}/rock) (Tested on arch, btw)

``` sh
wget https://github.com/Champii/Rock/releases/download/{version}/rock
chmod +x rock
./rock -V
```

### From source

You will need `llvm-12.0.1` and `clang-12.0.1` somewhere in your $PATH

Rock has been tested against Rust stable v1.60.0 and nightly

[Adding Rust Nightly](https://github.com/Champii/Rock/wiki/Adding-Rust-Nightly)

#### With Cargo from Git

```sh
cargo install --git https://github.com/Champii/Rock --locked
rock -V
```

#### Manual Clone and Build from Git

```sh
git clone https://github.com/Champii/Rock.git rock
cd rock
cargo run --<release|debug> -- -V
```

Note: If you clone and build manually, make sure to add `/[...]/rock/target/<release|debug>/` to you `$PATH` so you can run it anywhere on your system.

## Quickstart

- Lets create a new project folder to compute some factorials

``` sh
mkdir -p factorial/src && cd factorial
```

- Create a `factorial/src/main.rk` file:

```haskell
fact a =
    if a <= 1
    then 1
    else a * fact (a - 1)

main = fact(4).print!
```

Assuming that you built Rock and put its binary in your PATH:

``` sh
$ rock run
24
```

Take a look at `rock --help` for a quick tour of its flags and arguments

Note that you currently must be at the project root to run the compiler. (i.e. inside the `./factorial/` folder)

## Showcases

### Polymorphic function

``` haskell
id a = a

main =
  id 1 .print!
  id 2.2 .print!
  id "Test" .print!
```

Prints 

``` sh
$ rock run
1
2.2
Test
```

The `id` function here is polymorphic by default, as we don't make any constraint on the type that we should take or return.  
If we did something like this  
`id a = a + a`  
We would have constrained `a` to types that implement [`Num`](https://github.com/Champii/Rock/blob/master/std/src/num.rk)

Note that this example would still be valid, as `Int64`, `Float64` and `String` are all implementors of `Num`(*).  

The output would be:

``` sh
2
4.4
TestTest
```

(*) `String` is nowhere at its place here, and only implements `+` for string concatenation. This should change in the future with more traits like `Add` in rust

### Custom infix operator

``` haskell
infix |> 1
|> x f = f x

f a = a + 2

main = (4 |> f).print!
```

``` sh
$ rock run
6
```

You can create any operator that is made of any combination of one or more of `'+', '-', '/', '*', '|', '<', '>', '=', '!', '$', '@', '&'`  

Most of the commonly defined operators like `+`, `<=`, etc are already implemented by the [stdlib](https://github.com/Champii/Rock/tree/master/std) that is automaticaly compiled with every package.  
There is a `--nostd` option to allow you to use your own custom implementation. 

### Trait definition

This `trait ToString` is redondant with the `trait Show` implemented in the stdlib, and serves as a demonstration only

``` haskell
trait ToString a
  tostring :: a -> String

impl ToString Int64
  @tostring = @show!

impl ToString Float64
  @tostring = @show!

main =
  (33).tostring!.print!
  (42.42).tostring!.print!
```

``` sh
$ rock run
33
42.42
```

### Struct instance and methods 

``` haskell
struct Player
  level :: Int64
  name :: String

impl Player
  new level =
    Player
      level: level
      name: "Default"
  @getlevel = @level

main =
  let player = Player::new 1
  player.getlevel!.print!
```

``` sh
$ rock run
1
```

### Show implementation

``` haskell
struct Player
  level :: Int64
  name :: String

impl Show Player
  @show = @name + "(" + @level.show! + ")"

impl Print Player
  @print = printl @show!

main =
  let player = Player
    level: 42
    name: "MyName"

  player.print!
```

``` sh
$ rock run
MyName
```

Note that the `printl` method is defined in the stdlib as
```haskell
printl a = c_puts a.show!
```
with `c_puts` being the `libc` `puts`

### Modules and code separation

- `./myproj/src/foo.rk`

```haskell
bar a = a + 1
```

- `./myproj/src/main.rk`

```haskell
mod foo

use foo::bar

main = bar 1 .print!
```

```sh
$ rock run
2
```

Note that we could have skiped the
`use foo::bar`
if we wrote
`main = foo::bar 1 .print!` 

## REPL

Only supports basic expressions for now.
Very unstable, very work in progress.

Be warned that for a given session, the whole code is re-executed at each entry.  
This includes I/O of all sorts (Looking at you, open/read/write in loops)

Note that the REPL expects to be run from the project root, and expects some version of the stdlib
to be available in the `./src` folder

You can start a REPL session with 

``` sh
rock -r
# OR
rock --repl
```

``` sh
Rock: {version}
----

Type ':?' for help

> add a b = a + b
> let x = 30
30
> let y = 12
12
> add x, y
42
> :t add
add: (Int64 -> Int64 -> Int64)
> _
```

## Development notes

This project, its syntax and its APIs are subject to change at any moment.  
This is a personal project, so please bear with me

Differently put: this is a big red hot pile of experimental garbage right now
