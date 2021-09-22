# Rock 0.1.4-dev

Little toy language made with Rust and LLVM.  
Aim to follow the Rust model with enforced safeness with a borrow checker and native performances thanks to LLVM.  
It's highly inspired from Livescript, and will borrow (pun intended) some features and syntaxes from Crystal, from functional languages like Haskell, or even from Rust itself.

# VTable
- [Features][Features]
- [Ongoing development][Ongoing development]

## [Features]

- Strongly typed
- Type inference
- Custom operators
- Typeclass (Traits)
- Parametric Polymorphism by default
- Compile to LLVM IR

## Ongoing development

This project, its syntax and its APIs are subject to change at any moment.  
This is a personal project, so please bear with me  
(Differently put: this is a big red hot pile of experimental garbage right now)

## Install

How to install and run the binary:

### Using released [binary (v0.1.3)](https://github.com/Champii/Rock/releases/download/v0.1.3/rock) (tested on arch linux)

``` sh
wget https://github.com/Champii/Rock/releases/download/v0.1.3/rock
chmod +x rock
./rock -V
```

### With cargo (from git)

``` sh
cargo install --git https://github.com/Champii/Rock
rock -V
```

### From sources

``` sh
git clone https://github.com/Champii/Rock.git
cd Rock
cargo run -- -V
```

## Quickstart and Example

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

# Polymophic function
id a = a

fact a =
    if a <= 1
    then 1
    else a * fact (a - 1)

main = print fact id 4
```

Assuming that you built Rock and put its binary in your PATH:

``` sh
rock run
```

Should output

``` sh
24
```

