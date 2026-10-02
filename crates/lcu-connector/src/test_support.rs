//! Serveur HTTP local réservé aux tests : les requêtes réelles du connecteur sont vérifiées.

use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::{Credentials, LcuClient};

pub(crate) struct ExpectedRequest {
    pub method: &'static str,
    pub path: String,
    pub body: Option<Value>,
    pub status: u16,
    pub response: Value,
}

pub(crate) async fn mock_client(expected: Vec<ExpectedRequest>) -> (LcuClient, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let credentials =
        Credentials::from_lockfile(&format!("LeagueClient:1:{port}:test:http")).unwrap();
    let client = LcuClient::new(&credentials).unwrap();
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            for expected in expected {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let header_end = loop {
                    let mut chunk = [0; 4096];
                    let read = stream.read(&mut chunk).await.unwrap();
                    assert!(read > 0, "requête HTTP interrompue");
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                        break end + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let first_line = headers.lines().next().unwrap();
                assert_eq!(first_line, format!("{} {} HTTP/1.1", expected.method, expected.path));
                let content_length = headers.lines().find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().unwrap())
                }).unwrap_or(0);
                while request.len() < header_end + content_length {
                    let mut chunk = [0; 4096];
                    let read = stream.read(&mut chunk).await.unwrap();
                    assert!(read > 0, "corps HTTP interrompu");
                    request.extend_from_slice(&chunk[..read]);
                }
                let body = if content_length == 0 { None } else {
                    Some(serde_json::from_slice::<Value>(&request[header_end..header_end + content_length]).unwrap())
                };
                assert_eq!(body, expected.body);
                let response_body = if expected.status == 204 { String::new() } else { expected.response.to_string() };
                let response = format!(
                    "HTTP/1.1 {} Test\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    expected.status, response_body.len(), response_body
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        }).await.expect("le connecteur n'a pas envoyé les requêtes attendues");
    });
    (client, server)
}
