# Tooling Reference

Rock applications use one project command: `rock`. Run it from the directory containing `rock.toml` so it can resolve the entry source, path dependencies, standard library, and build outputs.

## Create a project

Create a project directory containing this `rock.toml`:

```toml
[crate]
name = "rock_book_project"
version = "0.1.0"

[lib]
path = "main.rk"
```

Create `main.rk` beside it:

```rock
main = ->
    "rock-book-project".println!
    0
```

The manifest's `[lib].path` selects the source entry even when that source contains an executable `main` function.

## Everyday commands

```console
$ rock format
$ rock build
$ rock run -- 7
$ rock expand
$ rock artifact
```

`format` rewrites the configured source. Inspect formatter changes before committing them, then compile again.

`build` resolves the manifest and path dependencies, then writes products below the project's `build/` directory. It prints the linked executable path on success.

`run` builds and executes the project. Values after `--` are application arguments rather than options to the build tool.

`expand` prints macro-expanded source. It is useful when a diagnostic points into generated declarations or when a repetition does not produce the expected items.

`artifact` materializes a reusable crate artifact. Normal path-dependent builds create and reuse dependency artifacts automatically, so most applications do not need to invoke it directly.

There is no `rock test` command. Use a complete Rock executable as an application smoke test, or use the Rust integration suite when changing the compiler itself.

## Standard library selection

The active toolchain supplies a matching precompiled standard library. `rock` loads it automatically for ordinary projects and makes the prelude available. A package can opt out explicitly:

```toml
[crate]
name = "bare"
version = "0.1.0"
no_std = true

[lib]
path = "lib.rk"
```

Without the standard library, familiar operators, collections, strings, and prelude traits are unavailable unless the package supplies alternatives through explicit dependencies.

## `rockup`

`rockup` installs and selects local Rock toolchains. A source checkout can be packaged and installed with:

```console
$ cargo build --release
$ target/release/rockup dev stdlib package \
    --path stdlib \
    --sysroot target/release
$ target/release/rockup toolchain install dev --path target/release
$ target/release/rockup default dev
$ target/release/rockup toolchain list
```

The first installed toolchain becomes the default automatically; `rockup default` switches it explicitly. Toolchain shims keep the user workflow on `rock` while selecting matching compiler and standard-library components behind the project command.

## Grammar and editor support

The separate grammar project is under `tree-sitter-rock/`:

```console
$ cd tree-sitter-rock
$ tree-sitter generate
$ tree-sitter build
$ tree-sitter test
```

It contains highlight and indentation queries for editor integrations. The formatter and compiler diagnostics remain the primary feedback loop; there is no mature language server or package-registry workflow yet.
