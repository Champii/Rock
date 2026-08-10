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
