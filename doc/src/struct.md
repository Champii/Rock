# Struct

Here is the idiomatic way to declare and instantiate a structure

```haskell
struct Counter
  value: Int64
  name: String

impl Counter
  new: x ->
    Foo 
      value: x
      name: "Counter"

  @increment: @->
    @value = @value + 1

main: ->
  Foo::new(42)
    .increment!
    .value
    .print!
```

There is a lot going on here, lets split this chunk of code in something more easy to understand

```haskell
struct Counter
  value: Int64
  name: String
```

This is the declaration of a structure called `Counter` with two fields, `value` and `name`.

```haskell
impl Counter
  new: x ->
    Foo 
      value: x
      name: "Counter"

  @increment: @->
    @value = @value + 1
```

This is the implementation of the structure `Counter`. The first method `new` is the constructor,
it takes a single argument `x` and returns a new instance of `Counter` with the `value` set to `x` and the `name` set to `"Counter"`.

The second method is an instance method, it takes no arguments and returns itself  
The `@->` operator automatically return `@` aka `self`). It increments the value of `value` by one.

We could have written something more compact

```haskell
impl Counter
  new: x -> Foo value: x, name: "Counter"
  @increment: @-> @value = @value + 1

main: -> Foo::new 42 .increment!.value.print!
```

This is the main function, it creates a new instance of `Counter` with the `value` set to `42` and then calls the method `increment` on it, and finally prints the value of the `value` field

This program outputs

```sh
43
```

