# multithreaded-http-server

Minimal HTTP/1.1 server with no web framework. Connections arrive on a `TcpListener`, are handed to a fixed-size thread pool, and responses are written by hand.

## Stack

- **Rust standard library** — TCP sockets and threading
- Hand-rolled HTTP request parsing and response formatting
- Fixed-size worker thread pool

## What was built

- Bind address and worker-count CLI arguments
- Routes for health and echo endpoints
- Concurrent connection handling without Tokio/Axum
- Unit tests for HTTP parsing, routing, and the thread pool

## Run

```bash
cargo run -- 127.0.0.1:7878 8
```

Arguments: `<bind_addr> <worker_threads>`.

### Try it

```bash
curl http://127.0.0.1:7878/
curl http://127.0.0.1:7878/health
curl http://127.0.0.1:7878/echo/hello
curl -X POST -d 'payload' http://127.0.0.1:7878/echo
```

### Routes

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/` | Basic landing response |
| GET | `/health` | Liveness check |
| GET | `/echo/:msg` | Echo path segment |
| POST | `/echo` | Echo request body |

### Tests

```bash
cargo test
```
