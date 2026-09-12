# Installation

Rock's release toolchain supports only Linux x86_64 with GNU glibc (`x86_64-unknown-linux-gnu`). The binary baseline is Ubuntu 24.04, requiring glibc 2.39 or newer, LLVM 18 shared libraries, and a C linker available as `cc`.

> **Availability:** historical GitHub releases exist, but no release with the new rockup bootstrap and toolchain assets is published yet. Until then, use the source installation below. Version `v0.1.0` in this chapter is an example, not an available release promise.

## Prerequisites

For binary installation on Ubuntu 24.04, install the runtime requirements yourself:

```console
$ sudo apt install libllvm18 build-essential curl ca-certificates
```

GNU tar, gzip, and `sha256sum` must also be installed (normally already present on Ubuntu). Rockup uses curl for HTTPS downloads and those tools to verify and unpack archives. The installer never invokes `sudo` or installs operating-system packages. Rust and LLVM development headers are not required to use a binary release.

## Install a release

Once the first release with bootstrap assets is published, download `install.sh` from the official `Champii/Rock` release into a private temporary directory:

```sh
install_dir=$(mktemp -d)
if curl -fsSL https://github.com/Champii/Rock/releases/latest/download/install.sh -o "$install_dir/install.sh"; then
    # Optionally inspect "$install_dir/install.sh" before executing it.
    sh "$install_dir/install.sh"
fi
rm -r "$install_dir"
```

Downloading a file first lets you inspect it rather than piping a network response into a shell. Only run the script if curl succeeds. The bootstrap downloads the standalone `rockup-x86_64-unknown-linux-gnu` and its exact `.sha256` sidecar over HTTPS, checks that the sidecar names that asset and that its checksum matches, then executes `rockup install stable`. Checksums detect corruption; they are not independent signatures, so you still trust the official release publisher and HTTPS.

`stable` means GitHub's latest published non-prerelease release, not a promise that this experimental language has a stable API. To select a specific published release, use its `vVERSION` tag:

```console
$ sh scripts/install.sh v0.1.0
```

That command uses the bootstrap from a checkout. With the downloaded script, pass the same version to `sh "$install_dir/install.sh"` before removing the temporary directory. Stable bootstrap assets come from `releases/latest/download`; pinned assets come from `releases/download/vVERSION`.

### Home and shell setup

Rockup persists itself as `~/.rockup/bin/rockup`, installs toolchains under `~/.rockup/toolchains`, and creates `rock`, `rockc`, and `rock-lsp` shims in `~/.rockup/bin`. Set `ROCKUP_HOME` to a nonempty absolute path before installation to use another location; keep using that value in later shells.

The bootstrap refuses to overwrite an existing `ROCKUP_HOME/bin/rockup`, including a dangling symlink. Use that installed manager's `install`, `update`, or `self update` command instead of rerunning the bootstrap. Installing a release does not delete an installed local `dev` toolchain or replace an existing default; the first toolchain in an empty home becomes the default.

Rockup writes an `env` file and a managed activation block to `.bashrc` for Bash, `.zshrc` for Zsh, or `.profile` for other shells, based on `SHELL`. It also updates existing Bash and Zsh configuration files. Restart your shell, or activate a POSIX-compatible shell immediately:

```sh
. "${ROCKUP_HOME:-$HOME/.rockup}/env"
rock --version
rock-lsp --help
```

The generated activation script uses POSIX shell syntax; it is not native Fish or PowerShell setup. `rock-lsp` without arguments starts an LSP server over standard input/output, not an interactive prompt.

## Manage releases and pins

With rockup installed, the shortest commands install or update the latest stable release:

```console
$ rockup install
$ rockup update
```

Both commands default to `stable`. Use `update` when that toolchain is already installed.

Install a version without changing an existing default:

```console
$ rockup install v0.1.0
```

The bare spelling `rockup install 0.1.0` selects the same release and stores it as `v0.1.0`; do not run both installation commands for the same version. To choose it globally or run a single command explicitly:

```console
$ rockup default v0.1.0
$ rockup run v0.1.0 rock --version
```

`rockup default NAME` selects an installed toolchain, including local `dev`, and automatically installs a missing stable or versioned release. Arbitrary local names must first be installed with `--path`.

For a project pin, create `rock-toolchain.toml` in the project directory with this schema:

```toml
[toolchain]
channel = "v0.1.0"
```

Install that version first. The shims select a toolchain using `ROCKUP_TOOLCHAIN` first, then the nearest `rock-toolchain.toml` in the current directory or its ancestors, then the global default. A project pin selects an installed toolchain; it does not automatically download one. Use the canonical `v` spelling for version pins, or an installed local name such as `dev`.

Update the moving stable toolchain and the manager separately:

```console
$ rockup update
$ rockup update stable
$ rockup self update
$ rockup list
```

The first two commands are equivalent. Updating `stable` leaves separately installed pinned versions and local development toolchains in place. `self update` replaces the running rockup executable with the latest stable manager. To remove a version, select a different default first and remove any project or environment selection that still refers to it:

```console
$ rockup default stable
$ rockup remove v0.1.0
```

Rockup is not a full Rustup replacement. Release downloads do not support Windows, macOS, musl, other CPU architectures, a nightly channel, or automatic cross-target installation. The local `target add --path` component workflow does not imply downloadable cross-target releases.

## Build from source

For compiler development or before binary assets are published, install these additional tools through your operating system:

- Git
- A Rust toolchain with Cargo
- LLVM 18 development tools (including `llvm-config`), headers, and shared libraries
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
$ target/release/rockup install dev --path target/release
$ target/release/rockup default dev
```

The installer creates command shims and prints the shell setup it added. Restart the shell when prompted, then verify the user-facing command:

```console
$ rock --version
$ rock-lsp --help
$ rockup list
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

## Common setup failures

- HTTP 404 for bootstrap assets means the selected release does not provide the new assets; use the source workflow until a compatible release is published.
- A checksum mismatch or malformed sidecar stops installation before the downloaded manager executes. Do not bypass verification; retry or report the release asset problem.
- `GLIBC_2.39` errors mean the host is older than the release baseline. Use Ubuntu 24.04 or a compatible newer GNU system, or build from source on your host.
- `llvm-config` or shared-library errors usually mean LLVM 18 is missing or its library path is not configured.
- A missing standard-library component means the packaging step did not complete or the installed toolchain does not match the checkout.
- If `rock` is not found after installation, restart the shell or apply the activation command printed by `rockup`.
- If `rock-lsp` is missing from a toolchain built from an older checkout, rebuild and package the current workspace before installing it; a current toolchain installation requires the server binary too.
- If an editor cannot find `rock-lsp` but the shell can, start the editor from that shell or configure an absolute server command as shown in [Editors and Diagnostics](editor-and-diagnostics.md).
- If a generated executable is not found, inspect the project's `build/` directory.
