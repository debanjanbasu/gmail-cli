//! Minimal local HTTP server to receive the OAuth callback.

use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::Mutex;

use crate::core::error::{GrrError, Result};

impl super::GoogleAuth {
    /// Start local HTTP server to receive OAuth callback
    pub(crate) async fn start_callback_server(&self) -> Result<(String, String)> {
        let (tx, rx) = tokio::sync::oneshot::channel::<(String, String)>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3434")
            .await
            .map_err(GrrError::Io)?;

        tracing::info!("Waiting for OAuth callback on http://localhost:3434/oauth/callback");

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

        let callback = tokio::time::timeout(Duration::from_secs(120), rx).await;
        server_handle.abort();
        let (code, state) = callback
            .map_err(|_| GrrError::Timeout("OAuth callback timeout".into()))?
            .map_err(|_| GrrError::Auth(anyhow!("OAuth callback channel closed").into()))?;

        if code.is_empty() {
            return Err(GrrError::Auth(
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
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_query_param;

    #[test]
    fn callback_query_parameters_are_decoded() {
        let request = "GET /oauth/callback?code=a%2Fb%2Bc&state=state-value HTTP/1.1";
        assert_eq!(parse_query_param(request, "code").as_deref(), Some("a/b+c"));
        assert_eq!(
            parse_query_param(request, "state").as_deref(),
            Some("state-value")
        );
    }
}
