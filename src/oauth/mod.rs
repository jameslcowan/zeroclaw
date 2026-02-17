// ═══════════════════════════════════════════════════════════════
// OAUTH MODULE - OAuth authentication flow for API providers
// ═══════════════════════════════════════════════════════════════

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

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

/// OAuth state for tracking the flow
#[derive(Debug, Clone)]
pub struct OAuthState {
    /// CSRF/state token
    pub state: String,
    /// Provider name
    pub provider: String,
    /// Port for local callback server
    pub port: u16,
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
    pub fn auth_url(&self) -> Result<String> {
        use std::collections::HashMap;

        let state = Self::generate_state();
        let scope = self.scopes.join(" ");
        let mut params: HashMap<&str, &str> = HashMap::new();
        params.insert("client_id", &self.client_id);
        params.insert("redirect_uri", &self.redirect_uri);
        params.insert("response_type", "code");
        params.insert("scope", &scope);
        params.insert("state", &state);

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
    fn form_encode(params: &std::collections::HashMap<&str, &str>) -> String {
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
    /// 1. Start local HTTP server for callback
    /// 2. Generate auth URL
    /// 3. Open browser for user to authenticate
    /// 4. Wait for callback
    /// 5. Return credentials
    pub async fn authenticate(&self) -> Result<OAuthCredentials> {
        let port = self.extract_port()?;

        // Start local HTTP server for callback
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
        use axum::{
            extract::Query,
            response::Html,
            routing::get,
            Router,
        };
        use serde::Deserialize;
        use std::collections::HashMap;
        use std::net::SocketAddr;

        let received_credential = Arc::clone(&self.received_credential);

        // Callback handler
        let cred_for_handler = Arc::clone(&received_credential);

        #[derive(Deserialize)]
        struct CallbackParams {
            code: Option<String>,
            #[serde(flatten)]
            other: HashMap<String, String>,
        }

        let app = Router::new().route("/callback", get(move |Query(params): Query<CallbackParams>| {
            let cred = Arc::clone(&cred_for_handler);
            async move {
                if let Some(code) = params.code {
                    // TODO: Exchange code for access token via token_url
                    let credential = OAuthCredentials {
                        access_token: code.clone(),
                        refresh_token: None,
                        token_type: "Bearer".to_string(),
                        expires_in: None,
                        provider: "qwen".to_string(),
                    };

                    *cred.lock().await = Some(credential);

                    Html(
                        format!(
                            r#"
<!DOCTYPE html>
<html>
<head><title>Authentication Successful</title></head>
<body>
    <h1>Authentication Successful!</h1>
    <p>You can close this window and return to ZeroClaw.</p>
    <script>window.close();</script>
</body>
</html>
"#
                        )
                    )
                } else {
                    let error = params.other.get("error").map(|s| s.as_str()).unwrap_or("Unknown error");
                    Html(format!(
                        r#"
<!DOCTYPE html>
<html>
<head><title>Authentication Failed</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>Error: {}</p>
    <p>You can close this window and try again.</p>
</body>
</html>
"#,
                        error
                    ))
                }
            }
        }));

        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let url = self.config.auth_url()?;
        tracing::info!("Opening browser for OAuth authentication...");
        tracing::info!("Please authenticate at: {}", url);

        // Open browser
        if let Err(e) = opener::open(&url) {
            tracing::warn!("Failed to open browser: {}", e);
            tracing::info!("Please open this URL in your browser: {}", url);
        }

        // Run the server and wait for callback
        let (server_task, shutdown) = self.run_server(app, addr);

        // Wait for credential or timeout
        let credential = tokio::time::timeout(
            tokio::time::Duration::from_secs(300), // 5 minute timeout
            Self::wait_for_credential(Arc::clone(&self.received_credential)),
        )
        .await
        .context("OAuth authentication timed out")?
        .context("OAuth callback not received")?;

        // Shutdown server
        let _ = shutdown.send(());
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), server_task).await;

        Ok(credential)
    }

    /// Run the axum server with graceful shutdown
    fn run_server(
        &self,
        app: axum::Router,
        addr: std::net::SocketAddr,
    ) -> (tokio::task::JoinHandle<()>, tokio::sync::oneshot::Sender<()>) {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let task = tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!("OAuth callback server failed to bind: {}", e);
                    return;
                }
            };

            let server = axum::serve(listener, app);

            // Graceful shutdown
            let graceful = server.with_graceful_shutdown(async {
                match rx.await {
                    Ok(()) => {},
                    Err(_) => {},
                }
            });

            if let Err(e) = graceful.await {
                tracing::warn!("OAuth callback server error: {}", e);
            }
        });

        (task, tx)
    }

    /// Wait for credential to be received
    async fn wait_for_credential(
        credential: Arc<Mutex<Option<OAuthCredentials>>>,
    ) -> Result<OAuthCredentials> {
        loop {
            {
                let cred = credential.lock().await;
                if let Some(c) = cred.as_ref() {
                    return Ok(c.clone());
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
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
