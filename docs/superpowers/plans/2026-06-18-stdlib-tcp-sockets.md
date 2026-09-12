# Stdlib TCP Sockets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add safe blocking IPv4 TCP sockets to the Rock stdlib.

**Architecture:** Keep raw POSIX declarations in `stdlib/libc.rk`, implement typed wrappers in `stdlib/net.rk`, and expose only `Result`-returning safe APIs. Use raw 16-byte `sockaddr_in` buffers inside stdlib so public Rock types stay platform-independent.

**Tech Stack:** Rock stdlib files under `stdlib/`; Rust integration tests in `lib/tests/integration.rs`; verification through focused `cargo test -p rock-lib --test integration ...` commands and full `cargo test -p rock-lib`.

**VCS Note:** Do not stage, commit, push, amend, or otherwise mutate VCS state unless the current user explicitly asks.

---

## File Structure

- Modify `stdlib/libc.rk`: add POSIX networking extern declarations and byte-order helpers.
- Create `stdlib/net.rk`: define public socket types, internal sockaddr helpers, error mapping, TCP listener/stream APIs, and `Drop` cleanup.
- Modify `stdlib/lib.rk`: declare `< mod net` so the module is included in stdlib artifacts.
- Do not modify `stdlib/prelude.rk`: network types stay explicitly imported or qualified.
- Modify `lib/tests/integration.rs`: add focused tests for address constructors, private fields, TCP roundtrip, connection errors, and errno/sockaddr proof behavior.

---

### Task 1: Prove POSIX FFI Surface

**Files:**
- Modify: `lib/src/lexer/lexer.rs`
- Modify: nearby lexer/parser tests if needed
- Modify: `stdlib/libc.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add a failing leading-underscore identifier regression**

Add the smallest lexer or parser regression that proves `__errno_location` is tokenized as one identifier, not as underscore tokens. Prefer a lexer unit test near existing lexer tests.

- [ ] **Step 2: Run the identifier regression and verify it fails**

Run the smallest relevant lexer/parser test command.

Expected: FAIL because the lexer currently emits `_` tokens for leading underscores.

- [ ] **Step 3: Allow leading-underscore identifiers without breaking wildcard `_`**

Update lexing so `_` remains `TokenType::Underscore` when it appears alone, but `_name` and `__name` lex as `TokenType::Ident`.

- [ ] **Step 4: Run the identifier regression and verify it passes**

Run the same focused lexer/parser test command.

Expected: PASS.

- [ ] **Step 5: Add a failing FFI proof test**

Append this test near other stdlib integration tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_socket_posix_ffi_surface_compiles_and_returns_errno() {
    let output = compile_and_run(
        r#"
> stdlib::libc::socket
> stdlib::libc::close
> stdlib::libc::__errno_location

last_errno = ->
    ptr = __errno_location ()
    unsafe *ptr

main = ->
    fd = socket (-1), 1, 0
    if fd < 0
        last_errno! .println!
    else
        close fd .println!
    0
"#,
    );

    let errno = output.trim().parse::<i64>().expect("errno should print as integer");
    assert!(errno > 0, "invalid socket domain should set errno, got {errno}");
}
```

- [ ] **Step 6: Run the proof test and verify it fails**

Run:

```bash
cargo test -p rock-lib --test integration test_socket_posix_ffi_surface_compiles_and_returns_errno -- --exact --nocapture
```

Expected: FAIL because `stdlib::libc::socket`, `close`, and `__errno_location` are not declared.

- [ ] **Step 7: Add POSIX externs to `stdlib/libc.rk`**

Append this section after the existing I/O externs:

```rock
// POSIX sockets and file descriptors
< extern socket: I32 -> I32 -> I32 -> I32
< extern bind: I32 -> *U8 -> I32 -> I32
< extern listen: I32 -> I32 -> I32
< extern accept: I32 -> *U8 -> *I32 -> I32
< extern connect: I32 -> *U8 -> I32 -> I32
< extern read: I32 -> *U8 -> I64 -> I64
< extern write: I32 -> *U8 -> I64 -> I64
< extern shutdown: I32 -> I32 -> I32
< extern close: I32 -> I32
< extern getsockname: I32 -> *U8 -> *I32 -> I32
< extern htons: U16 -> U16
< extern ntohs: U16 -> U16
< extern htonl: U32 -> U32
< extern ntohl: U32 -> U32
< extern __errno_location: Unit -> *I32
```

- [ ] **Step 8: Run the proof test and verify it passes**

Run:

```bash
cargo test -p rock-lib --test integration test_socket_posix_ffi_surface_compiles_and_returns_errno -- --exact --nocapture
```

Expected: PASS. If the only failure is zero-argument extern call syntax for `__errno_location ()`, keep the test and fix the call syntax in the test and later `net.rk`; do not continue until errno access works.

---

### Task 2: Add Address Types And Module Export

**Files:**
- Create: `stdlib/net.rk`
- Modify: `stdlib/lib.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing address constructor test**

Append this test near the POSIX proof test:

```rust
#[test]
fn test_socket_addr_v4_constructors() {
    let output = compile_and_run(
        r#"
> stdlib::net::*

main = ->
    local = Ipv4Addr::localhost!
    any = Ipv4Addr::any!
    custom = Ipv4Addr::new 1, 2, 3, 4
    addr = SocketAddrV4::localhost 8080
    local.a.println!
    local.d.println!
    any.a.println!
    custom.c.println!
    addr.port.println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["127", "1", "0", "3", "8080"]);
}
```

- [ ] **Step 2: Run the address test and verify it fails**

Run:

```bash
cargo test -p rock-lib --test integration test_socket_addr_v4_constructors -- --exact --nocapture
```

Expected: FAIL because `stdlib::net` is missing.

- [ ] **Step 3: Create `stdlib/net.rk` with address and error types**

Create `stdlib/net.rk` with this content:

```rock
// Blocking IPv4 TCP networking.

> stdlib::libc::malloc
> stdlib::libc::free
> stdlib::libc::__errno_location
> stdlib::result::Result
> stdlib::drop::Drop

< enum SocketError
    Os I32
    InvalidAddress
    Closed

< struct Ipv4Addr
    < a: U8
    < b: U8
    < c: U8
    < d: U8

< struct SocketAddrV4
    < ip: Ipv4Addr
    < port: I64

< struct TcpListener
    fd: I32

< struct TcpStream
    fd: I32

AF_INET = -> 2
SOCK_STREAM = -> 1
SOCKADDR_IN_LEN = -> 16
SOMAXCONN_DEFAULT = -> 128

last_os_error = ->
    ptr = __errno_location ()
    SocketError::Os (unsafe *ptr)

impl Ipv4Addr
    localhost = ->
        Ipv4Addr
            a: 127
            b: 0
            c: 0
            d: 1

    any = ->
        Ipv4Addr
            a: 0
            b: 0
            c: 0
            d: 0

    new = a, b, c, d ->
        Ipv4Addr
            a: a
            b: b
            c: c
            d: d

impl SocketAddrV4
    new = ip, port ->
        SocketAddrV4
            ip: ip
            port: port

    localhost = port -> SocketAddrV4::new (Ipv4Addr::localhost!), port

    any = port -> SocketAddrV4::new (Ipv4Addr::any!), port
```

- [ ] **Step 4: Declare `net` from `stdlib/lib.rk`**

Insert after the `hash_map` module declaration:

```rock
// Net module - blocking IPv4 TCP sockets
< mod net
```

Do not add `stdlib::net::*` to `stdlib/prelude.rk`.

- [ ] **Step 5: Run the address test and verify it passes**

Run:

```bash
cargo test -p rock-lib --test integration test_socket_addr_v4_constructors -- --exact --nocapture
```

Expected: PASS.

---

### Task 3: Add Internal Sockaddr Helpers

**Files:**
- Modify: `stdlib/net.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add a failing `local_addr` port-zero test**

Append this test:

```rust
#[test]
fn test_tcp_listener_bind_port_zero_reports_local_addr() {
    let output = compile_and_run(
        r#"
> stdlib::net::*

main = ->
    match (TcpListener::bind (SocketAddrV4::localhost 0))
        Result::Ok listener =>
            match listener.local_addr!
                Result::Ok addr =>
                    addr.port > 0 .println!
                    addr.ip.a.println!
                    addr.ip.d.println!
                    0
                Result::Err _ => 2
        Result::Err _ => 1
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["true", "127", "1"]);
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
cargo test -p rock-lib --test integration test_tcp_listener_bind_port_zero_reports_local_addr -- --exact --nocapture
```

Expected: FAIL because `TcpListener::bind` and `local_addr` are missing.

- [ ] **Step 3: Extend `stdlib/net.rk` imports**

Add these imports below the existing `libc` imports:

```rock
> stdlib::libc::socket
> stdlib::libc::bind
> stdlib::libc::listen
> stdlib::libc::getsockname
> stdlib::libc::htons
> stdlib::libc::ntohs
> stdlib::libc::htonl
> stdlib::libc::ntohl
> stdlib::libc::close
```

- [ ] **Step 4: Add internal sockaddr helpers to `stdlib/net.rk`**

Append these helpers before the `impl Ipv4Addr` block:

```rock
port_is_valid = port -> port >= 0 && port <= 65535

ipv4_to_u32 = ip ->
    ((ip.a as U32) << (24 as U32)) |
        ((ip.b as U32) << (16 as U32)) |
        ((ip.c as U32) << (8 as U32)) |
        (ip.d as U32)

u32_to_ipv4 = raw ->
    Ipv4Addr
        a: ((raw >> (24 as U32)) & (255 as U32)) as U8
        b: ((raw >> (16 as U32)) & (255 as U32)) as U8
        c: ((raw >> (8 as U32)) & (255 as U32)) as U8
        d: (raw & (255 as U32)) as U8

sockaddr_v4_new = addr ->
    buf = malloc (SOCKADDR_IN_LEN!)
    family = buf as *U16
    port = (buf + 2) as *U16
    ip = (buf + 4) as *U32
    i = 0
    while i < SOCKADDR_IN_LEN!
        unsafe buf[i] = 0
        i = i + 1
    unsafe
        family[0] = (AF_INET!) as U16
        port[0] = htons (addr.port as U16)
        ip[0] = htonl (ipv4_to_u32 addr.ip)
    buf

sockaddr_v4_read = buf ->
    port_ptr = (buf + 2) as *U16
    ip_ptr = (buf + 4) as *U32
    port = ntohs (unsafe port_ptr[0])
    ip = ntohl (unsafe ip_ptr[0])
    SocketAddrV4
        ip: u32_to_ipv4 ip
        port: port as I64
```

- [ ] **Step 5: Add `TcpListener.bind` and `local_addr`**

Append this implementation to `stdlib/net.rk`:

```rock
impl TcpListener
    bind: SocketAddrV4 -> Result TcpListener, SocketError
    bind = addr ->
        if !(port_is_valid addr.port)
            Result::Err SocketError::InvalidAddress
        else
            fd = socket (AF_INET!), (SOCK_STREAM!), 0
            if fd < 0
                Result::Err (last_os_error!)
            else
                raw_addr = sockaddr_v4_new addr
                bind_result = bind fd, raw_addr, (SOCKADDR_IN_LEN!)
                free raw_addr
                if bind_result < 0
                    close fd
                    Result::Err (last_os_error!)
                else
                    listen_result = listen fd, (SOMAXCONN_DEFAULT!)
                    if listen_result < 0
                        close fd
                        Result::Err (last_os_error!)
                    else
                        Result::Ok (TcpListener
                            fd: fd)

    @local_addr: TcpListener -> Result SocketAddrV4, SocketError
    @local_addr = ->
        raw_addr = malloc (SOCKADDR_IN_LEN!)
        len_ptr = (malloc 4) as *I32
        unsafe len_ptr[0] = (SOCKADDR_IN_LEN!) as I32
        result = getsockname self.fd, raw_addr, len_ptr
        if result < 0
            free raw_addr
            free (len_ptr as *U8)
            Result::Err (last_os_error!)
        else
            addr = sockaddr_v4_read raw_addr
            free raw_addr
            free (len_ptr as *U8)
            Result::Ok addr

impl Drop for TcpListener
    @drop = -> close self.fd
```

- [ ] **Step 6: Run the port-zero test**

Run:

```bash
cargo test -p rock-lib --test integration test_tcp_listener_bind_port_zero_reports_local_addr -- --exact --nocapture
```

Expected: PASS.

---

### Task 4: Implement TcpStream Connect, Accept, Read, Write

**Files:**
- Modify: `stdlib/net.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing localhost roundtrip test**

Append this test:

```rust
#[test]
fn test_tcp_listener_stream_localhost_roundtrip() {
    let output = compile_and_run(
        r#"
> stdlib::net::*

main = ->
    match (TcpListener::bind (SocketAddrV4::localhost 0))
        Result::Ok listener =>
            match listener.local_addr!
                Result::Ok addr =>
                    match (TcpStream::connect addr)
                        Result::Ok client =>
                            match listener.accept!
                                Result::Ok server =>
                                    match (client.write_str "ping")
                                        Result::Ok written =>
                                            mut buf = [0 as U8, 0 as U8, 0 as U8, 0 as U8]
                                            slice = &mut buf
                                            match (server.read slice)
                                                Result::Ok read =>
                                                    written.println!
                                                    read.println!
                                                    buf[0] as I64 .println!
                                                    buf[3] as I64 .println!
                                                    0
                                                Result::Err _ => 5
                                        Result::Err _ => 4
                                Result::Err _ => 3
                        Result::Err _ => 2
                Result::Err _ => 1
        Result::Err _ => 9
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["4", "4", "112", "103"]);
}
```

- [ ] **Step 2: Run the roundtrip test and verify it fails**

Run:

```bash
cargo test -p rock-lib --test integration test_tcp_listener_stream_localhost_roundtrip -- --exact --nocapture
```

Expected: FAIL because `TcpStream::connect`, `TcpListener.accept`, `read`, and `write_str` are missing.

- [ ] **Step 3: Extend `stdlib/net.rk` imports**

Add these imports:

```rock
> stdlib::libc::accept
> stdlib::libc::connect
> stdlib::libc::read
> stdlib::libc::write
> stdlib::libc::shutdown
```

- [ ] **Step 4: Add `TcpListener.accept` and `TcpStream` methods**

Append this code to `stdlib/net.rk` before the `Drop for TcpListener` impl, moving the existing `Drop for TcpListener` to the end of the file if necessary:

```rock
impl TcpListener
    @accept: TcpListener -> Result TcpStream, SocketError
    @accept = ->
        raw_addr = malloc (SOCKADDR_IN_LEN!)
        len_ptr = (malloc 4) as *I32
        unsafe len_ptr[0] = (SOCKADDR_IN_LEN!) as I32
        fd = accept self.fd, raw_addr, len_ptr
        free raw_addr
        free (len_ptr as *U8)
        if fd < 0
            Result::Err (last_os_error!)
        else
            Result::Ok (TcpStream
                fd: fd)

impl TcpStream
    connect: SocketAddrV4 -> Result TcpStream, SocketError
    connect = addr ->
        if !(port_is_valid addr.port)
            Result::Err SocketError::InvalidAddress
        else
            fd = socket (AF_INET!), (SOCK_STREAM!), 0
            if fd < 0
                Result::Err (last_os_error!)
            else
                raw_addr = sockaddr_v4_new addr
                result = connect fd, raw_addr, (SOCKADDR_IN_LEN!)
                free raw_addr
                if result < 0
                    close fd
                    Result::Err (last_os_error!)
                else
                    Result::Ok (TcpStream
                        fd: fd)

    @read: TcpStream -> &mut [U8] -> Result I64, SocketError
    @read = buf ->
        ptr = (~ArrPtr (*buf)) as *U8
        len = ~ArrayLen (*buf)
        n = read self.fd, ptr, len
        if n < 0
            Result::Err (last_os_error!)
        else
            Result::Ok n

    @write: TcpStream -> &[U8] -> Result I64, SocketError
    @write = buf ->
        ptr = (~ArrPtr buf) as *U8
        len = ~ArrayLen buf
        n = write self.fd, ptr, len
        if n < 0
            Result::Err (last_os_error!)
        else
            Result::Ok n

    @write_str: TcpStream -> &Str -> Result I64, SocketError
    @write_str = s ->
        ptr = (~ArrPtr s) as *U8
        len = ~ArrayLen s
        n = write self.fd, ptr, len
        if n < 0
            Result::Err (last_os_error!)
        else
            Result::Ok n

    @shutdown: TcpStream -> Result Unit, SocketError
    @shutdown = ->
        result = shutdown self.fd, 2
        if result < 0
            Result::Err (last_os_error!)
        else
            Result::Ok ()

impl Drop for TcpStream
    @drop = -> close self.fd
```

Keep one `impl Drop for TcpListener` in the file:

```rock
impl Drop for TcpListener
    @drop = -> close self.fd
```

- [ ] **Step 5: Run the roundtrip test**

Run:

```bash
cargo test -p rock-lib --test integration test_tcp_listener_stream_localhost_roundtrip -- --exact --nocapture
```

Expected: PASS.

---

### Task 5: Add Error And Privacy Tests

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add private field tests**

Append these tests:

```rust
#[test]
fn test_tcp_listener_fd_field_is_private() {
    compile_should_fail(
        r#"
> stdlib::net::*

main = ->
    match (TcpListener::bind (SocketAddrV4::localhost 0))
        Result::Ok listener =>
            fd = listener.fd
            0
        Result::Err _ => 1
"#,
        "Field 'fd' of struct 'TcpListener' is private",
    );
}

#[test]
fn test_tcp_stream_fd_field_is_private() {
    compile_should_fail(
        r#"
> stdlib::net::*

main = ->
    match (TcpStream::connect (SocketAddrV4::localhost 1))
        Result::Ok stream =>
            fd = stream.fd
            0
        Result::Err _ => 1
"#,
        "Field 'fd' of struct 'TcpStream' is private",
    );
}
```

- [ ] **Step 2: Add connection error test**

Append this test:

```rust
#[test]
fn test_tcp_stream_connect_failure_returns_error() {
    let output = compile_and_run(
        r#"
> stdlib::net::*

main = ->
    match (TcpStream::connect (SocketAddrV4::localhost 1))
        Result::Ok _ => 0.println!
        Result::Err _ => 1.println!
    0
"#,
    );

    assert_eq!(output.trim(), "1");
}
```

- [ ] **Step 3: Run focused tests**

Run these one at a time:

```bash
cargo test -p rock-lib --test integration test_tcp_listener_fd_field_is_private -- --exact --nocapture
```

Expected: PASS.

```bash
cargo test -p rock-lib --test integration test_tcp_stream_fd_field_is_private -- --exact --nocapture
```

Expected: PASS.

```bash
cargo test -p rock-lib --test integration test_tcp_stream_connect_failure_returns_error -- --exact --nocapture
```

Expected: PASS. If localhost port `1` is open in the environment, change the Rust assertion to accept either `"1"` or `"0"` and add a comment that port `1` is environment-dependent; do not change `TcpStream::connect` semantics.

---

### Task 6: Final Verification

**Files:**
- Verify: `stdlib/libc.rk`
- Verify: `stdlib/net.rk`
- Verify: `stdlib/lib.rk`
- Verify: `stdlib/prelude.rk`
- Verify: `lib/tests/integration.rs`
- Verify: `docs/superpowers/specs/2026-06-18-stdlib-tcp-sockets-design.md`
- Verify: `docs/superpowers/plans/2026-06-18-stdlib-tcp-sockets.md`

- [ ] **Step 1: Confirm net is not in the prelude**

Run:

```bash
git diff -- stdlib/prelude.rk
```

Expected: no diff for `stdlib/prelude.rk`.

- [ ] **Step 2: Run formatting check**

Run:

```bash
cargo fmt --all --check
```

Expected: PASS.

- [ ] **Step 3: Run whitespace check**

Run:

```bash
git diff --check
```

Expected: PASS with no whitespace errors.

- [ ] **Step 4: Run full rock-lib tests**

Run:

```bash
cargo test -p rock-lib
```

Expected: PASS. Baseline before this work was `1654` library tests, `293` integration tests, parser test, and doctests passing; integration count should increase by the socket tests.

- [ ] **Step 5: Inspect final diff**

Run:

```bash
git diff -- stdlib/libc.rk stdlib/net.rk stdlib/lib.rk stdlib/prelude.rk lib/tests/integration.rs docs/superpowers/specs/2026-06-18-stdlib-tcp-sockets-design.md docs/superpowers/plans/2026-06-18-stdlib-tcp-sockets.md
```

Expected: diff is limited to POSIX externs, `net` module, stdlib module declaration, socket tests, and docs.

---

## Self-Review Notes

- Spec coverage: blocking IPv4 TCP, typed public values, `Result` errors, `Drop`, private fd fields, explicit import policy, localhost roundtrip, and failure cases are covered.
- Placeholder scan: no placeholder implementation steps remain.
- Type consistency: method names and signatures match the approved spec, with `TcpListener.local_addr` included for deterministic port-zero tests.
