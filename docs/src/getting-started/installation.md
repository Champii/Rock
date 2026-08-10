# Installation

Rock currently builds from source and targets `x86_64-unknown-linux-gnu`. Code generation requires LLVM 18 and a C linker.

## Prerequisites

Install these tools through your operating system:

- Git
- A Rust toolchain with Cargo
- LLVM 18, including its shared libraries
- A C linker available as `cc`

Clone the repository and build the release toolchain:

```console
$ git clone https://github.com/Champii/Rock.git
$ cd Rock
$ cargo build --release
```

## Preparing a development toolchain

Package the standard library beside the release binaries, then install that directory as a local toolchain:

```console
$ target/release/rockup dev stdlib package \
    --path stdlib \
    --sysroot target/release
$ target/release/rockup toolchain install dev --path target/release
$ target/release/rockup default dev
```

The installer creates command shims and prints the shell setup it added. Restart the shell when prompted, then verify the user-facing command:

```console
$ rock --version
$ target/release/rockup toolchain list
```

The selected toolchain contains the compiler implementation and matching standard library, but application authors interact with it through `rock`. The project command discovers manifests, builds path dependencies, selects the standard library, and manages output paths.

Run the compiler's Rust tests when changing the language implementation:

```console
$ cargo test -p rock-lib
```

> **Current status:** Rock does not yet ship through a package registry or a stable binary installer. LLVM and target support remain platform-specific.

## Common setup failures

- `llvm-config` or shared-library errors usually mean LLVM 18 is missing or its library path is not configured.
- A missing standard-library component means the packaging step did not complete or the installed toolchain does not match the checkout.
- If `rock` is not found after installation, restart the shell or apply the activation command printed by `rockup`.
- If a generated executable is not found, inspect the project's `build/` directory.
