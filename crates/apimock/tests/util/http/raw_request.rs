//! Send a request-line byte-for-byte, bypassing any client-side URL
//! normalisation.
//!
//! # Why this exists alongside `TestRequest`
//!
//! `TestRequest` builds requests through `reqwest::Url::parse`, which
//! resolves `..` segments out of the path before the request is ever
//! sent — the same normalisation most browsers, proxies and HTTP
//! libraries apply. A raw `..` reaching the server at all requires a
//! client that skips that step, the way `curl --path-as-is` does. This
//! writes the request line directly to the socket so the path arrives
//! exactly as given.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Send `GET <raw_path> HTTP/1.1` verbatim and return the numeric
/// status code from the response's status line.
pub async fn raw_get_status(host: &str, port: u16, raw_path: &str) -> u16 {
    let mut stream = TcpStream::connect((host, port))
        .await
        .expect("failed to connect for raw request");

    let request =
        format!("GET {raw_path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("failed to write raw request");

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .expect("failed to read raw response");

    let response = String::from_utf8_lossy(&buf);
    let status_line = response
        .lines()
        .next()
        .unwrap_or_else(|| panic!("empty response to raw request {raw_path}"));

    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("couldn't parse a status code from: {status_line}"))
}
