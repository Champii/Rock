# Rock v0.2.1-develop

[![Rust](https://github.com/Champii/Rock/actions/workflows/rust.yml/badge.svg?branch=develop)](https://github.com/Champii/Rock/actions/workflows/rust.yml)

Little language made with Rust and LLVM.

Aim to follow the enforced safeness of the Rust model with a borrow checker (Soon™) and achieve high native performances thanks to LLVM.  
Rock is highly inspired from Livescript and Rust, and will also borrow (pun intended) some features from Crystal, from functional languages like Haskell, and even from Rust itself.

No to be taken seriously (yet)

# VTable
- [Features]( #features )
- [Install]( #install )
    - [Using released binary]( #using-released-binary )
    - [With cargo from Git]( #with-cargo-from-git )
    - [From sources]( #from-sources )
- [Quickstart]( #quickstart )
- [Showcases]( #showcases )
    - [Polymorphic function]( #polymorphic-function )
    - [Custom infix operator]( #custom-infix-operator )
    - [Trait definition]( #trait-definition )
- [REPL]( #repl )
- [Development notes]( #development-notes )

## Features

- Strongly typed
- Type inference
- Custom operators
- Typeclass (Traits)
- Polymorphism by default
- Compile to LLVM IR
- REPL (ALPHA)

## Install

Warning: This project has only been tested on Linux x86_64.

How to install and run the compiler:

### Using released binary 

You will need `clang` somewhere in your $PATH

Linux x86_64 only

[Rock v0.2.1-develop](https://github.com/Champii/Rock/releases/download/v0.2.1-develop/rock) (Tested on arch, btw)

``` sh
wget https://github.com/Champii/Rock/releases/download/v0.2.1-develop/rock
chmod +x rock
./rock -V
```

### From source

You will need `llvm-12.0.1` and `clang-12.0.1` somewhere in your $PATH

#### With cargo from git

``` sh
cargo install --git https://github.com/Champii/Rock --locked
rock -V
```

#### Manual clone and build from git

``` sh
git clone https://github.com/Champii/Rock.git
cd Rock
cargo run -- -V
```

## Quickstart

Lets create a new project folder to compute some factorials

``` sh
mkdir -P factorial/src && cd factorial
```

Add some files like this:

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
rock run
```

Should output

``` sh
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
rock run
```

Prints 

``` sh
1
2.2
Test
```

### Custom infix operator

``` haskell
infix |> 1
|> x f = f x

f a = a + 2

main = print (4 |> f)
```

``` sh
rock run
```

Prints `6`

### Trait definition

This `trait ToString` is redondant with the `trait Show` implemented in the lib, and serves as a demonstration only

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

Prints 

```
33
42.42
```

### Struct instance and Show implementation

``` haskell
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
Rock: v0.2.1-develop
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
