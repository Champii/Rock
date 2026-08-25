# Input, Output, and Files

Rock's I/O abstractions use `Result` and ownership rather than exceptions or manual close calls. `File`, `Stdin`, `Stdout`, and sockets implement the shared `Read` and `Write` traits, so generic code can work with more than one kind of byte stream.

## The `Read` and `Write` traits

`Read` takes a mutable byte slice and reports a byte count or `IoError`. `Write` reports a byte count for a slice or string and provides a helper for writing a prefix repeatedly. Import the trait when calling its methods through a generic bound or a concrete value.

```rock
> stdlib::fs::File
> stdlib::io::IoError
> stdlib::io::Read
> stdlib::io::Write

write_text: &mut W -> &Str -> Result I64, IoError where W: Write
write_text = writer, text ->
    written = writer.write_str text?
    Result::Ok written

read_text_prefix: &mut R -> &mut [U8] -> Result I64, IoError where R: Read
read_text_prefix = reader, buffer ->
    reader.read buffer

write_demo: &Str -> Result I64, IoError
write_demo = path ->
    mut file = File::create path?
    write_text &mut file, "hello"

main = ->
    match write_demo "rock-io-traits.txt"
        Result::Ok count => count.println!
        Result::Err _ => -1 .println!
    0
```

This example writes five bytes and prints `5`; `read_text_prefix` has the same contract for a reader even though `main` only demonstrates the write path. `read` needs an exclusive buffer because it fills memory. `write` and `write_str` borrow their source bytes, and `write_all_prefix` keeps writing until the requested prefix is complete or an error occurs. The file closes automatically when its owner is dropped.

## Files

`File::open` reads an existing path, `File::create` creates or truncates a path, and `File::append` creates or appends. Each returns `Result File, IoError`, so a function that uses `?` must return a compatible `Result`.

```rock
> stdlib::fs::File
> stdlib::io::IoError
> stdlib::io::Write

write_greeting: &Str -> Result I64, IoError
write_greeting = path ->
    mut file = File::create path?
    file.write_str "Hello from Rock\n"

append_greeting: &Str -> Result I64, IoError
append_greeting = path ->
    mut file = File::append path?
    file.write_str "Again\n"

main = ->
    match write_greeting "rock-greeting.txt"
        Result::Ok count =>
            match append_greeting "rock-greeting.txt"
                Result::Ok appended =>
                    count + appended .println!
                    0
                Result::Err _ => 1
        Result::Err _ => 1
```

The successful output is `22`, and `rock-greeting.txt` contains `Hello from Rock` followed by `Again`, each followed by a newline. The descriptor is closed by `Drop`; callers do not call a separate close API. An absent path reports an operating-system `IoError::Os code`. A path containing a null byte reports `IoError::InvalidPath` before the operating-system call.

## Reading bytes

Only the prefix indicated by the returned count contains newly read bytes. A zero count means end of file. This complete program creates its input first, then opens it and prints the count plus two byte values.

```rock
> stdlib::fs::File
> stdlib::io::IoError
> stdlib::io::Read
> stdlib::io::Write

prepare: &Str -> Result I64, IoError
prepare = path ->
    mut file = File::create path?
    file.write_str "world"

read_file: &Str -> Result I64, IoError
read_file = path ->
    mut file = File::open path?
    mut bytes: [U8; 5] = [0, 0, 0, 0, 0]
    count = file.read (&mut bytes)?
    count.println!
    bytes[0] as I64 .println!
    bytes[4] as I64 .println!
    Result::Ok count

main = ->
    match prepare "rock-io-input.txt"
        Result::Ok _ =>
            match read_file "rock-io-input.txt"
                Result::Ok _ => 0
                Result::Err _ => 1
        Result::Err _ => 1
```

The output is `5`, `119`, and `100`, corresponding to the five bytes and the letters `w` and `d`. The buffer is borrowed mutably only during `read`; the file remains the owner of its descriptor and is closed after `read_file` returns.

## Standard streams

`stdout!` and `stdin!` create lightweight handles. The output example is deterministic and avoids requiring interactive input; `Stdin` implements the same `Read` contract for a caller that supplies a mutable buffer.

```rock
> stdlib::io::stdout
> stdlib::io::Write

main = ->
    mut output = stdout!
    bytes: [U8; 5] = [104, 101, 108, 108, 111]
    match output.write_all (&bytes[..3])
        Result::Ok count => count
        Result::Err _ => 1
    0
```

The process writes `hel` to standard output and returns `0`. The native range creates a borrowed three-byte view, and `write_all` handles partial operating-system writes until the whole slice is sent or an error occurs.

Reading standard input uses the same ownership contract. This program is compile-tested but requires input at runtime; with `abc` on standard input it prints `3`.

```rock
> stdlib::io::IoError
> stdlib::io::Read
> stdlib::io::stdin

read_stdin: () -> Result I64, IoError
read_stdin = ->
    mut input = stdin!
    mut buffer: [U8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    input.read &mut buffer

main = ->
    match read_stdin!
        Result::Ok count => count.println!
        Result::Err _ => -1 .println!
    0
```

## Copying and the `|>>` operator

The generic `copy` function reads from `&mut R` and writes to `&W` until end of file. `|>>` is its pipeline-oriented spelling and consumes the source reader while borrowing the destination writer. This program creates a source file so the example has no hidden input.

```rock
> stdlib::fs::File
> stdlib::io::IoError
> stdlib::io::Write

prepare: &Str -> Result I64, IoError
prepare = path ->
    mut file = File::create path?
    file.write_str "functional byte pipe"

copy_file: &Str -> &Str -> Result I64, IoError
copy_file = source, target ->
    output = File::create target?
    (File::open source?) |>> &output

main = ->
    match prepare "rock-copy-input.txt"
        Result::Ok _ =>
            match copy_file "rock-copy-input.txt", "rock-copy-output.txt"
                Result::Ok copied =>
                    copied.println!
                    0
                Result::Err _ => 1
        Result::Err _ => 1
```

The output is `20`, and the destination contains the same 20 bytes. The source `File` is moved into the pipe, while `output` remains the destination owner. This shape is useful for generic file, socket, and standard-stream code.

## Program arguments

`args!` is an explicitly imported function that returns an owned `Vec String` snapshot. It includes `argv[0]`, so a run with no user arguments still has length `1`.

```rock
> stdlib::env::args

main = ->
    values: Vec String = args!
    values.len!.println!
    0
```

With no user arguments the output is `1`; with `first second` after the run separator it is `3`. The vector owns each argument string, and there is currently no standard environment-variable API.

## Errors, ownership, and portability

Use a concrete error type such as `IoError` in public signatures. Match `Result::Ok` and `Result::Err` at the boundary, or use `?` in a helper that returns the same carrier. Moving a `File` transfers descriptor ownership; borrowing it for `Read` or `Write` leaves the caller responsible for the eventual drop. A failed `?` returns early and does not restore values already moved into the failed operation.

The current file and stream implementations are POSIX-oriented. Error numbers and some behavior depend on the host operating system, and the examples that create files require a writable working directory.
