# Tooling Reference

This chapter uses one concrete project throughout: `/tmp/rock-book-project`.
The commands assume the checkout is `/root/new_lang2` and that the standard
library artifact is `/tmp/rock-book-stdlib/stdlib.rkca`.

## Create a project

Create the project directory and write this manifest to
`/tmp/rock-book-project/rock.toml`:

```toml
[crate]
name = "rock_book_project"
version = "0.1.0"

[lib]
path = "main.rk"
```

Write this complete program to `/tmp/rock-book-project/main.rk`:

```rock
main = ->
    "rock-book-project".println!
    0
```

The source contains its own `main` function and uses only prelude facilities,
so it needs no non-prelude import.

## The `rock` command

Run project commands from the project directory. Each command below names the
same concrete paths and arguments:

```console
$ cd /tmp/rock-book-project
$ /root/new_lang2/target/release/rock format
$ /root/new_lang2/target/release/rock build
$ /root/new_lang2/target/release/rock run -- 7
$ /root/new_lang2/target/release/rock expand
$ /root/new_lang2/target/release/rock artifact
```

`format` rewrites the configured source. Inspect the diff before committing
formatter output, then compile again. `build` resolves the manifest and path
modules and writes products below `/tmp/rock-book-project/build`. `run` builds
and executes the result; `7` is an application argument after the `--`
separator. `expand` prints macro-expanded source, and `artifact` materializes
a reusable crate artifact.

There is no `rock test` command. Use a complete Rock executable as a smoke
test, or use the Rust integration suite when changing the compiler itself.

## A direct `rockc` build

`rockc` is useful when a script needs explicit source and artifact paths. The
following command compiles the same project without consulting a manifest:

```console
$ /root/new_lang2/target/release/rockc \
    --entry-file /tmp/rock-book-project/main.rk \
    --output-dir /tmp/rock-book-project/build/direct \
    --crate-name rock_book_project \
    --extern-artifact stdlib=/tmp/rock-book-stdlib/stdlib.rkca
$ /tmp/rock-book-project/build/direct/rock_book_project
rock-book-project
```

Useful stable output options for the same invocation are:

```console
$ /root/new_lang2/target/release/rockc \
    --entry-file /tmp/rock-book-project/main.rk \
    --output-dir /tmp/rock-book-project/build/object \
    --crate-name rock_book_project \
    --extern-artifact stdlib=/tmp/rock-book-stdlib/stdlib.rkca \
    --emit-object \
    --no-link

$ /root/new_lang2/target/release/rockc \
    --entry-file /tmp/rock-book-project/main.rk \
    --output-dir /tmp/rock-book-project/build/llvm \
    --crate-name rock_book_project \
    --extern-artifact stdlib=/tmp/rock-book-stdlib/stdlib.rkca \
    --emit-llvm \
    --no-link
```

`--entry-file` selects the source entry, `--output-dir` selects generated
products, `--crate-name` controls the crate identity, and
`--extern-artifact` supplies a compiled dependency. `--no-link` stops before
linking. `--emit-object` and `--emit-llvm` preserve intermediate products.
`--emit-artifact` writes a `.rkca` artifact when used with `--no-link`, and
`--validate-artifact` checks an existing artifact.

## Standard-library artifacts

If the artifact is not already available, build it once from the repository
checkout:

```console
$ mkdir -p /tmp/rock-book-stdlib
$ /root/new_lang2/target/release/rockc \
    --entry-file /root/new_lang2/stdlib/lib.rk \
    --crate-name stdlib \
    --no-prelude \
    --emit-artifact /tmp/rock-book-stdlib/stdlib.rkca \
    --no-link
```

Passing an artifact named `stdlib` makes its prelude available. The compiler
does not search a global sysroot for an implicit standard library.

## `rockup`

`rockup` manages local toolchain material. With a release build in this
checkout, the concrete development sequence is:

```console
$ cargo run --manifest-path /root/new_lang2/rockup/Cargo.toml -- toolchain list
$ cargo run --manifest-path /root/new_lang2/rockup/Cargo.toml -- toolchain install dev --path /root/new_lang2/target/release
$ cargo run --manifest-path /root/new_lang2/rockup/Cargo.toml -- default dev
$ cargo run --manifest-path /root/new_lang2/rockup/Cargo.toml -- env
$ cargo run --manifest-path /root/new_lang2/rockup/Cargo.toml -- run dev -- /root/new_lang2/target/release/rock build
```

The selected environment can set `ROCKUP_HOME`, `ROCKUP_TOOLCHAIN`,
`ROCK_SYSROOT`, and `ROCKC`. Inspect the values printed by `rockup env` before
debugging a missing compiler or standard-library artifact.

## Grammar and editor support

The separate grammar project is at `/root/new_lang2/tree-sitter-rock`:

```console
$ cd /root/new_lang2/tree-sitter-rock
$ tree-sitter generate
$ tree-sitter build
$ tree-sitter test
```

It contains highlight and indentation queries for editor integrations. The
book's web snippets use a separate Highlight.js grammar, so editor installation
is not required to read the rendered book.

The formatter and compiler diagnostics are the primary feedback loop. There
is no mature language server or package-registry workflow yet.
