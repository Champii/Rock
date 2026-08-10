# Installation

Rock currently builds from source and targets `x86_64-unknown-linux-gnu`. Code generation requires LLVM 18 and a C linker.

## Prerequisites

Install these tools through your operating system:

- Git
- A Rust toolchain with Cargo
- LLVM 18, including its shared libraries
- A C linker available as `cc`

Clone the repository and build the workspace:

```console
$ git clone https://github.com/Champii/new_lang.git
$ cd new_lang
$ cargo build --release
```

The build produces the user-facing `rock` and `rockc` executables under `target/release`. `rock` is the project-oriented command; `rockc` compiles one entry file with explicit inputs.

Run the compiler's Rust tests when changing the language implementation:

```console
$ cargo test -p rock-lib
```

## Preparing the standard library

Normal Rock programs use a precompiled standard-library artifact. In a repository checkout, build one directly:

```console
$ target/release/rockc \
    --entry-file stdlib/lib.rk \
    --crate-name stdlib \
    --no-prelude \
    --emit-artifact /tmp/rock-book-stdlib/stdlib.rkca \
    --no-link
```

Pass that artifact to direct compiler invocations:

```text
--extern-artifact stdlib=/tmp/rock-book-stdlib/stdlib.rkca
```

The explicit dependency matters. The compiler is not coupled to a built-in stdlib location, and it injects stdlib prelude names only after it has loaded an artifact named `stdlib`.

## Choosing a workflow

Use `rock` from a configured project directory for normal application work. Use `rockc` when you need a reproducible one-file command, a debug dump, or a scriptable build. Both commands ultimately compile the same language; the difference is how much project setup they perform for you.

> **Current status:** Rock does not yet ship through a package registry or a stable binary installer. LLVM and target support remain platform-specific.

## Common setup failures

- `llvm-config` or shared-library errors usually mean LLVM 18 is missing or a library path is not configured.
- A missing prelude name in a direct `rockc` run usually means the explicit stdlib artifact option was omitted.
- If a generated executable is not found, inspect the `--output-dir` directory rather than assuming the source directory contains it.
