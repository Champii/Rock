# Rock {version}

[![Rust](https://github.com/Champii/Rock/actions/workflows/rust.yml/badge.svg?branch={branch})](https://github.com/Champii/Rock/actions/workflows/rust.yml)

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with enforced safeness with a borrow checker and native performances thanks to LLVM.  
It's highly inspired from Livescript, and will borrow (pun intended) some features and syntaxes from Crystal, from functional languages like Haskell, or even from Rust itself.

# VTable
- [Features]( #features )
- [Install]( #install )
    - [Using released binary]( #using-released-binary )
    - [With cargo from Git]( #with-cargo-from-git )
    - [From sources]( #from-sources )
- [Quickstart]( #quickstart )
- [Showcases]( #showcases )
- [Development notes]( #development-notes )

## Features

- Strongly typed
- Type inference
- Custom operators
- Typeclass (Traits)
- Parametric Polymorphism by default
- Compile to LLVM IR

## Install

Warning: This project has only been tested on Linux x86_64.

How to install and run the compiler:

### Using released binary 

Linux x86_64 only

[Rock {version}](https://github.com/Champii/Rock/releases/download/{version}/rock) (Tested on arch, btw)

``` sh
wget https://github.com/Champii/Rock/releases/download/{version}/rock
chmod +x rock
./rock -V
```

### From source

You will need LLVM12.0.1 and Clang12.0.1 somewhere in your $PATH

#### With cargo from git

``` sh
cargo install --git https://github.com/Champii/Rock
rock -V
```

#### Manual checkout and build from git

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

- Copy the std lib files from [std](https://github.com/Champii/Rock/blob/master/std/src) into `./src/`

- Create a `./src/main.rk` file:

```haskell
mod lib

use lib::prelude::*

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

## Showcases

### Polymophic function


``` haskell
mod lib

use lib::prelude::*

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
mod lib

use lib::prelude::*

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
mod lib

use lib::prelude::*

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
mod lib

use lib::prelude::*

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

## Development notes

This project, its syntax and its APIs are subject to change at any moment.  
This is a personal project, so please bear with me  
(Differently put: this is a big red hot pile of experimental garbage right now)
