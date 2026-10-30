//! Minimal one-shot HTTP/1.1 server for adapter tests. It records the raw
//! request and writes a close-delimited response in timed chunks.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct FakeResponse {
    pub status: &'static str,
    pub content_type: &'static str,
    pub chunks: Vec<Vec<u8>>,
    pub chunk_delay: Duration,
    /// Keep the connection open after the last chunk (stall/cancel tests).
    pub hold_open: Option<Duration>,
}

impl FakeResponse {
    pub fn sse(chunks: &[&str]) -> Self {
        Self {
            status: "200 OK",
            content_type: "text/event-stream",
            chunks: chunks
                .iter()
                .map(|chunk| chunk.as_bytes().to_vec())
                .collect(),
            chunk_delay: Duration::from_millis(15),
            hold_open: None,
        }
    }

    pub fn bytes(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            chunks,
            ..Self::sse(&[])
        }
    }
}

/// Returns the base URL (`http://127.0.0.1:port`) and a handle yielding the
/// captured request text.
pub async fn serve_once(response: FakeResponse) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_request(&mut socket).await;
        let head = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
            response.status, response.content_type
        );
        let _ = socket.write_all(head.as_bytes()).await;
        for chunk in &response.chunks {
            if socket.write_all(chunk).await.is_err() {
                return request;
            }
            let _ = socket.flush().await;
            tokio::time::sleep(response.chunk_delay).await;
        }
        if let Some(hold) = response.hold_open {
            tokio::time::sleep(hold).await;
        }
        let _ = socket.shutdown().await;
        request
    });
    (format!("http://{}", address), handle)
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = socket.read(&mut chunk).await.unwrap_or(0);
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = find(&buffer, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buffer[..end]).to_ascii_lowercase();
            let length = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buffer.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buffer).into_owned()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Loopback tests must not be routed through a system proxy.
pub fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}
