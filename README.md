# Rock v0.2.2

[![Rust](https://github.com/Champii/Rock/actions/workflows/rust.yml/badge.svg?branch=-)](https://github.com/Champii/Rock/actions/workflows/rust.yml)

Little language made with Rust and LLVM.

Aim to follow the enforced safeness of the Rust model with a borrow checker (Soon™) and achieve high native performances thanks to LLVM.  
Rock is highly inspired from Livescript and Rust, and will also borrow (pun intended) some features from Crystal, from functional languages like Haskell, and even from Rust itself.

No to be taken seriously (yet)

---

## Index

- [Rock v0.2.2](#rock-v022)
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
    - [Struct instance and Show implementation](#struct-instance-and-show-implementation)
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

[Rock v0.2.2](https://github.com/Champii/Rock/releases/download/v0.2.2/rock) (Tested on arch, btw)

``` sh
wget https://github.com/Champii/Rock/releases/download/v0.2.2/rock
chmod +x rock
./rock -V
```

### From source

You will need `llvm-12.0.1` and `clang-12.0.1` somewhere in your $PATH

You will also want the nightly channel added for Rust.

#### Adding Rust Nightly

To check if you already have the nightly channel added for Rust, use:

```sh
rustup show
```

This will give you infomation about which build channels you have for rust and which one is active. If you have the nightly, you should see something like this:

```sh
Default host: x86_64-unknown-linux-gnu
rustup home:  /home/<username>/.rustup

installed toolchains
--------------------

stable-x86_64-unknown-linux-gnu (default)
nightly-x86_64-unknown-linux-gnu

active toolchain
----------------

stable-x86_64-unknown-linux-gnu (default)
rustc 1.61.0 (fe5b13d68 2022-05-18)
```

If you don't see the nightly build you can add it using the following command:

```sh
rustup install nightly
```

This will add the option to use the nightly build of Rust for this and any other projects.
Note you don't have to switch to the nightly to be the active toolchain but can use it specific projects, see below.

#### With Cargo from Git

If your active toolchain is stable:

```sh
cargo +nightly install --git https://github.com/Champii/Rock --locked
rock -V
```

If your active rust toolchain is nightly:

``` sh
cargo install --git https://github.com/Champii/Rock --locked
rock -V
```

#### Manual Clone and Build from Git

If your active toolchain is stable:

```sh
git clone https://github.com/Champii/Rock.git rock
cd rock
cargo +nightly run --<release|debug> -- -V
```

If your active toolchain is nightly:
You can pick the release or debug build

``` sh
git clone https://github.com/Champii/Rock.git rock
cd rock
cargo run --<release|debug> -- -V
```

Note: If you clone and build manually, make sure to add `path-to-install/rock/target/<release|debug>/` to you `$PATH` so you can run it anywhere on your system.

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

main = print fact 4
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
  print id 1
  print id 2.2
  print id "Test"
```

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

main = print (4 |> f)
```

``` sh
$ rock run
6
```

You can create any operator that is made of any combination of one or more of `'+', '-', '/', '*', '|', '<', '>', '=', '!', '$', '@', '&'`  

Most of the commonly defined operators like `+`, `<=`, etc are already implemented by the [stdlib](https://github.com/Champii/Rock/tree/master/std) that is automaticaly compiled with every package.  
There is a `--nostd` option to allow you to use your own custom implementation.

### Trait Definition

This `trait ToString` is redondant with the `trait Show` implemented in the stdlib, and serves as a demonstration only

``` haskell
trait ToString a
  toString :: a -> String

impl ToString Int64
  toString x = show x

impl ToString Float64
  toString x = show x

main =
  print toString 33
  print toString 42.42

```

``` sh
rock run
```

Prints:

```sh
33
42.42
```

### Struct instance and Show implementation

```haskell
struct Player
  level :: Int64
  name :: String

impl Show Player
  show p = show p.name

main =
  let player = 
    Player
      level: 42
      name: "MyName"

  print player
```

``` sh
rock run
```

Prints `MyName`

### Modules and Code Separation

- `./myproj/src/foo.rk`

```haskell
bar a = a + 1
```

- `./myproj/src/main.rk`

```haskell
mod foo

use foo::bar

main = print bar 1
```

Prints `2`

Note that we could have skiped the
`use foo::bar`
if we wrote
`main = print foo::bar 1`

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
Rock: v0.2.2
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
