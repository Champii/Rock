# Safe Rock HTTP Server Design

## Status

Approved for implementation on 2026-08-21.

The initial phases 1 through 5 release is implemented. Phase 6 remains the
separate production-hardening scope described below; the current package does
not claim slow-client resistance or graceful shutdown.

## Goal

Build `rock_http` as a reusable Rock path-dependency package implementing a
bounded, blocking HTTP/1.1 server. Protocol code belongs in `rock_http`; only
generic I/O and socket capabilities belong in `stdlib`.

The first release must be correct and useful within its documented scope. It
does not attempt to provide async I/O, TLS, WebSockets, chunked transfer
coding, persistent connections, HTTP/2, or HTTP/3.

## Safety Boundary

`rock_http` must use safe abstractions exclusively.

- No `unsafe` blocks or unsafe function declarations.
- No raw pointers.
- No direct libc imports or calls.
- No compiler intrinsics.
- No direct file-descriptor access.
- No dependency on private stdlib networking internals.
- Low-level capabilities missing from the public API must first be exposed
  through a safe abstraction in `stdlib`.

The crate may use safe public abstractions such as `TcpListener`, `TcpStream`,
`Read`, `Write`, slices, `Vec`, `String`, `Result`, `Option`, `Arc`, `Mutex`,
and `spawn`.

Verification must audit all source under `rock_http/` for `unsafe`, raw-pointer
types, intrinsic calls, libc imports, and private descriptor access.

## Current Foundation

- Native ranges support borrowed views such as `&buffer[..count]`.
- `TcpListener` and `TcpStream` provide blocking IPv4 TCP networking with RAII
  descriptor cleanup.
- `Read` and `Write` provide portable stream abstractions, but their legacy
  prefix APIs must be replaced by native slice operations.
- Owned threads, detached `JoinHandle` cleanup, `Arc`, and `Mutex` support a
  thread-per-connection server.
- `String::from_bytes`, `String::as_str`, and `Vec` provide basic owned data,
  but safe byte-view and parsing conveniences remain limited.
- `test_project/crates/net_lib/src/http.rk` is a data-model mock, not a server
  implementation, and is not an implementation base.

## Public Data Model

```rock
enum Method
    Get
    Head
    Post
    Put
    Delete
    Options
    Patch
    Other String

enum Version
    Http10
    Http11

struct Header
    name: String
    value: String

struct Request
    method: Method
    target: String
    version: Version
    headers: Vec Header
    body: Vec U8

enum Body
    Empty
    Text String
    Bytes (Vec U8)

struct Response
    status: I64
    headers: Vec Header
    body: Body

struct ServerConfig
    max_request_line_bytes: I64
    max_header_bytes: I64
    max_headers: I64
    max_body_bytes: I64

struct Server
    listener: TcpListener
    config: ServerConfig
```

Headers are an ordered `Vec Header`, not a map. HTTP permits repeated fields,
field-name comparison is case-insensitive, and wire order can matter in
practice. `Request::header` returns the first matching value while an
all-values operation preserves duplicates.

## Public Operations

The initial application-facing operations are:

```rock
Server::bind addr
server.serve handler

request.header "content-type"
request.headers_named "set-cookie"

Response::new 200
Response::text 200, "Hello"
Response::bytes 200, bytes
response.with_header "content-type", "text/plain; charset=utf-8"
```

`Server::serve` accepts one shared callable from `Request` to `Response`.
Applications that require mutable shared state capture an `Arc (Mutex T)`.
The handler must satisfy the callable and thread-safety bounds needed to share
it with connection workers.

## Protocol Scope

The initial implementation supports:

- HTTP/1.0 and HTTP/1.1 request lines.
- Fixed-length request bodies framed by `Content-Length`.
- Request lines, headers, and bodies fragmented across arbitrary TCP reads.
- Ordered duplicate headers and case-insensitive field-name lookup.
- Text and binary response bodies.
- One request and one response per connection.
- Thread-per-connection execution.
- Explicit request-line, header, header-count, and body limits.

The initial implementation does not support:

- Chunked request or response bodies.
- Keep-alive or request pipelining.
- TLS or compression.
- Multipart parsing.
- WebSocket upgrades.
- Async execution.
- HTTP/2 or HTTP/3.

Unsupported transfer coding is rejected explicitly rather than interpreted as
an empty body.

## Request Decoding

The decoder incrementally accumulates bytes until it finds `\r\n\r\n`. It
then parses the request line and ordered headers, determines the expected body
length, and waits until that body is complete.

The decoder reports one of three states:

- `Incomplete` when another read may complete a valid request.
- `Complete` with an owned request and the number of consumed bytes.
- `Error` with a typed parse or limit error.

Required validation includes:

- Exactly three request-line components.
- A recognized HTTP version.
- Valid method and target bytes for the initial ASCII-oriented API.
- A colon in every non-empty header line.
- Valid field-name bytes and trimmed field values.
- Decimal, non-negative `Content-Length` without overflow.
- Duplicate `Content-Length` values must agree.
- `Transfer-Encoding` is rejected in the first release.
- Configured request-line, header-byte, header-count, and body limits.

## Response Encoding

Responses use a valid HTTP/1.1 status line followed by CRLF-delimited headers
and an optional body.

- Add `Content-Length` when the application did not supply it.
- Add `Connection: close` when the application did not supply it.
- Preserve application header order and duplicates.
- Suppress body bytes for `HEAD` and status codes that forbid bodies.
- Validate status codes before writing.
- Use `Write::write_all`; never assume one write completes the output.

## Connection And Server Behavior

Each accepted connection processes one request, writes one response, and
closes. Parse errors become bounded HTTP error responses such as `400`, `413`,
`431`, or `501` where writing a response is still possible.

The listener distinguishes fatal accept failures from connection-local
failures. A malformed request, disconnected client, handler failure boundary,
or response-write failure must terminate only that worker.

The first concurrency model is one detached worker per accepted connection.
Later hardening adds timeouts and a connection cap before the library is
described as production-ready.

## Implementation Sequence

### Phase 1: Safe I/O Foundation

- Keep `Write::write` as the backend primitive.
- Centralize correct partial-write handling in the safe generic `write_all`
  operation and expose it through each `Write` implementation.
- Centralize `write_str` through a safe byte view and the same write-all loop.
- Remove `write_all_prefix`, `write_prefix`, `send_all_prefix`, and
  `send_prefix` APIs.
- Migrate `File`, `Stdout`, `TcpStream`, `Arc W`, `io::copy`, examples, and
  tests to native slices.
- Add safe string/byte view operations required by the HTTP parser and writer.

### Phase 2: Package And Protocol Types

- Add `rock_http/rock.toml` and `rock_http/src/lib.rk`.
- Separate request, response, error, parser, and server modules.
- Export only the intended application-facing API.
- Add constructors and ordered, case-insensitive header lookup.

### Phase 3: Bounded Request Decoder

- Implement incremental framing and parsing.
- Enforce all configured limits before unbounded allocation.
- Support fixed-length bodies and typed incomplete/error states.
- Cover every meaningful TCP fragmentation boundary in deterministic tests.

### Phase 4: Response Writer

- Implement status-line, header, framing, and body serialization.
- Generate required connection and length headers.
- Add exact wire-byte tests, including `HEAD` and no-body statuses.

### Phase 5: Connection And Concurrent Server

- Connect decoding, handler invocation, and response writing.
- Map parse failures to protocol responses.
- Add thread-per-connection serving with shared safe handlers.
- Prove one connection failure does not terminate the listener.

### Phase 6: Hardening

- Add safe socket read and write timeout APIs to `stdlib`.
- Add configurable maximum concurrent connections.
- Add safe `SO_REUSEADDR` listener configuration.
- Add structured logging hooks.
- Consider keep-alive only after buffered leftovers and idle timeouts are
  reliable.

## Verification

The implementation requires:

- Focused stdlib tests for partial writes, zero-byte writes, native subslices,
  strings, files, stdout, TCP streams, and `Arc W` forwarding.
- Pure decoder tests covering each fragmentation boundary.
- Malformed request-line and malformed header tests.
- Request-line, header-byte, header-count, and body-size limit tests.
- Duplicate-header and case-insensitive lookup tests.
- Conflicting `Content-Length` and transfer-encoding rejection tests.
- Exact response wire-format tests.
- Real localhost GET, POST, binary-body, `HEAD`, malformed-request,
  partial-read, and concurrent-client tests.
- A path-dependency build proving an application can import `rock_http`.
- Existing stdlib TCP, I/O, slicing, artifact, and threading regressions.
- A source audit proving `rock_http/` contains no unsafe surface.

The smallest useful release is phases 1 through 5. Compiler changes are not
expected; a missing low-level capability should be added as a safe stdlib API
rather than bypassed inside `rock_http`.
