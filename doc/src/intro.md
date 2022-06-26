# Introduction

Rock is an experimental native language.

Its main goal is to mix some parts of popular functionnal languages like haskell or livescript with
the rigor of Rust while staying elegant and fast with minimal runtime

Rock is at an early development stage. Don't expect everything to work smoothly. (you have been warned)

```haskell
struct Player
  level: Int64
  name: String

impl Player
  new: x ->
    Player
      level: x
      name: "MyName"

impl Show Player
  @show: -> @name + "(" + @level.show! + ")"

use std::print::printl

impl Print Player
  @print: -> printl @

main: ->
  let player = Player::new 42
  player.print!
```

Rock syntax is entierly based on indentation, like Livescript.  
Each whitespace count, and tabulations `\t` are prohibited.  

The number of whitespace to make one level of indentation is taken from the first indent level.  
If your first indentation has two whitespaces, the rest of the file must have the same number of whitespace per level (here two)

We generally use two as a default, but you can use any number you want. Here is the same example with four whitespaces:

```haskell
struct Player
    level: Int64
    name: String

impl Player
    new: x ->
        Player
            level: x
            name: "MyName"
```

