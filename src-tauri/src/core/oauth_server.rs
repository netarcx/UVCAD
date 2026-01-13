use crate::utils::error::{Result, UvcadError};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

pub struct OAuthCallbackServer {
    port: u16,
}

impl OAuthCallbackServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn wait_for_callback(&self) -> Result<OAuthCallback> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| UvcadError::OAuthError(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("OAuth callback server listening on {}", addr);

        let (tx, rx) = oneshot::channel();
        let tx = Arc::new(Mutex::new(Some(tx)));

        // Accept one connection
        if let Ok((mut socket, _)) = listener.accept().await {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut request_line = String::new();

            // Read the first line of the HTTP request
            if reader.read_line(&mut request_line).await.is_ok() {
                tracing::info!("Received OAuth callback request: {}", request_line.trim());

                // Parse the request line (e.g., "GET /oauth/callback?code=...&state=... HTTP/1.1")
                if let Some(callback) = Self::parse_callback(&request_line) {
                    // Send success response
                    let response = "HTTP/1.1 200 OK\r\n\
                                   Content-Type: text/html\r\n\
                                   Connection: close\r\n\
                                   \r\n\
                                   <html><body>\
                                   <h1>Authentication Successful!</h1>\
                                   <p>You can close this window and return to UVCAD.</p>\
                                   <script>window.close();</script>\
                                   </body></html>";

                    let _ = writer.write_all(response.as_bytes()).await;

                    // Send the callback data
                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(callback);
                    }
                } else {
                    // Send error response
                    let response = "HTTP/1.1 400 Bad Request\r\n\
                                   Content-Type: text/html\r\n\
                                   Connection: close\r\n\
                                   \r\n\
                                   <html><body>\
                                   <h1>Authentication Failed</h1>\
                                   <p>Invalid callback parameters.</p>\
                                   </body></html>";

                    let _ = writer.write_all(response.as_bytes()).await;
                }
            }
        }

        // Wait for the callback data
        rx.await.map_err(|_| UvcadError::OAuthError("OAuth callback timeout".to_string()))
    }

    fn parse_callback(request_line: &str) -> Option<OAuthCallback> {
        // Parse: GET /oauth/callback?code=...&state=... HTTP/1.1
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let path = parts[1];
        if !path.starts_with("/oauth/callback") {
            return None;
        }

        // Extract query parameters
        let query = path.split('?').nth(1)?;
        let params: std::collections::HashMap<&str, &str> = query
            .split('&')
            .filter_map(|pair| {
                let mut split = pair.split('=');
                Some((split.next()?, split.next()?))
            })
            .collect();

        let code = params.get("code")?.to_string();
        let state = params.get("state")?.to_string();

        Some(OAuthCallback { code, state })
    }
}
