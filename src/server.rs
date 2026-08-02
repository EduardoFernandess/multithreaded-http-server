use crate::http::{Request, Response};
use crate::thread_pool::ThreadPool;
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;

pub type Handler = Arc<dyn Fn(&Request) -> Response + Send + Sync + 'static>;

pub struct Server {
    listener: TcpListener,
    pool: ThreadPool,
    handler: Handler,
}

impl Server {
    pub fn bind<A: ToSocketAddrs>(addr: A, workers: usize, handler: Handler) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        Ok(Self {
            listener,
            pool: ThreadPool::new(workers),
            handler,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub fn run(&self) -> std::io::Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            let handler = Arc::clone(&self.handler);
            self.pool.execute(move || {
                if let Err(err) = handle_connection(stream, &handler) {
                    eprintln!("connection error: {err}");
                }
            });
        }
        Ok(())
    }
}

fn handle_connection(mut stream: TcpStream, handler: &Handler) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let response = match Request::parse(&mut reader) {
        Ok(req) => handler(&req),
        Err(_) => Response::bad_request("malformed request"),
    };
    response.write_to(&mut stream)?;
    stream.flush()?;
    Ok(())
}

pub fn default_handler(req: &Request) -> Response {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => Response::ok("multithreaded-http-server\n"),
        ("GET", "/health") => Response::ok("ok\n"),
        ("GET", path) if path.starts_with("/echo/") => {
            let msg = &path["/echo/".len()..];
            Response::ok(format!("{msg}\n"))
        }
        ("POST", "/echo") => Response::ok(req.body.clone()),
        _ => Response::not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn serves_health_endpoint() {
        let handler: Handler = Arc::new(default_handler);
        let server = Server::bind("127.0.0.1:0", 2, handler).unwrap();
        let addr = server.local_addr().unwrap();

        thread::spawn(move || {
            let _ = server.run();
        });
        thread::sleep(Duration::from_millis(50));

        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();

        let mut buf = String::new();
        stream.read_to_string(&mut buf).unwrap();
        assert!(buf.contains("HTTP/1.1 200 OK"));
        assert!(buf.contains("ok"));
    }
}
