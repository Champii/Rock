# Function signature

## Syntax

```haskell
foo: a => Int64 => String
foo: x, y -> "bar"
```

Here, the `foo` function takes 2 arguments, of type `a` (generic) and `Int64`, and returns a `String`
The implementation ignore the two arguments and returns "bar"

A signature is always formed of at least one type.  
The last (or only) type is the return type

Functions can take functions as parameter, and must be representable in a signature:

```haskell
my_func: a => (a => b) => b
my_func: x, f -> f x

# A function of type (a => b) that resolves to (Int64 => String)
handler: x -> x.show!

main: -> my_func 42, handler .print!
```

Here the second argument of the function `my_func` is a function that take a generic type `a` and returns a type `b`, and the whole method returns a type `b`

This outputs:

```
42
```

