# Function signature

## Syntax

```haskell
trait MyTrait a
  my_func: a => Int64 => String
```

Here, the `my_func` method takes 2 arguments of type `a` (generic) and `Int64`, and returns a `String`

A signature is always formed of at least one type.  
The last (or only) type is the return type

