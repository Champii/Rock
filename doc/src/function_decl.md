# Function

## Syntax

The function declaration format is:

```haskell
function_name: arg1, arg2, arg3 -> return_value
```

The function call format is:

```haskell
function_name arg1, arg2, arg3
```

But you can add parenthesis

```haskell
function_name(arg1, arg2, arg3)
```

Every Rock package must have a `./src/main.rk` file containing a `main` function

```haskell
main: -> "Hello World!".print!
```

You can call functions with no args with a bang `!` like the `.print!` above or with
explicit parenthesis like `.print()`  
But the idiomatic way is to avoid parenthesis as much as possible

```haskell
add: x, y -> x + y
main: -> add 2, 3
```

Rock does not allow for uppercase identifiers, so you should embrace the snake case.
Uppercase names are reserved for custom types like [Struct](./struct.md) or [Trait](./trait.md)

## Polymorphism

Every function in Rock is polymorphic by default, and only infer the types based on the caller and the return value of its body.
Multiple calls with different types will generate each corresponding function, just like the templates in C++ or in Rust, except the generic parameter is always implicit if no constraints have been made.

For example, lets declare the most polymorphic function of all, `id`:

```haskell
id: x -> x
```

This function takes a `x` argument and returns it. Here `x` can be of any type

```haskell
main: ->
  id 42
  id 6.66
  id "Hello"
  id my_custom_struct
```

The infered signature of the function is `id: a => a`, with `a` being any type

If we had changed the body of `id` to be:

```haskell
id: x -> x + x
```

the previous main would still work if all of the types implemented the `Num` trait from the stdlib, that provide implementation of `+` for the basic types



