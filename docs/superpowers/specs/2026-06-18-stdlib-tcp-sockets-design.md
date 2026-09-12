# Stdlib TCP Sockets Design

## Context

Rock's stdlib currently has `Result`, `Option`, `Drop`, `Vec`, `String`, raw `extern` support in `stdlib/libc.rk`, and private-field stdlib wrappers such as `String`, `Vec`, and `HashMap`. It does not have networking support.

This design adds the first socket slice: safe, blocking IPv4 TCP wrappers in the stdlib. The public API should match Rock's existing language features: explicit `Result` errors, `Drop` cleanup, private raw fields, method-call and method-section ergonomics, and `unsafe` isolated inside stdlib implementation code.

## Goals

- Add blocking IPv4 TCP client/server support.
- Expose typed public network values instead of raw POSIX socket state.
- Return `Result` for all fallible operations.
- Close file descriptors automatically through `Drop`.
- Keep raw OS ABI details out of the public API.
- Add integration tests for localhost TCP behavior and error paths.

## Non-Goals

- Do not add async sockets, nonblocking mode, polling, epoll, select, or event loops.
- Do not add UDP, IPv6, DNS resolution, TLS, socket options, or Unix domain sockets in this slice.
- Do not add compiler-owned socket builtins or special stdlib loading behavior.
- Do not expose raw `sockaddr_in` or generic POSIX socket APIs as the primary user-facing API.
- Do not solve generic collection drop-glue issues or allocator design as part of sockets.

## Public API

Add a public `stdlib::net` module with these types:

```rock
enum SocketError
    Os I32
    InvalidAddress
    Closed

struct Ipv4Addr
    < a: U8
    < b: U8
    < c: U8
    < d: U8

struct SocketAddrV4
    < ip: Ipv4Addr
    < port: I64

struct TcpListener
    fd: I32

struct TcpStream
    fd: I32
```

Public methods:

```rock
impl Ipv4Addr
    localhost = -> Ipv4Addr
    any = -> Ipv4Addr
    new = a, b, c, d -> Ipv4Addr

impl SocketAddrV4
    new = ip, port -> SocketAddrV4
    localhost = port -> SocketAddrV4
    any = port -> SocketAddrV4

impl TcpListener
    bind: SocketAddrV4 -> Result TcpListener, SocketError
    @accept: TcpListener -> Result TcpStream, SocketError
    @local_addr: TcpListener -> Result SocketAddrV4, SocketError

impl TcpStream
    connect: SocketAddrV4 -> Result TcpStream, SocketError
    @read: TcpStream -> &mut [U8] -> Result I64, SocketError
    @write: TcpStream -> &[U8] -> Result I64, SocketError
    @write_str: TcpStream -> &Str -> Result I64, SocketError
    @shutdown: TcpStream -> Result Unit, SocketError
```

`TcpListener.fd` and `TcpStream.fd` remain private. Users interact through typed methods only.

## Ergonomics

The API should compose with existing `Result` operators and method sections:

```rock
(TcpListener::bind (SocketAddrV4::localhost 8080)
    >>= (.accept!)
    >>= (.write_str "hello")
    <&> (_ -> 0)).unwrap_or 1
```

This works because each fallible socket operation returns `Result`, and final success can be mapped into a process return code.

## OS Boundary

Add raw C/POSIX declarations and constants in stdlib-owned implementation modules, not as the primary public API. The first implementation uses this extern surface:

- `socket`
- `bind`
- `listen`
- `accept`
- `connect`
- `read`
- `write`
- `shutdown`
- `close`
- `getsockname`
- `errno` access through the smallest platform-specific extern wrapper the current target supports

If Rock struct layout cannot reliably represent `sockaddr_in`, the stdlib implementation should allocate a raw byte buffer for the address and write fields manually. Public `SocketAddrV4` stays platform-independent either way.

## Error Handling

- Any negative OS return value maps to `Result::Err (SocketError::Os errno)`.
- Invalid public addresses or ports map to `SocketError::InvalidAddress`.
- A read returning `0` initially remains `Result::Ok 0`, matching POSIX EOF behavior.
- `SocketError::Closed` is reserved for future explicit closed-state tracking and is not required to be produced in this slice.

## Resource Handling

Add `Drop` implementations:

```rock
impl Drop for TcpListener
    @drop = -> close self.fd

impl Drop for TcpStream
    @drop = -> close self.fd
```

Dropping a listener or stream closes the owned fd. This slice does not add fd duplication, ownership transfer, or manual `close` methods.

## Module Exports

- Add `< mod net` to `stdlib/lib.rk`.
- Do not re-export `stdlib::net::*` from `stdlib/prelude.rk` in the first slice. Users import or qualify `stdlib::net::TcpStream` and related types explicitly.

## Tests

Add integration tests in `lib/tests/integration.rs` for:

- Constructing `Ipv4Addr::localhost` and `SocketAddrV4::localhost`.
- Rejecting private `fd` field access on `TcpListener` and `TcpStream`.
- A localhost TCP roundtrip: bind listener, connect stream, accept stream, write bytes or `&Str`, read bytes, verify payload.
- Connection failure returns `Result::Err` for localhost port `1`. If the environment unexpectedly accepts connections on port `1`, skip only that assertion in the test helper rather than weakening the socket API.

Use port `0` plus `local_addr`/`getsockname` for the localhost roundtrip test. This avoids parallel port collisions and makes the test deterministic.

## Follow-Up Work

Create separate beads for:

- UDP sockets.
- IPv6 and `IpAddr` enum support.
- DNS resolution and hostname APIs.
- Nonblocking sockets and poll/event-loop abstractions.
- Socket options such as reuse-address, no-delay, and timeouts.
- TLS or higher-level protocol helpers.
