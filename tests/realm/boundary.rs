//! Exercise browser protection and body limits through actual TCP, including chunked bodies.
use crate::support::client::Mcp;
use anyhow::{Result, ensure};
use reqwest::Method;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};

pub async fn run(mcp: &Mcp, access: &str) -> Result<()> {
    for headers in [
        vec![("host", "attacker.example:8765")],
        vec![("origin", "https://attacker.example")],
        vec![("origin", "null")],
        vec![
            ("origin", "http://127.0.0.1:8765"),
            ("origin", "https://attacker.example"),
        ],
    ] {
        mcp.request(
            Some(access),
            Method::POST,
            "tools/list",
            json!({}),
            403,
            &headers,
        )
        .await?;
    }
    mcp.request(
        Some(access),
        Method::POST,
        "tools/list",
        json!({}),
        200,
        &[("origin", "http://127.0.0.1:8765")],
    )
    .await?;
    mcp.call(
        access,
        "tools/list",
        json!({"padding":"x".repeat(16385)}),
        413,
    )
    .await?;
    tokio::task::spawn_blocking(chunked).await??;
    let listeners = std::fs::read_to_string("/proc/net/tcp")?;
    let bound: Vec<_> = listeners
        .lines()
        .skip(1)
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|parts| parts[1].ends_with(":223D") && parts[3] == "0A")
        .map(|parts| parts[1])
        .collect();
    ensure!(
        bound == ["0100007F:223D"],
        "MCP listener is not confined to IPv4 loopback"
    );
    Ok(())
}

fn chunked() -> Result<()> {
    let mut socket = std::net::TcpStream::connect("127.0.0.1:8765")?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let mut request = b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:8765\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
    for _ in 0..2 {
        request.extend_from_slice(b"2000\r\n");
        request.extend_from_slice(&[b'x'; 8192]);
        request.extend_from_slice(b"\r\n");
    }
    // Cross the limit by one byte and send once. Extra writes after the server's
    // valid early 413 raced connection closure and falsely failed with EPIPE.
    request.extend_from_slice(b"1\r\nx\r\n0\r\n\r\n");
    socket.write_all(&request)?;
    let mut response = String::new();
    BufReader::new(socket).read_line(&mut response)?;
    ensure!(response.starts_with("HTTP/1.1 413"));
    Ok(())
}
