# Trait

The trait feature is very similar to what Rust have  
You can define some 'interfaces' that has some methods, you can then implement them for your types.

For example, we will implement a trait that double itself with the `+` operator

```haskell
trait CanDouble a
  double_me: a => a
```

We define a trait `CanDouble` that takes a generic type `a`, and declare a method `double_me` that takes a generic type `a` and return a value of the same type.

We can now implement the trait for any type we want, like `Int64`

```haskell
impl CanDouble Int64
  double_me: self -> self + self
```

We can then call this method:

```haskell
main: -> (2).double_me!.print!
```

This output
```
4
```

The `self` here have been desugared for lisibility but you could have written

```haskell
impl CanDouble Int64
  double_me: @ -> @ + @
```

Those are strictly equivalent, as `@` desugar to `self`

You could also have used the following notation to automatically inject the `self` parameter

```haskell
impl CanDouble Int64
  @double_me: -> @ + @
```

And if you wanted to mutate the `self`, as well as returning `self` for chainable capababilities:

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

