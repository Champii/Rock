# Current Limitations

Rock is an active prototype. The constraints below describe behavior that a
programmer can encounter today, not promises about a final language design.

## Platform and distribution

The supported code-generation target is currently
`x86_64-unknown-linux-gnu`, and code generation requires LLVM 18 plus a C
linker. The FFI-backed standard library is POSIX- and Linux-oriented. There is
no stable binary installer or public package registry workflow.

Builds therefore need an installed toolchain prepared from a checkout. From this repository, use:

```console
$ cargo build --release
$ target/release/rockup dev stdlib package \
    --path stdlib \
    --sysroot target/release
$ target/release/rockup toolchain install dev --path target/release
```

Use path dependencies rather than registry-only dependencies. A complete
two-package layout is shown as `text` because each source file is a separate
compilation unit:

```text
/tmp/rock-path-project/geometry/rock.toml
[crate]
name = "geometry"
version = "0.1.0"

[lib]
path = "lib.rk"

/tmp/rock-path-project/geometry/lib.rk
< square: I64 -> I64
< square = value -> value * value

/tmp/rock-path-project/app/rock.toml
[crate]
name = "app"
version = "0.1.0"

[lib]
path = "main.rk"

[dependencies]
geometry = { path = "../geometry" }

/tmp/rock-path-project/app/main.rk
> geometry::square

main = ->
    result: I64 = square 6
    result.println!
    0
```

The workaround is to keep the dependency local and compile from
`/tmp/rock-path-project/app` with the project command. Version-only registry
entries are parsed but are not resolved.

## Tooling

There is no `rock test` subcommand. Use a complete executable as an
application smoke test, and use the Rust integration suite for compiler work:

```console
$ cargo test -p rock-lib --test integration test_hello_world -- --exact
$ cargo test -p rock-lib --test integration test_unsafe_operator_function_requires_unsafe -- --exact
$ cargo test -p rock-lib
```

The formatter can still change some trait implementation headers, nested
generic applications, and `&mut` expressions unexpectedly. This complete
program is a small formatter regression reproducer:

```rock
increment: &mut I64 -> Unit
increment = value ->
    *value = *value + 1
    return

main = ->
    mut number: I64 = 4
    increment &mut number
    number.println!
    0
```

Format it, inspect the resulting diff, and compile the formatted file. The
workaround is to keep the pre-format source under version control and restore
the affected declaration manually when the formatter changes its meaning.
The tree-sitter grammar, formatter, and older root documentation can also lag
the active parser; compiler diagnostics are authoritative for a build.

Inline modules are another parser-only surface. The following source shape is
not a supported compiled project:

```text
main.rk
mod inline

main = ->
    0
```

Use a file-backed module instead, with `inline.rk` beside `main.rk`, and import
its exported names explicitly. Macro diagnostics and indentation recovery are
also incomplete, and editor and language-server support remains early.

## Language surface

### Guarded non-copy payloads

The compiler currently rejects a guard that consumes a non-copy enum payload.
This is a complete diagnostic reproducer:

```rock
enum Message
    Text String
    Empty

has_text: Message -> Bool
has_text = message ->
    match message
        Message::Text text if text.len! > 0 => true
        Message::Text _ => false
        Message::Empty => false

main = ->
    message: Message = Message::Text String::from_str "hello"
    result: Bool = has_text message
    result.println!
    0
```

The workaround is to match without the guard and perform the condition inside
the arm, where the payload's ownership is unambiguous:

```rock
enum Message
    Text String
    Empty

has_text: Message -> Bool
has_text = message ->
    match message
        Message::Text text => text.len! > 0
        Message::Empty => false

main = ->
    message: Message = Message::Text String::from_str "hello"
    result: Bool = has_text message
    result.println!
    0
```

Incomplete enum matches are not diagnosed in every path, so list every
variant or end with `_` even when the checker accepts less source.

### Tail effects and unit arrows

A side-effecting call in a helper's tail conditional can be dropped when the
caller ignores the helper's result. This complete program should not be used
to rely on the print inside `choose`:

```rock
effect: I64 -> I64
effect = value ->
    value.println!
    value

choose: Bool -> I64
choose = flag ->
    if flag
        effect 7
    else
        0

main = ->
    choose true
    0
```

Return data from the helper and consume it in the caller instead:

```rock
effect: I64 -> I64
effect = value ->
    value.println!
    value

choose: Bool -> I64
choose = flag ->
    if flag
        effect 7
    else
        0

main = ->
    chosen: I64 = choose true
    chosen.println!
    0
```

For a `!->` function, the current compiler skips a trailing value expression
instead of evaluating and discarding it. Put every required effect before an
explicit `return`:

```rock
discard: I64 -> Unit
discard = value !->
    value.println!
    99.println!

main = ->
    discard 7
    0
```

The workaround is:

```rock
discard: I64 -> Unit
discard = value !->
    value.println!
    99.println!
    return

main = ->
    discard 7
    0
```

### Operators, generic carriers, and text

Custom infix operators work when the program declares them. Custom unary
authoring is less complete; use a named function for a new prefix operation:

```rock
negate: I64 -> I64
negate = value -> 0 - value

main = ->
    result: I64 = negate 3
    result.println!
    0
```

`?` works with the standard `Option` and `Result` carriers and with a custom
carrier that implements matching `Try` and `FromResidual` contracts. The
complete custom `MyFlow` implementation in [Option, Result, and
`?`](../functional/error-handling.md#defining-a-custom--carrier) shows both
success and early propagation. What remains limited is automatic conversion
between unrelated custom residual families; use an explicit `match`, implement
the exact `FromResidual` conversion, or convert into `Result T, E` at the
boundary.

Strings, characters, and Unicode text handling are byte-oriented and escaped
literal behavior is not mature. This complete program measures encoded bytes,
not user-perceived characters:

```rock
> stdlib::eq::Eq
> stdlib::hash::Hash

main = ->
    text: String = String::from_str "cafe"
    length: I64 = text.len!
    length.println!
    0
```

Use ASCII protocol data or process a deliberately specified byte encoding until
Unicode-aware APIs are available. There is no async/await syntax or runtime;
use an ordinary synchronous function or the prototype thread API instead.

There are no top-level global or static values and no stable C-layout
attributes. Keep state in an owned value passed to `main`'s helpers, and use
raw buffers with an explicit byte layout at a C boundary rather than assuming
that a Rock struct has a C representation.

## Standard-library surface

### Strings and `Vec`

`String` length and indexing are byte-oriented. `Vec` has no general iterator
API and no `pop` method. This complete program uses the available length and
index operations to inspect the final element:

```rock
main = ->
    mut values: Vec I64 = Vec::new!
    values.push 10
    values.push 20
    length: I64 = values.len!
    last: I64 = values[length - 1]
    last.println!
    0
```

The workaround for removing the final element is to track the logical length
and use a supported operation such as `swap_remove` when order does not matter.
If order matters, copy the retained values into a new vector.

### `HashMap`

`HashMap` currently lacks removal, iteration, entry APIs, and configurable
hashing. Its intended lookup surface is `new`, `len`, `insert`, `get`, and
`contains_key`:

```rock
main = ->
    mut scores: HashMap String, I64 = HashMap::new!
    scores.insert String::from_str "Ada", 10
    key: String = String::from_str "Ada"
    present: Bool = scores.contains_key (& key)
    if present
        score: Option &I64 = scores.get (& key)
        match score
            Option::Some value => value.println!
            Option::None => 0.println!
    else
        "missing".println!
    0
```

An installed toolchain must contain the standard library built from the same
revision as the compiler. If these calls fail after switching revisions,
repackage and reinstall the development toolchain before diagnosing the source.

The workaround for removal or traversal is to rebuild a new map from known
keys, or maintain a separate owned list of keys while the map is in use.

Complete generic element dropping in every `Vec` and `HashMap` storage path is
still active work, and some zero-sized generic allocations are rejected. Keep
long-lived resource-heavy containers concrete and test cleanup behavior before
using them in a service.

### Environment, networking, and concurrency

Environment-variable lookup is not available. Pass configuration as explicit
arguments instead:

```rock
> stdlib::env::args

main = ->
    arguments: Vec String = args!
    arguments.len!.println!
    0
```

Networking is blocking IPv4 TCP only. It has no TLS, UDP, IPv6, or timeout
abstraction. This complete program demonstrates the supported bind boundary:

```rock
> stdlib::io::IoError
> stdlib::net::Ipv4Addr
> stdlib::net::SocketAddrV4
> stdlib::net::TcpListener

bind_local = ->
    address: SocketAddrV4 = SocketAddrV4::new Ipv4Addr::localhost!, 0
    TcpListener::bind address

main = ->
    result: Result TcpListener, IoError = bind_local!
    match result
        Result::Ok _ => 1.println!
        Result::Err error => error.println!
    0
```

Use a thread around a blocking operation when limited concurrency is enough,
and design shutdown explicitly. There are no channels, detached tasks,
condition variables, thread pools, or async executors. Dropping a `JoinHandle`
blocks until completion, so the workaround is to join handles deliberately at
known lifecycle points and keep closure captures small.

Thread and atomic implementations rely on platform ABI assumptions. Treat
long-running, memory-intensive, and concurrent applications as experiments
until cleanup and scheduling gaps close.

## Working with the prototype

Keep programs and dependencies pinned to one compiler revision, compile after
formatting, and prefer tested stdlib APIs over syntax found only in old plans.
When relying on a new edge case, add a parser test and an integration test.
Keep unsafe and FFI boundaries small, and report the smallest complete source
that demonstrates a limitation.
