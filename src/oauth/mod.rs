// ═══════════════════════════════════════════════════════════════
// OAUTH MODULE - OAuth authentication flow for API providers
// ═══════════════════════════════════════════════════════════════

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// OAuth configuration for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Provider name (e.g., "qwen", "dashscope")
    pub provider: String,
    /// OAuth authorization URL
    pub auth_url: String,
    /// OAuth token URL
    pub token_url: String,
    /// Client ID
    pub client_id: String,
    /// Optional client secret
    pub client_secret: Option<String>,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// Callback URL (typically localhost)
    pub redirect_uri: String,
}

/// OAuth credentials received after successful authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    /// Access token
    pub access_token: String,
    /// Refresh token (if available)
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expires in seconds
    pub expires_in: Option<u64>,
    /// Provider name
    pub provider: String,
}

impl OAuthConfig {
    /// Create OAuth configuration for QWEN/Dashscope
    pub fn qwen() -> Self {
        Self {
            provider: "qwen".to_string(),
            auth_url: "https://dashscope.aliyuncs.com/authorizations".to_string(),
            token_url: "https://dashscope.aliyuncs.com/authorizations/oauth/token".to_string(),
            client_id: "zeroclaw".to_string(),
            client_secret: None,
            scopes: vec!["openid".to_string(), "profile".to_string()],
            redirect_uri: "http://localhost:8080/callback".to_string(),
        }
    }

    /// Generate the OAuth authorization URL
    pub fn get_auth_url(&self) -> Result<String> {
        use std::collections::HashMap;

        let state = Self::generate_state();
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("client_id".to_string(), self.client_id.clone());
        params.insert("redirect_uri".to_string(), self.redirect_uri.clone());
        params.insert("response_type".to_string(), "code".to_string());
        params.insert("scope".to_string(), self.scopes.join(" "));
        params.insert("state".to_string(), state);

        let url = format!("{}?{}", self.auth_url, Self::form_encode(&params));
        Ok(url)
    }

    /// Generate a random state token for CSRF protection
    fn generate_state() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{:x}", timestamp)
    }

    /// URL-encode form parameters
    fn form_encode(params: &HashMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
}

/// OAuth flow handler
pub struct OAuthHandler {
    config: OAuthConfig,
    received_credential: Arc<Mutex<Option<OAuthCredentials>>>,
}

impl OAuthHandler {
    /// Create a new OAuth handler
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config,
            received_credential: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the OAuth flow
    pub async fn authenticate(&self) -> Result<OAuthCredentials> {
        let port = self.extract_port()?;
        let credential = self.start_callback_server(port).await?;
        Ok(credential)
    }

    /// Extract port from redirect URI
    fn extract_port(&self) -> Result<u16> {
        let uri = &self.config.redirect_uri;
        if let Some(port_str) = uri.strip_prefix("http://localhost:").and_then(|s| {
            s.strip_suffix("/callback")
        }) {
            port_str.parse::<u16>().context("Invalid port in redirect_uri")
        } else {
            anyhow::bail!("Invalid redirect_uri format: {}", uri)
        }
    }

    /// Start local HTTP server to catch OAuth callback
    async fn start_callback_server(&self, port: u16) -> Result<OAuthCredentials> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await.context("Failed to bind to local server port")?;

        let url = self.config.get_auth_url()?;
        tracing::info!("Opening browser for OAuth authentication...");
        tracing::info!("Please authenticate at: {}", url);

        // Open browser using opener crate
        if let Err(e) = opener::open(url.as_str()) {
            tracing::warn!("Failed to open browser: {}", e);
            tracing::info!("Please open this URL in your browser: {}", url);
        }

        // Wait for callback
        let credential = tokio::time::timeout(
            tokio::time::Duration::from_secs(300),
            self.handle_callback(listener),
        )
        .await
        .context("OAuth authentication timed out")?
        .context("OAuth callback not received")?;

        Ok(credential)
    }

    /// Handle OAuth callback from browser
    async fn handle_callback(&self, listener: TcpListener) -> Result<OAuthCredentials> {
        let (mut socket, _) = listener.accept().await.context("Failed to accept connection")?;

        let (reader, mut writer) = socket.split();
        let mut buf_reader = BufReader::new(reader);
        let mut request_line = String::new();

        buf_reader.read_line(&mut request_line).await?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() < 2 {
            anyhow::bail!("Invalid HTTP request");
        }

        let path = parts[1];

        // Read headers until empty line
        loop {
            let mut line = String::new();
            buf_reader.read_line(&mut line).await?;
            if line.trim().is_empty() {
                break;
            }
        }

        // Parse query parameters
        let credential = if let Some(query) = path.strip_prefix("/callback?") {
            let params = Self::parse_query_params(query);

            if let Some(code) = params.get("code") {
                Some(OAuthCredentials {
                    access_token: code.clone(),
                    refresh_token: None,
                    token_type: "Bearer".to_string(),
                    expires_in: None,
                    provider: self.config.provider.clone(),
                })
            } else if let Some(error) = params.get("error") {
                anyhow::bail!("OAuth error: {}", error);
            } else {
                anyhow::bail!("No code in OAuth callback");
            }
        } else {
            anyhow::bail!("Invalid callback path: {}", path);
        };

        let success_html = r#"<!DOCTYPE html>
<html>
<head><title>Authentication Successful</title></head>
<body>
    <h1>Authentication Successful!</h1>
    <p>You can close this window and return to ZeroClaw.</p>
    <script>window.close();</script>
</body>
</html>"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            success_html.len(),
            success_html
        );

        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;

        credential.ok_or_else(|| anyhow::anyhow!("Failed to receive OAuth credentials"))
    }

    /// Parse query parameters from URL
    fn parse_query_params(query: &str) -> std::collections::HashMap<String, String> {
        query
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                Some((
                    parts.next()?.to_string(),
                    parts.next()?.to_string(),
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_config_qwen_has_correct_fields() {
        let config = OAuthConfig::qwen();
        assert_eq!(config.provider, "qwen");
        assert!(config.auth_url.contains("dashscope"));
        assert_eq!(config.redirect_uri, "http://localhost:8080/callback");
    }

    #[test]
    fn oauth_state_generate_state_is_unique() {
        let state1 = OAuthConfig::generate_state();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let state2 = OAuthConfig::generate_state();
        assert_ne!(state1, state2);
    }

    #[test]
    fn parse_query_params_handles_valid_input() {
        let query = "code=test_code&state=test_state";
        let params = OAuthHandler::parse_query_params(query);
        assert_eq!(params.get("code"), Some(&"test_code".to_string()));
        assert_eq!(params.get("state"), Some(&"test_state".to_string()));
    }

    #[test]
    fn parse_query_params_handles_empty() {
        let query = "";
        let params = OAuthHandler::parse_query_params(query);
        assert!(params.is_empty());
    }
}
