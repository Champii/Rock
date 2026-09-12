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

Binary releases support only `x86_64-unknown-linux-gnu`, with an Ubuntu 24.04 baseline (glibc 2.39 or newer). LLVM 18 is statically linked, so end users do not need to install LLVM. These are not fully static executables: system libraries and a C linker are still required. The installer does not install system packages or run `sudo`.

On Ubuntu 24.04, install the runtime prerequisites yourself:

```console
$ sudo apt install build-essential curl ca-certificates
```

GNU tar, gzip, and `sha256sum` must also be available. Once the first release with the new rockup assets is published, download the bootstrap to a private temporary directory and run it:

```sh
install_dir=$(mktemp -d)
if curl -fsSL https://github.com/Champii/Rock/releases/latest/download/install.sh -o "$install_dir/install.sh"; then
    # Optionally inspect "$install_dir/install.sh" before running it.
    sh "$install_dir/install.sh"
fi
rm -r "$install_dir"
```

The bootstrap verifies the standalone rockup download against its exact SHA-256 sidecar, then asks it to install `stable`, the latest non-prerelease release. Pass `vVERSION` to pin a published release instead. Rockup installs under `~/.rockup` (or an absolute `ROCKUP_HOME`) and sets up command shims and shell activation. Restart your shell, then run `rock --version`.

**Availability:** historical GitHub releases exist, but the new bootstrap/toolchain assets are not yet published; use the source installation below until they are. The example version `v0.1.0` is illustrative, not a claim of availability.

### Build From Source

Building from source requires Git, Rust with Cargo, LLVM 18 development files and static archives, and a C linker available as `cc`. On Ubuntu 24.04, install the build dependencies and select LLVM 18 explicitly; there is no dynamic-linking fallback:

```console
$ sudo apt install llvm-18-dev libpolly-18-dev libzstd-dev libxml2-dev zlib1g-dev libffi-dev libedit-dev libncurses-dev build-essential
$ export LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
```

```console
$ git clone https://github.com/Champii/Rock.git
$ cd Rock
$ cargo build --release
$ target/release/rockup dev stdlib package \
    --path stdlib \
    --sysroot target/release
$ target/release/rockup install dev --path target/release
$ target/release/rockup default dev
```

Restart the shell when prompted so the installed `rock` command is available. Local `dev` toolchains remain supported alongside release toolchains.

### Manage Toolchains

```console
$ rockup install
$ rockup update
$ rockup install v0.1.0
$ rockup default v0.1.0
$ rockup run v0.1.0 rock --version
$ rockup self update
$ rockup list
$ rockup remove v0.1.0
```

`install` and `update` default to the latest stable release; pass a version to select a specific release. `install` also accepts a bare version such as `0.1.0`; `default NAME` installs a missing stable or versioned release before selecting it. `self update` updates rockup itself. Project pins and shell setup are explained in the installation guide. This is a small toolchain manager, not full Rustup parity: no Windows, macOS, nightly channel, or automatic cross-target downloads.

See the [installation guide](https://champii.github.io/Rock/getting-started/installation.html) for setup details and troubleshooting, and the [release maintainer guide](docs/releases.md) for packaging and draft publication.

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
    @send_all: &[U8] -> Result I64, IoError
    @send_all = bytes ->
        guard = self.lock.lock!
        self.stream.write_all bytes

impl ServerState
    ^@remove: I64 -> ()
    ^@remove = id -> self.clients.retain (.id != id)

    @snapshot: Vec ClientWriter
    @snapshot = -> self.clients.map_ref (.writer.clone!)

broadcast: &mut SharedServerState -> &[U8; 1024] -> I64 -> Result I64, IoError
broadcast = state, bytes, len ->
    chunk = &bytes[..len]
    guard = state.inner.lock!
    guard.snapshot!.for_each_owned target !-> target.send_all chunk
    Result::Ok len
```

### TCP and threads as a result pipeline

The client maps thread errors, binds the spawned reader into a curried continuation, propagates I/O failures with `?`, and maps the final join result back to the byte count.

```haskell
struct ClientReader
    < stream: Arc TcpStream

receive_with: Arc TcpStream -> T -> (&mut T -> &[U8] -> Result I64, IoError) -> I64
receive_with = stream, mut target, write ->
    mut buffer: [U8; 1024] = [0; 1024]
    mut total: I64 = 0

    while true
        match stream.recv &mut buffer
            Result::Ok count =>
                if count <= 0
                    return total
                else
                    match write &mut target, &buffer[..count]
                        Result::Ok written => total = total + written
                        Result::Err _ => return total
            Result::Err _ => return total
    total

impl ClientReader
    ~@run: I64
    ~@run = -> receive_with self.stream.clone!, stdout!, (output, bytes -> output.write_all bytes)

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
    total = pump_stdin &stream?
    stream.shutdown_write!?

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

The `rock-lsp` binary provides live diagnostics, inferred variable types on hover, function signatures on hover, and call signature help:

```console
$ cargo build -p rock-lsp --release
$ target/release/rock-lsp
```

Configure an editor LSP client to start that command for `*.rk` files. The server finds the nearest `rock.toml`, reuses `rock build` dependency and toolchain resolution, and overlays unsaved editor buffers on the project entry graph. `--extern-artifact` and `--no-prelude` remain available as explicit overrides for standalone or experimental workflows.

### Neovim 0.12

This repository is also a dependency-free Neovim plugin built on `vim.lsp.config` and `vim.lsp.enable`. `rock-lsp` is installed and selected by `rockup` with the rest of the Rock toolchain.

Neovim 0.12's built-in package manager can install and configure the plugin:

```lua
vim.pack.add({ "https://github.com/Champii/Rock" })
require("rock").setup()
```

For a local checkout, add the repository to `runtimepath` instead. Use `:checkhealth rock`, `:RockLspInfo`, and `:RockLspRestart` to inspect the integration. Tree-sitter highlighting remains provided by [`tree-sitter-rock`](tree-sitter-rock/).

There is currently no `rock test` command. Application tests are ordinary Rock programs or external harnesses; compiler contributors use the Rust integration suite.

---

## Documentation

The full book is published as [The Rock Programming Language](https://champii.github.io/Rock/). Its source is under [`docs/`](docs/). First follow the [book build setup](docs/checks/README.md) to install the syntax-highlighting tools and generate the Rock parser, then build locally with:

```console
$ mdbook build docs
```

Compiler contributors can run the Rust test suite with:

```console
$ cargo test -p rock-lib
```
