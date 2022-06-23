# Rock v0.4.2

[![Rust](https://github.com/Champii/Rock/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/Champii/Rock/actions/workflows/rust.yml)

Native language made with Rust and LLVM.

Aim to follow the enforced safeness of the Rust model with a borrow checker (Soon™) and to achieve high native performances thanks to LLVM.  
Rock is highly inspired from [Livescript](https://livescript.net/), [Haskell](https://www.haskell.org/) and [Rust](https://www.rust-lang.org/).

No to be taken seriously (yet). Rock is still in its early conception phase, and everything can change and/or break at any time.  
Feel free to discuss any new feature or change you may like in an issue! We welcome and value every contribution.

## Index

- [Rock v0.4.2](#rock-v0.4.2)
  - [Index](#index)
  - [Features](#features)
  - [Install](#install)
    - [From source](#from-source)
      - [With Cargo from Git (Recommanded)](#with-cargo-from-git-recommanded)
      - [Manual Clone and Build from Git](#manual-clone-and-build-from-git)
    - [Using Released Binary](#using-released-binary)
  - [Quickstart](#quickstart)
  - [Showcases](#showcases)
    - [Polymorphic function](#polymorphic-function)
    - [Custom infix operator](#custom-infix-operator)
    - [Trait Definition](#trait-definition)
    - [Trait default method](#trait-default-method)
    - [Struct instance and methods]( #struct-instance-and-methods )
    - [Show and Print implementation]( #show-and-print-implementation )
    - [Modules and Code Separation](#modules-and-code-separation)
  - [Development notes](#development-notes)

---

## Features

- Strongly typed
- Type inference
- Custom operators
- Typeclass (Traits)
- Polymorphism by default
- Compile to LLVM IR

---

## Install

### From source

You will need `llvm-13` and `clang-13` somewhere in your $PATH

#### With Cargo from Git (Recommanded)

```sh
cargo install --locked --git https://github.com/Champii/Rock --tag v0.4.2
rock -V
```

#### Manual Clone and Build from Git

```sh
git clone https://github.com/Champii/Rock.git
cd Rock
cargo run --release -- -V
```

Note: If you clone and build manually, make sure to add `Rock/target/release/` to you `$PATH` so you can run it anywhere on your system.  
This method uses the `master` branch of Rock, that is not stable. You can checkout the latest version tag.

Rock has been tested against Rust stable v1.60.0 and nightly

[Adding Rust Nightly](https://github.com/Champii/Rock/wiki/Adding-Rust-Nightly)

### Using Released Binary

[Rock v0.4.2](https://github.com/Champii/Rock/releases/download/v0.4.2/rock)

``` sh
wget https://github.com/Champii/Rock/releases/download/v0.4.2/rock
chmod +x rock
./rock -V
```

This install method is not well tested yet, and might not work for your environment.
It requires a x86_64 architecture and GLIBC 2.34. (Don't try to upgrade your GLIBC if you don't know what you are doing)

---

## Quickstart

- Lets create a new project folder to compute some factorials

``` sh
rock new factorial && cd factorial
```

- Edit the `factorial/src/main.rk` file:

```haskell
fact: x ->
  if x <= 1
  then 1
  else x * fact (x - 1)

main: -> fact 4 .print!
```

Assuming that you built Rock and put its binary in your PATH:

``` sh
$ rock run
24
```

Rock should have produced a `./build/` folder, that contains your `a.out` executable.
You can execute it directly:

```sh
$ ./build/a.out
24
```

Take a look at `rock --help` for a quick tour of its flags and arguments

Note that you currently MUST be at the project root to run the compiler. (i.e. inside the `./factorial/` folder)

---

## Showcases

### Polymorphic function

``` haskell
id: x -> x

main: ->
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
`id: x -> x + x`  
We would have constrained `x` to types that implement [`Num`](https://github.com/Champii/Rock/blob/master/std/src/num.rk)

Note that this example would still be valid, as `Int64`, `Float64` and `String` are all implementors of `Num`.  
`String` is nowhere at its place here, and only implements `+` for string concatenation. This should change in the future with more traits like `Add` in rust

The output would be:

``` sh
2
4.4
TestTest
```

### Custom infix operator

``` haskell
infix |> 1
|>: x, f -> f x

f: x -> x + 2

main: -> (4 |> f).print!
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
  tostring: a => String

impl ToString Int64
  @tostring: -> @show!

impl ToString Float64
  @tostring: -> @show!

main: ->
  (33).tostring!.print!
  (42.42).tostring!.print!
```

``` sh
$ rock run
33
42.42
```

### Trait default method

``` haskell
trait ToString a
  @tostring: -> @show!

impl ToString Int64
impl ToString Float64

main: ->
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
  level: Int64
  name: String

impl Player
  new: level ->
    Player
      level: level
      name: "Default"
  @getlevel: -> @level

main: ->
  # The parenthesis are needed here because of a bug
  # with the chained dot notation in the parser
  Player::new(1)
    .getlevel!
    .print!
```

``` sh
$ rock run
1
```

### Show and Print implementation

``` haskell
struct Player
  level: Int64
  name: String

impl Show Player
  @show: -> @name + "(" + @level.show! + ")"

# This will be automatic in the future
impl Print Player

main: ->
  let player = Player
    level: 42
    name: "MyName"

  player.print!
```

``` sh
$ rock run
MyName(42)
```

Note that the `printl` method is defined in the stdlib as
```haskell
printl: x -> puts x.show!
```
with `puts` being an external from the `libc`

### Modules and code separation

- `./myproj/src/foo.rk`

```haskell
bar: x -> x + 1
```

- `./myproj/src/main.rk`

```haskell
mod foo

use foo::bar

main: -> bar 1 .print!
```

```sh
$ rock run
2
```

Note that we could have skiped the
`use foo::bar`
if we wrote
`main: -> foo::bar 1 .print!`

---

## Development notes

This project, its syntax and its APIs are subject to change at any moment.  
This is a personal project, so please bear with me

Differently put: this is a big red hot pile of experimental garbage right now
