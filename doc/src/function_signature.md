# Function signature

## Syntax

```haskell
trait MyTrait a
  my_func: a => Int64 => String
```

Here, the `my_func` method takes 2 arguments of type `a` (generic) and `Int64`, and returns a `String`

A signature is always formed of at least one type.  
The last (or only) type is the return type

Functions can take functions as parameter, and must be representable in a signature:

```haskell
trait MyTrait a
  my_func: a => (a => b) => b
```

Here the first argument of the method `my_func` is the `self`, the second is a function that take the generic type `a` taken from the trait definition and returns a type `b`, and the whole method returns a type `b`

An implementation of this trait would be:

```haskell
impl MyTrait Int64
  my_func: f -> f @

# A function of type (a => b)
handler: x -> x.show!

main: -> (2).my_func handler .print!
```

This outputs:

```
2
```

