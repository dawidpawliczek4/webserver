# HTTP Webserver

A minimal HTTP/1.1 server serving static files from per-domain directories, written in Rust.

University project for **Computer Networks** course.

## How it works

The server listens on a TCP port and serves files from `<directory>/<Host>/<path>`. Each incoming request is parsed for the request line, `Host` and `Connection` headers. Connections are kept alive with a 1-second idle timeout and closed early when the client sends `Connection: close`.

**Features:**
- response codes: `200 OK`, `301 Moved Permanently`, `403 Forbidden`, `404 Not Found`, `501 Not Implemented`
- directory traversal protection via `fs::canonicalize` + `starts_with` check on the domain root
- directories redirect to `index.html` with a `Location` header
- `Content-Type` derived from extension (`html`, `txt`, `css`, `jpg`/`jpeg`, `png`, `pdf`, else `application/octet-stream`)
- persistent connections — multiple requests per TCP connection
- resilient to malformed input (returns 501 and keeps connection alive)

## Build & run

```bash
make
./webserver <port> <directory>
```

Example:
```bash
./webserver 8888 p4-webpages
```

Then open `http://virbian:8888/` in a browser (requires `/etc/hosts` entries for `virbian` and `virtual-domain.example.com` pointing to `127.0.0.1`).

## Structure

| File | Contents |
|------|----------|
| `src/main.rs` | argument parsing, accept loop, request parsing, response building |
| `Cargo.toml` | crate metadata, edition 2024 |
| `Makefile` | wraps `cargo build --release`; provides `clean` and `distclean` |
