//! `--healthcheck` probe for distroless images (no shell, no curl).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Exits successfully only when `GET /health` on the local port returns 200.
pub fn probe(bind: &str) -> Result<(), std::io::Error> {
    let port = bind.rsplit(':').next().unwrap_or("8000");
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n")?;
    let mut head = [0u8; 12];
    stream.read_exact(&mut head)?;
    match head.ends_with(b"200") {
        true => Ok(()),
        false => Err(std::io::Error::other("health endpoint not OK")),
    }
}
