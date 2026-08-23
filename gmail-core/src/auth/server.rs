//! Minimal local HTTP server to receive the OAuth callback.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::Mutex;

use crate::error::{GmailError, Result};

impl super::GmailAuth {
    /// Start local HTTP server to receive OAuth callback
    pub(crate) async fn start_callback_server(&self) -> Result<(String, String)> {
        let (tx, rx) = tokio::sync::oneshot::channel::<(String, String)>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3434")
            .await
            .map_err(GmailError::Io)?;

        tracing::info!("Waiting for OAuth callback on http://0.0.0.0:3434/oauth/callback");

        // Run server in background
        let server_handle = tokio::spawn(async move {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };

                let tx = tx.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};

                    // Read request headers (up to CRLFCRLF)
                    let mut buf = vec![0u8; 8192];
                    let mut read = 0usize;
                    loop {
                        let n = match stream.read(&mut buf[read..]).await {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        read += n;
                        if buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                        if read >= buf.len() {
                            break;
                        }
                    }

                    let request = String::from_utf8_lossy(&buf[..read]);
                    let code = parse_query_param(&request, "code").unwrap_or_default();
                    let state = parse_query_param(&request, "state").unwrap_or_default();

                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send((code, state));
                    }

                    let body = "<h1>Authentication successful! You can close this window.</h1>";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                });
            }
        });

        // Wait for callback with timeout
        let (code, state) = tokio::time::timeout(Duration::from_secs(120), rx)
            .await
            .map_err(|_| GmailError::Timeout("OAuth callback timeout".into()))?
            .map_err(|_| GmailError::Auth(anyhow!("OAuth callback channel closed").into()))?;

        // Give the server time to send the response before aborting
        tokio::time::sleep(Duration::from_millis(500)).await;
        server_handle.abort();

        if code.is_empty() {
            return Err(GmailError::Auth(
                anyhow!("No authorization code received").into(),
            ));
        }

        Ok((code, state))
    }
}

/// Extract a query parameter value from an HTTP request line.
fn parse_query_param(request: &str, key: &str) -> Option<String> {
    let request_line = request.lines().next()?;
    let path = request_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;
    let params: HashMap<&str, &str> = query
        .split('&')
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            match (it.next(), it.next()) {
                (Some(k), Some(v)) => Some((k, v)),
                _ => None,
            }
        })
        .collect();
    params.get(key).map(|v| v.to_string())
}
