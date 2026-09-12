# rock_http

`rock_http` is a safe, blocking HTTP/1.1 server library for Rock. It provides
bounded request parsing, ordered headers, response framing, and a
thread-per-connection server over the standard library's safe networking and
threading APIs.

## Safety

The crate contains no `unsafe`, raw pointers, libc calls, compiler intrinsics,
or file-descriptor access. Low-level operations remain encapsulated by safe
standard-library types such as `TcpListener`, `TcpStream`, `Vec`, `String`,
`Arc`, and `spawn_detached`.

## Dependency

Add the package as a path dependency:

```toml
[dependencies]
rock_http = { path = "../rock_http" }
```

## Server

```rock
> stdlib::net::SocketAddrV4
> stdlib::result::Result
> rock_http::request::Request
> rock_http::response::Response
> rock_http::server::Server

handle: Request -> Response
handle = request ->
    if request.target.as_str! == "/health"
        Response::text 200, "ok"
    else
        Response::text 404, "not found"

main = ->
    address = SocketAddrV4::localhost 8080
    match Server::bind address
        Result::Ok server =>
            result = server.serve handle
            0
        Result::Err _ => 1
```

`Server::serve` shares the handler safely and detaches one owned worker per
connection. A request or response failure closes only that connection.

Ordinary programs should prefer `main = !->`: it evaluates the body for effects,
discards its result, and returns unit (`()`), which maps to process exit status
`0` without a final `0`. This example intentionally keeps `main = ->` to return
status `1` if binding fails and `0` if serving returns. Changing it to the discard
arrow would discard those statuses, not propagate failure to the process.

## Protocol Scope

The initial release supports:

- HTTP/1.0 and HTTP/1.1 request lines.
- Fixed request bodies through `Content-Length`.
- Requests fragmented across arbitrary TCP reads.
- Ordered duplicate headers with case-insensitive lookup.
- Text and binary responses.
- Exact `Content-Length` response framing.
- One request and response per connection with `Connection: close`.
- Configurable request-line, header, header-count, and body limits.
- `400`, `413`, `431`, and `501` protocol error responses.

The initial release intentionally does not support chunked transfer coding,
keep-alive, pipelining, TLS, compression, multipart parsing, WebSockets,
async I/O, HTTP/2, or HTTP/3.

## Build

From `rock_http/example/`:

```console
$ rock build
$ rock run
```

The full design and safety contract are recorded in
`docs/superpowers/specs/2026-08-21-rock-http-server-design.md`.
