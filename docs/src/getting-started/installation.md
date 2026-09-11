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
$ rock-lsp --help
$ target/release/rockup toolchain list
```

The selected toolchain contains the compiler implementation and matching standard library, but application authors interact with it through `rock`. The project command discovers manifests, builds path dependencies, selects the standard library, and manages output paths.

It also contains `rock-lsp`, the language server used by editors. `rockup` installs a shim for this command alongside `rock`, so an editor started from the activated shell can find the selected server. `rock-lsp --help` checks that the command can start; running it without arguments starts an LSP server over standard input and output, not an interactive compiler prompt.

## Optional editor setup

The repository includes a Neovim plugin that uses Neovim's built-in LSP client; it does not require `nvim-lspconfig`. The plugin requires Neovim 0.11 or newer. With Neovim 0.12, add this to `init.lua` to install it with the built-in package manager:

```lua
vim.pack.add({ "https://github.com/Champii/Rock" })
require("rock").setup()
```

Installing the plugin does not build or install the Rock toolchain. Complete the toolchain steps above first, then open a project's `.rk` file and run `:checkhealth rock`.

For local-checkout setup, editor commands, and a walkthrough of compiler errors, continue with [Editors and Diagnostics](editor-and-diagnostics.md). Other LSP clients can start `rock-lsp` for `.rk` files using the `rock` language identifier and standard input/output transport.

## Compiler contributor checks

Run the compiler's Rust tests when changing the language implementation:

```console
$ cargo test -p rock-lib
```

> **Current status:** Rock does not yet ship through a package registry or a stable binary installer. LLVM and target support remain platform-specific.

## Common setup failures

- `llvm-config` or shared-library errors usually mean LLVM 18 is missing or its library path is not configured.
- A missing standard-library component means the packaging step did not complete or the installed toolchain does not match the checkout.
- If `rock` is not found after installation, restart the shell or apply the activation command printed by `rockup`.
- If `rock-lsp` is missing from a toolchain built from an older checkout, rebuild and package the current workspace before installing it; a current toolchain installation requires the server binary too.
- If an editor cannot find `rock-lsp` but the shell can, start the editor from that shell or configure an absolute server command as shown in [Editors and Diagnostics](editor-and-diagnostics.md).
- If a generated executable is not found, inspect the project's `build/` directory.
