
## TODO (by order):

- v0.1.1
    - operator precedence
    - sub, mul div and mod for ints
- v0.1.2
    - while/for_in
- v0.1.3
    - immutable by default
    - mut keyword
- v0.1.4
    - type aliasing
    - floats
- v0.1.5
    - enums
    - escaped chars
- v1.0.0
    - desugar
    - operator overload
    - pattern matching
    - macro
    - borrow checker
    - traits and impl
    - stdlib

# Goal

```haskell
class Foo
    val :> Int

    @val ->

    bar -> 1

trait Num
    +: A -> A
    -: A -> A
    *: A -> A
    /: A -> A

impl Num for UInt_8: Int
    +: -> ~#compiler_add_uint_8 @, it

impl Num for Foo
    +: -> Foo @val + it.val

class Iterator
    collec: [A]
    item: A
    idx: 0

    @collec ->

    next: -> 
        @item = @collec[@idx]
        @idx++
        @item

trait IntoIterator
    iter: Iterator

impl IntoIterator for Foo
    iter: -> Iterator @

main ->
  a = Foo 1
  b = Foo 2
  a + b
```

# NEW GLOBAL LONG TERM TODO

## 1
  - Use the simpliest syntax possible
  - Custom operators
  - If/Else

## 2

### Todo Dependency graph
  - Modules
    - Rename/mangle fns before codegen
  - Polymophism
    - Mark generic functions
    - Allow them to pass the typechecker if never called
  - Currying
    - Closure
        - LowLevel Structs
  - Pattern matching
  - Custom operators
    - Operator as Identifier (special syntax)
    - Infix notation 
    - Custom precedence definition

### TO FIXE

  - Function that takes a function as parameter must be declared before that function
  - Functions with same arguments name mess with inference 
  - Make unused funcs to pass the infer
