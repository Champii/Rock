# Function declaration

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

