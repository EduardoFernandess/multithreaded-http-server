use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("incomplete request")]
    Incomplete,
    #[error("malformed request line")]
    BadRequestLine,
    #[error("unsupported HTTP version: {0}")]
    UnsupportedVersion(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn parse<R: Read>(reader: &mut BufReader<R>) -> Result<Self, HttpError> {
        let mut request_line = String::new();
        let bytes = reader.read_line(&mut request_line)?;
        if bytes == 0 {
            return Err(HttpError::Incomplete);
        }

        let mut parts = request_line.trim_end().split_whitespace();
        let method = parts.next().ok_or(HttpError::BadRequestLine)?.to_string();
        let path = parts.next().ok_or(HttpError::BadRequestLine)?.to_string();
        let version = parts.next().ok_or(HttpError::BadRequestLine)?.to_string();

        if !version.starts_with("HTTP/") {
            return Err(HttpError::UnsupportedVersion(version));
        }

        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                return Err(HttpError::Incomplete);
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').ok_or(HttpError::BadRequestLine)?;
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }

        let content_length = headers
            .get("content-length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body)?;
        }

        Ok(Self {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, reason: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        let mut headers = HashMap::new();
        headers.insert("Content-Length".into(), body.len().to_string());
        headers.insert("Connection".into(), "close".into());
        Self {
            status,
            reason: reason.into(),
            headers,
            body,
        }
    }

    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        let mut resp = Self::new(200, "OK", body);
        resp.headers
            .insert("Content-Type".into(), "text/plain; charset=utf-8".into());
        resp
    }

    pub fn not_found() -> Self {
        let mut resp = Self::new(404, "Not Found", b"Not Found".to_vec());
        resp.headers
            .insert("Content-Type".into(), "text/plain; charset=utf-8".into());
        resp
    }

    pub fn bad_request(msg: &str) -> Self {
        let mut resp = Self::new(400, "Bad Request", msg.as_bytes().to_vec());
        resp.headers
            .insert("Content-Type".into(), "text/plain; charset=utf-8".into());
        resp
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "HTTP/1.1 {} {}\r\n", self.status, self.reason)?;
        for (k, v) in &self.headers {
            write!(writer, "{k}: {v}\r\n")?;
        }
        write!(writer, "\r\n")?;
        writer.write_all(&self.body)?;
        Ok(())
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status, self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_get_request() {
        let raw = b"GET /hello HTTP/1.1\r\nHost: localhost\r\nUser-Agent: test\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(&raw[..]));
        let req = Request::parse(&mut reader).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/hello");
        assert_eq!(req.headers.get("host").unwrap(), "localhost");
        assert!(req.body.is_empty());
    }

    #[test]
    fn parses_post_with_body() {
        let raw = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nhello";
        let mut reader = BufReader::new(Cursor::new(&raw[..]));
        let req = Request::parse(&mut reader).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body, b"hello");
    }

    #[test]
    fn writes_response() {
        let resp = Response::ok("hi");
        let mut buf = Vec::new();
        resp.write_to(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(s.contains("Content-Length: 2"));
        assert!(s.ends_with("hi"));
    }
}
