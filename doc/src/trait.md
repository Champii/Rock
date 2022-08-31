# Trait

## Declaration

The trait feature is very similar to what Rust have  
You can define some 'interfaces' that has some methods, you can then implement them for your types.

For example, we will implement a trait that double itself with the `+` operator

```haskell
trait CanDouble
  @double_me: @
```

We define a trait `CanDouble` and declare a method `double_me` that returns the generic self type `@`.

## Implementation

We can now implement the trait for any type we want, like `Int64`

```haskell
impl CanDouble Int64
  @double_me: -> @ + @
```

We can then call this method:

```haskell
main: -> (2).double_me!.print!
```

This output
```
4
```

## Default method

You can define trait methods that have a default implementation.  
That means you don't have to reimplement it for each type, but you can override the default implementation with your own if you need it.

```haskell
trait CanDouble
  @double_me: -> @ + @

impl CanDouble Int64

main: -> (2).double_me!.print!
```

### Overriding

You can override a default implementation

```haskell
trait CanDouble
  @double_me: -> @ + @

impl CanDouble Int64
  @double_me: -> @ * 2

main: -> (2).double_me!.print!
```

