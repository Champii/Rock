# Structure

## Declaration:

Here is the idiomatic way to declare a structure

```haskell
struct Counter
  value: Int64
  name: String
```

This structure has two fields `value` and `name` with their types respectively `Int64` and `String`

## Implementation

You can attach some methods to the structure, for example here a class-method `new`
that takes a `x` and is used as a constructor

The `increment` method is an instance-method that takes nothing, increments the `value` field by `1` and returns `@`

```haskell
impl Counter
  new: x ->
    Counter 
      value: x
      name: "Counter"

  @increment: @->
    @value = @value + 1
```

You can learn more about the `@` parameter here: [Self](./self.md)

```haskell
main: ->
  Counter::new(41)
    .increment!
    .value
    .print!
```

This prints `42`

---

*Note: We could have written something more compact, but less readable

```haskell
impl Counter
  new: x -> Counter value: x, name: "Counter"
  @increment: @-> @value = @value + 1

main: -> Counter::new 41 .increment!.value.print!
```


