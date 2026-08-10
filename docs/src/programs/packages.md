# Packages and Dependencies

A package is a buildable Rock crate described by `rock.toml`. The manifest names the crate, selects its source entry, and records dependencies. `rock` builds a package graph and materializes artifacts; `rockc` can consume those artifacts directly when a script needs exact inputs.

## A minimal package

The manifest requires `[crate]`, `[lib]`, a crate name, a version, and a source path. The complete project is:

Project tree:

```text
hello-package/
|-- rock.toml
`-- main.rk
```

### `rock.toml`

```toml
[crate]
name = "hello"
version = "0.1.0"

[lib]
path = "main.rk"
```

### `main.rk`

```rock
main = ->
    "project entry point".println!
    0
```

From `hello-package/`, `rock build` compiles the entry and writes products below `build/`; `rock run` builds and executes the linked program. The output is `project entry point` and the program returns `0`. The manifest key is called `lib.path` even when the selected source contains an executable `main` function.

## A path dependency

Path dependencies are the supported way to compose local packages today. The example has two complete projects and shows the whole source surface on both sides of the dependency.

Project tree:

```text
workspace/
|-- geometry/
|   |-- rock.toml
|   `-- main.rk
`-- app/
    |-- rock.toml
    `-- main.rk
```

### `geometry/rock.toml`

```toml
[crate]
name = "geometry"
version = "0.1.0"

[lib]
path = "main.rk"
```

### `geometry/main.rk`

```rock
< struct Point
    < x: I64
    < y: I64

distance_squared: Point -> I64
distance_squared = point -> point.x * point.x + point.y * point.y

< distance_squared
```

### `app/rock.toml`

```toml
[crate]
name = "geometry-app"
version = "0.1.0"

[lib]
path = "main.rk"

[dependencies]
geometry = { path = "../geometry" }
```

### `app/main.rk`

```rock
> geometry::Point
> geometry::distance_squared

main = ->
    point = Point
        x: 3
        y: 4
    (distance_squared point).println!
    0
```

`rock build` builds `geometry` before `geometry-app`, then passes the fresh dependency artifact to the root compilation. The application prints `25`. The dependency name in the manifest is also the crate path used by `> geometry::Point` and `> geometry::distance_squared`; the imported names must have been exported by `geometry/main.rk`.

Version-only registry dependencies are accepted by manifest parsing but are not resolved by the current package workflow. Use a path dependency for a local project and do not assume that `version = "1.0"` downloads or locates a package.

## Crate artifacts and `rockc`

`rock artifact` produces a reusable `.rkca` product containing the typed exported interface, generic bodies needed by consumers, operator declarations, dependency records, and object-linkage metadata. Application authors normally let `rock` manage those paths. A direct `rockc` invocation names every artifact explicitly:

```console
$ rockc --entry-file main.rk \
    --extern-artifact stdlib=build/stdlib.rkca \
    --extern-artifact geometry=../geometry/build/artifacts/geometry-0.1.0.rkca
```

The compiler does not search neighboring source directories for an implicit dependency. A missing or mismatched artifact is a build error, not an invitation to compile a source directory automatically.

## Prelude and `no_std`

Passing the `stdlib` artifact makes its prelude available automatically. A low-level package can opt out in its manifest. The complete no-stdlib source uses only a primitive return value, so it does not rely on prelude operators or types.

Project tree:

```text
bare-package/
|-- rock.toml
`-- lib.rk
```

### `rock.toml`

```toml
[crate]
name = "bare"
version = "0.1.0"
no_std = true

[lib]
path = "lib.rk"
```

### `lib.rk`

```rock
main = ->
    0
```

Without the prelude, `String`, `Option`, familiar methods, and standard operator implementations are not injected. This is intentional: primitive operator meanings are supplied by explicit library declarations and implementations rather than by compiler-owned fallbacks. Add an explicit dependency and import when a low-level package needs a particular capability.

## Project commands and arguments

The package commands operate from the directory containing `rock.toml`:

```console
$ rock format
$ rock build
$ rock run -- first second
$ rock expand
$ rock artifact
```

`rock run -- first second` passes two user arguments after the separator; the program's argument API also includes `argv[0]`. There is currently no `rock test` subcommand. Use ordinary Rock programs or an external harness for application-level tests.

## Common mistakes and current limits

- Keep the manifest's crate name, imported crate path, and artifact name aligned.
- Keep `[lib].path` relative to the package root and ensure the file exists.
- Build dependencies before invoking `rockc`; `--extern-artifact` accepts artifacts, not source directories.
- Do not expect registry resolution, implicit source discovery, or compiler-owned stdlib injection.
- `no_std = true` is a package-level choice; code that uses prelude types must opt into the appropriate explicit dependency instead.
