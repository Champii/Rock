# Primitives

Rock actually support some essential primitives.

## Boolean

The `Bool` type

```haskell
main: -> 
  true.print!
  false.print!
```

## Integer

The most stable integer is `Int64` and should be used until the rest follows.

```haskell
main: -> 0
```

Rock has an inference mechanism that automatically assign types to variables and primitives

Here the `main` function is special and has a fixed signature `(Int64)` so `0` is a `Int64`  
You can learn more about [Function signature](./function_signature.md)

## Float

```haskell
main: ->
  let x = 2.2
  x.print!
```

## Char

```haskell
main: -> '*'.print!
```

## String

Strings are immutable (like in most language)

```haskell
main: -> ("Hello" + " World!").print!
```

You can index them to get a `Char`

```haskell
second_char: -> "Hello"[1]
```

Strings are 0-indexed

In the future there will be a distinction between native `Str` and on-the-heap `String` like in Rust

## Array

There is very limited support for arrays, they are still in development.

```haskell
main: ->
  let a = [1, 2, 3]
  a[2] = 4
  a[2]
```

