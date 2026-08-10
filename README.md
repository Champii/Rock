# Rock

The beginner guide is available as [The Rock Programming Language](https://champii.github.io/new_lang/). The source lives under [`docs/`](docs/), and local builds use `mdbook build docs`.

## Desired features

## Chain calls without defining variable

```haskell
write_file = ->
    File::open "test.txt"?
       .write "Hello, World!"?
       ..close!
```

## If and loops as expressions

```haskell
do_something = x ->
    value = if x > 42 then 42 else x

    list =
        while value > 0
            value++

    new_list =
        for item in list
            item + 2
```

## Functions as first class citizen

```haskell
call = f, x -> f x

return_fn = -> x -> x + 2

main = -> call return_fn!, 5
```

## Function signature

```haskell
add : T -> T -> T
add = x, y -> x + y
```

## Function currying

```haskell
add = x, y -> x + y

add4 = add 4

main = -> add4 2
```

## Function Shorthand

```haskell
plus2 = (+2)

main = -> plus2 2
```

## Custom operators

```haskell
// The pipe operator
infix 1 |>
|> = x, f -> f x

// The not operator
unary !
! = x ->
    match x
        true => false
        false => true

main = ->
    [1, 2, 3]
        |> map (+2)

    inverse = !true
```

## FP-style stdlib composition

```haskell
inc = x -> x + 1

keep_even_opt = x ->
    if x % 2 == 0
        Option::Some x
    else
        Option::None

main = ->
    value = Option::Some 4

    ((value <&> inc).unwrap_or 0).println!
    ((value >>= keep_even_opt).unwrap_or 0).println!
    (((Option::None <|> Option::Some 9)).unwrap_or 0).println!
    (((41 |> inc))).println!
    0
```

Method-only equivalent:

```haskell
main = ->
    value = Option::Some 4

    ((value.map inc).unwrap_or 0).println!
    ((value.and_then keep_even_opt).unwrap_or 0).println!
    ((Option::None.or (Option::Some 9)).unwrap_or 0).println!
    0
```

## Trait

```haskell
trait ToString
    @to_string : String

impl ToString for Int16
    @to_string = -> @show!
```

## Dynamic trait

```haskell
some_func : <T: ToString> T -> String
some_fumc = x -> x.to_string!
```

## Structs

```haskell
struct Hello
    world : String
    some_default_field = true

impl Hello
    new = s -> Hello world: s
    @display = -> @world.print!

main = ->
    hello = Hello::new "World"
    hello.display!
```

## Generics

```haskell
struct Wrapper T
    inner: T

enum Choice T, U
    Left T
    Right U
```

## Unsafe pointer arithmetic

```haskell
main = ->
    unsafe
        p: *Int8 = 0

        // very unsafe
        *p
```

## Pattern matching

```haskell
main = ->
    a = (10, "hello")

    match a
        (0, "world")            => "something"
        foo @ (a, str) if a > 5 => str
        _                       => "otherwise"
```

## Destructuring

```haskell
fn_return_tuple = -> (10, "a string")

main = ->
    (num, str) = fn_return_tuple!
```

## Enums

```haskell
enum Error
    SomeError
    SomeErrorWithContext String
    SomeErrorWithMoreContext String, Int

impl Show for Error
    @show = ->
        match @
            Self::SomeError => "SomeError"
            Self::SomeErrorWithContext s => "SomeError" + s
            Self::SomeErrorWithMoreContext s, i => "SomeError" + s + i.show!

main = -> Error::SomeErrorWithContext "Hello" .print!
```

## Macros

```haskell
macro generate
    $name:ident, $type:ty =>
        struct Generated
            $name: $type

%generate hello, String
```

## Modules and dependancies

`Rock.toml`  
```toml
name = "my_awesome_package"

#[dependencies]
my_dep = [ path = "../my_dep" ]
```

`lib.rk`  
```haskell
mod my_mod

> my_dep::SomeType
> my_mod::SomeOtherType

struct MyStruct
    my_field: SomeType

< MyStruct
```

# TODO

## CLI

For a crate with a local `rock.toml`, `rock build` now behaves like a lightweight cargo-style wrapper:

```bash
cargo run -p rock -- build
```

To build and execute the current crate, use:

```bash
cargo run -p rock -- run -- <program args>
```

It will:

- read path dependencies from `rock.toml`
- build or reuse dependency artifacts under each dependency crate's `build/artifacts/`
- build or reuse dependency object files under each dependency crate's `build/objects/`
- compile the current crate using artifact-backed dependencies

You can also materialize the current crate artifact directly:

```bash
cargo run -p rock -- artifact
```

# TODO

  - Error recovery for parser statements (find the next Indent(x))

  - Macros
    - Macro nested var repetition $($($arg:ident)*)*
    - Don't ignore \n and indent for macro args parsing
    - Better error management and diagnostic details

  - New parser
    - Trait bound for parse_type in function signature
    - Slices
    - Escaped chars and strings
    - UTF-8
    - Allow trailing delimiters everywhere
    - Allow for custom unary operators
    - Struct default field value ?
    - Better error management

  - Parser
    - High priority
      - Add syntax for generic types in ParseType
      - Allow for multiline double dot
      - Allow for self inject signatures for methods in impl
      - Add mutability operator to variables
      - `if let` like `if Err e = run!`
      - `Ok a = f! else return Err "error"`
      - Escaped char in strings and char
      - Auto export current item `< struct Foo`
      - Slice of array `arr[1..]` and range
      - Default arguments and named arguments
      - Oneliner for loops (other than postfix) like array comprehension `[a for a in arr]`
      - `do` keyword for nested blocks
      - Comments (end of line ('#') or inline ('/*' '*/')
      - Multiline equal
      - Multiline Patterns
      - Allow right indent `if` and `loop` and `match` when at-the-next-line definition
      - Suffix ++ and -- for variables ?

    - Low priority list:
      - Allow typeless struct field when default value and let the inference take over
      - Fix that annoying problem with unaryexp that can be confused with function shorthand (space problem between operator and expr)
      - Async?
      - Allow parenthesis for function calls ?
      - String interpolation ? 
      - Replace the Type1OrType2 with an implem of Either<Type1, Type2>
      - Remove the `,` separator for arguments and follow haskell fn call format ?
      - Proc macros ? Derive ? 
      - Allow asm code ?
      - Allow right shifted indentation when assign ? (maybe a bad idea)
      - Multiline operators that can indent but not dedent

    - To investigate again
      - Fix nested multiline dot (indent problem)
      - Multiline arguments and spaced dot are in conflict. A spaced dot should close the arguments on the line its defined on

  - Desugar
    - Operator precedence
    - Operator into function calls
    - Loops into `loop`
    - Match into `if`
    - Dot notation into function calls
    - Closures into functiondecl

  - Modules and dependancies
    - Basic external module management

  - Code formating
    - If with no then
    - trait and impl ordering
    - Comments

  - Rust FFI ?
