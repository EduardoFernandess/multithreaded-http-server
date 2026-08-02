mod http;
mod server;
mod thread_pool;

use server::{default_handler, Handler, Server};
use std::env;
use std::sync::Arc;

fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7878".to_string());
    let workers = env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let handler: Handler = Arc::new(default_handler);
    let server = Server::bind(&addr, workers, handler).expect("failed to bind");
    println!(
        "listening on {} with {workers} workers",
        server.local_addr().unwrap()
    );
    server.run().expect("server error");
}
