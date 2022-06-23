# Self

## The `self` parameter

The `self` parameter is represented by the `@` symbol.

```haskell
impl CanDouble Int64
  @double_me: -> @ + @
```

This method desugars to:

```haskell
impl CanDouble Int64
  double_me: self -> self + self
```

Those are strictly equivalent, as `@` desugar to `self`
We can see that we also have a `@` at the start of the name of the method. This allows for auto-injection of the self-parameter:

## Auto-inject self parameter

The standard way of defining a self-method is to auto-inject the self parameter:

```haskell
impl CanDouble Int64
  @double_me: -> @ + @
```

## Auto-return self

And if you also wanted to return `self` for chainable capababilities:

```haskell
impl CanDouble Int64
  @double_me: @->
    @ = @ + @

main: ->
  let x = 2
  x.double_me!.double_me!.double_me!
  x.print!
```
Outputs
```
16
```

The `@->` automatically inject the `@` as last returned statement of the function's body.
This operator is only available when using the `@method_name:` notation

