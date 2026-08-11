# Rock

[![Book](https://github.com/Champii/Rock/actions/workflows/book.yml/badge.svg?branch=develop)](https://github.com/Champii/Rock/actions/workflows/book.yml)
[![Discord](https://img.shields.io/discord/990627124236939314.svg)](https://discord.gg/f6skPNB96J)

[Documentation](https://champii.github.io/Rock/)

A native, expression-oriented programming language built with Rust and LLVM.

Rock combines static typing and ownership with a compact functional syntax. It is inspired by [LiveScript](https://livescript.net/), [Haskell](https://www.haskell.org/), and [Rust](https://www.rust-lang.org/).

Rock is experimental. The language, compiler, and tooling can change or break at any time. Contributions and design discussions are welcome.

## Index

- [Features](#features)
- [Install](#install)
- [Quickstart](#quickstart)
- [Showcase](#showcase)
- [Tooling](#tooling)
- [Documentation](#documentation)

---

## Features

### Type system

- Strong static typing with local and generic type inference
- Algebraic data types, tuples, structs, enums, and exhaustive-style pattern matching
- Parametric polymorphism, trait bounds, associated types, and trait default methods
- Higher-kinded types and constructor bounds such as `F _: Functor`
- First-class functions, closures, callable structs, currying, and partial application
- User-defined infix operators with program-defined precedence and semantics

### Safety and systems programming

- Ownership, moves, shared references, mutable references, and borrow checking
- Explicit `unsafe` blocks for raw pointers, pointer arithmetic, and FFI boundaries
- Deterministic destruction through `Drop`
- Thread-safety contracts through `Send` and `Sync`
- Native code generation through LLVM 18
- C interoperability for building low-level libraries and platform bindings

### Functional programming

- `Option`, `Result`, and generic `?` short-circuiting through `Try`
- Standard `Functor`, `Bifunctor`, `Applicative`, `Monad`, `Foldable`, and `Traversable` traits
- Functional operators including `|>`, `<$>`, `<&>`, `<!>`, `<*>`, `>>=`, and `<|>`
- Effectful traversal and sequencing over `Option`, `Result`, and `Vec`
- Function-call holes and concise lambdas for point-free-style pipelines
- Complete-expression arguments and low-precedence spaced-dot chains such as `double x + 1 .println!`
- Expression-oriented `if`, `match`, loops, and blocks

### Programs and tooling

- Modules, explicit imports and exports, path dependencies, and reusable crate artifacts
- Declarative macros and macro expansion inspection
- A standard library with strings, vectors, hash maps, files, TCP networking, threads, atomics, `Arc`, and `Mutex`
- Project-aware build, run, format, expand, and artifact commands through `rock`

---

## Install

Rock currently targets `x86_64-unknown-linux-gnu`. Building from source requires Git, Rust with Cargo, LLVM 18 with shared libraries, and a C linker available as `cc`.

```console
$ git clone https://github.com/Champii/Rock.git
$ cd Rock
$ cargo build --release
$ target/release/rockup dev stdlib package \
    --path stdlib \
    --sysroot target/release
$ target/release/rockup toolchain install dev --path target/release
$ target/release/rockup default dev
```

Restart the shell when prompted so the installed `rock` command is available. Rock does not yet ship through a package registry or stable binary installer.

See the [installation guide](https://champii.github.io/Rock/getting-started/installation.html) for setup details and troubleshooting.

---

## Quickstart

Create a project with a manifest:

```console
$ mkdir hello-rock
$ cd hello-rock
```

`rock.toml`:

```toml
[crate]
name = "hello"
version = "0.1.0"

[lib]
path = "main.rk"
```

`main.rk`:

```haskell
main = ->
    "Hello, Rock!".println!
    0
```

Build and run it:

```console
$ rock run
Hello, Rock!
```

---

## Showcase

The working [TCP chat example](test_projects/new_new/main.rk) combines ownership, threads, synchronization, generic callbacks, and functional error handling without hiding the control flow.

### Shared state as collection transforms

Function-call holes keep the state operations focused on intent: remove a connection by its field, clone only the writers, then broadcast with a unit-returning callback.

```haskell
struct ClientWriter
    < stream: Arc TcpStream
    < lock: Arc (Mutex I64)

struct ServerConnection
    < id: I64
    < writer: ClientWriter

struct ServerState
    < clients: Vec ServerConnection
    < next_id: I64

struct SharedServerState
    < inner: Arc (Mutex ServerState)

impl Clone for ClientWriter
    @clone = -> ClientWriter
        stream: self.stream.clone!
        lock: self.lock.clone!

impl ClientWriter
    @send_all: &[U8] -> I64 -> Result I64, IoError
    @send_all = bytes, len ->
        guard = self.lock.lock!
        self.stream.send_all_prefix bytes, len

impl ServerState
    ^@remove: I64 -> Unit
    ^@remove = id -> self.clients.retain (.id != id)

    @snapshot: Vec ClientWriter
    @snapshot = -> self.clients.map_ref (.writer.clone!)

broadcast: &mut SharedServerState -> &[U8; 1024] -> I64 -> Result I64, IoError
broadcast = state, bytes, len ->
    guard = state.inner.lock!
    guard.snapshot!.for_each_owned target !-> target.send_all bytes, len
    Result::Ok len
```

### TCP and threads as a result pipeline

The client maps thread errors, binds the spawned reader into a curried continuation, propagates I/O failures with `?`, and maps the final join result back to the byte count.

```haskell
struct ClientReader
    < stream: Arc TcpStream

receive_with: Arc TcpStream -> T -> (&mut T -> &[U8; 1024] -> I64 -> Result I64, IoError) -> I64
receive_with = stream, mut target, write ->
    mut buffer: [U8; 1024] = [0; 1024]
    mut total: I64 = 0

    while true
        match stream.recv &mut buffer
            Result::Ok count =>
                if count <= 0
                    return total
                else
                    match write &mut target, &buffer, count
                        Result::Ok written => total = total + written
                        Result::Err _ => return total
            Result::Err _ => return total
    total

impl ClientReader
    ~@run: I64
    ~@run = -> receive_with self.stream.clone!, stdout!, Stdout::write_all_prefix

connect: () -> Result I64, IoError
connect = ->
    addr = Ipv4Addr::localhost!
    socket = SocketAddrV4::new addr, 9999
    TcpStream::connect socket >>= run_connection

run_connection: TcpStream -> Result I64, IoError
run_connection = stream ->
    shared = Arc::new stream
    reader = ClientReader
        stream: shared.clone!

    spawn (-> reader.run!)
        <!> thread_error_to_io
        >>= finish_connection shared.clone!

finish_connection: Arc TcpStream -> JoinHandle I64 -> Result I64, IoError
finish_connection = stream, handle ~>
    total = pump_stdin stream.clone!?
    stopped = stream.shutdown_write!?

    handle.join!
        <!> thread_error_to_io
        <&> _ -> total

pump_stdin: Arc TcpStream -> Result I64, IoError
pump_stdin = stream -> stdin! |>> &stream

thread_error_to_io: ThreadError -> IoError
thread_error_to_io = _ -> IoError::Os 1
```

More complete programs live in [`examples/`](examples/), [`test_projects/`](test_projects/), and the [language guide](https://champii.github.io/Rock/).

---

## Tooling

All application workflows use the project-aware `rock` command from a directory containing `rock.toml`:

```console
$ rock format
$ rock build
$ rock run -- first second
$ rock expand
$ rock artifact
```

- `format` rewrites the configured source file.
- `build` compiles the package and its path dependencies.
- `run` builds and executes the package; arguments after `--` go to the program.
- `expand` prints macro-expanded source.
- `artifact` materializes a reusable crate artifact.

There is currently no `rock test` command. Application tests are ordinary Rock programs or external harnesses; compiler contributors use the Rust integration suite.

---

## Documentation

The full book is published as [The Rock Programming Language](https://champii.github.io/Rock/). Its source is under [`docs/`](docs/) and can be built locally with:

```console
$ mdbook build docs
```

Compiler contributors can run the Rust test suite with:

```console
$ cargo test -p rock-lib
```
