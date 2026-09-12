# Projects and the Toolchain

A project gives `rock` enough information to find an entry file, resolve path modules, and choose build output locations. This chapter starts with one file; [Modules and Visibility](../programs/modules.md) later shows the complete source of a multi-file project, and [Packages and Dependencies](../programs/packages.md) builds two path-dependent packages.

## The manifest

Create a directory containing `rock.toml`:

```toml
[crate]
name = "hello"
version = "0.1.0"

[lib]
path = "main.rk"
```

Put a complete Rock program in `main.rk`:

```rock
main = ->
    "project entry point".println!
    0
```

Rock currently calls the source entry a library path even when it contains the executable `main` function. From the project directory, build and run it:

```console
$ rock build
$ rock run
project entry point
```

Pass application arguments after `--`:

```console
$ rock run -- first second
```

The project command writes products below `build/`, including object files, crate artifacts, and a linked executable when requested.

## Everyday commands

```console
$ rock format     # Format the configured entry source
$ rock build      # Build the current project
$ rock run -- first second # Build and run it with two arguments
$ rock expand     # Print macro-expanded source
$ rock artifact   # Materialize a reusable crate artifact
```

There is currently no `rock test` command. Compiler contributors use Cargo's Rust test suite; application tests are ordinary Rock programs or external harnesses.

## Formatting caution

The formatter is still maturing. Known rough edges affect some trait implementation headers, nested generic applications, and `&mut` expressions. Keep source under version control, inspect formatter changes, and compile after formatting nontrivial code.

## One project workflow

Keep even small learning programs in a project with `rock.toml`. This gives examples, scripts, editors, and larger packages the same `rock format`, `rock build`, and `rock run` workflow from the beginning.

## How the editor finds your program

When you open a `.rk` file, `rock-lsp` searches its parent directories for the nearest `rock.toml`. It uses the manifest's entry path and crate name, resolves path dependencies through the same project code as `rock build`, and selects the toolchain standard library. Ordinary projects do not need a hand-written list of editor dependency artifacts.

For the project above, opening `main.rk` analyzes the program selected by `[lib].path`. In a multi-file project, the server still starts from that entry and follows its module graph; opening an unrelated `.rk` file below the same manifest does not automatically include it in the program. See [Modules and Visibility](../programs/modules.md) for declaring modules.

Open source buffers are supplied to analysis with their unsaved text. Files in the entry graph that are not open are read from disk. Path dependencies are resolved as compiled artifacts, rather than merged into one live source workspace, so save dependency changes before expecting a consuming project to see them. Resolving dependencies can build or refresh artifacts and write build outputs; editor startup is not necessarily a read-only operation.

Project resolution is cached. After changing `rock.toml`, dependency files, or the selected toolchain, save a Rock source buffer to invalidate its cached project resolution, or restart the language server. In Neovim, use `:RockLspRestart`. Unsaved manifest edits are not a replacement for the manifest on disk.

Without a manifest, the server analyzes the opened file as a standalone entry and does not automatically supply the toolchain standard library. Prefer a small project like the one above for learning, especially when using operators or prelude methods.

## From editor feedback to a build

Live diagnostics help you fix syntax, name resolution, and type errors before saving. They are not a successful build: the editor's source analysis does not run borrow checking, code generation, or linking for the main project.

Save your source, then check the complete program:

```console
$ rock build
$ rock run
```

If the build fails, read the first diagnostic's message and source labels before the final command-failure summary. A label can point to a module or dependency rather than the entry file. [Editors and Diagnostics](editor-and-diagnostics.md) walks through both source errors and setup failures.
