use crate::core::oauth_server::OAuthCallbackServer;
use crate::utils::error::{Result, UvcadError};
use crate::utils::keyring::{OAuthTokens, TokenManager};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use oauth2::reqwest::async_http_client;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AuthManager {
    token_manager: TokenManager,
    oauth_client: Option<BasicClient>,
    pkce_verifier: Arc<Mutex<Option<String>>>,
    csrf_token: Arc<Mutex<Option<String>>>,
}

impl AuthManager {
    pub fn new() -> Result<Self> {
        let token_manager = TokenManager::new("google_drive")?;

        Ok(Self {
            token_manager,
            oauth_client: None,
            pkce_verifier: Arc::new(Mutex::new(None)),
            csrf_token: Arc::new(Mutex::new(None)),
        })
    }

    pub fn initialize_oauth(
        &mut self,
        client_id: String,
        client_secret: String,
    ) -> Result<()> {
        // Google OAuth endpoints
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| UvcadError::OAuthError(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| UvcadError::OAuthError(format!("Invalid token URL: {}", e)))?;

        let redirect_url = RedirectUrl::new("http://127.0.0.1:8080/oauth/callback".to_string())
            .map_err(|e| UvcadError::OAuthError(format!("Invalid redirect URL: {}", e)))?;

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        self.oauth_client = Some(client);
        Ok(())
    }

    pub async fn start_auth_flow(&self) -> Result<String> {
        let client = self.oauth_client.as_ref()
            .ok_or_else(|| UvcadError::OAuthError("OAuth client not initialized".to_string()))?;

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/drive".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Store PKCE verifier and CSRF token for later verification
        *self.pkce_verifier.lock().await = Some(pkce_verifier.secret().clone());
        *self.csrf_token.lock().await = Some(csrf_token.secret().clone());

        Ok(auth_url.to_string())
    }

    pub async fn complete_auth_flow(&self) -> Result<OAuthTokens> {
        // Start the callback server
        let server = OAuthCallbackServer::new(8080);

        tracing::info!("Waiting for OAuth callback...");
        let callback = server.wait_for_callback().await?;

        // Verify CSRF token
        let expected_csrf = self.csrf_token.lock().await.clone()
            .ok_or_else(|| UvcadError::OAuthError("No CSRF token found".to_string()))?;

        if callback.state != expected_csrf {
            return Err(UvcadError::OAuthError("CSRF token mismatch".to_string()));
        }

        // Exchange authorization code for tokens
        let pkce_verifier = self.pkce_verifier.lock().await.clone()
            .ok_or_else(|| UvcadError::OAuthError("No PKCE verifier found".to_string()))?;

        self.exchange_code(callback.code, pkce_verifier).await
    }

    async fn exchange_code(
        &self,
        code: String,
        pkce_verifier: String,
    ) -> Result<OAuthTokens> {
        let client = self.oauth_client.as_ref()
            .ok_or_else(|| UvcadError::OAuthError("OAuth client not initialized".to_string()))?;

        let token_result = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(async_http_client)
            .await
            .map_err(|e| UvcadError::OAuthError(format!("Token exchange failed: {}", e)))?;

        let tokens = OAuthTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_at: token_result.expires_in().map(|d| {
                (chrono::Utc::now() + chrono::Duration::seconds(d.as_secs() as i64)).timestamp()
            }),
        };

        // Store tokens securely
        self.token_manager.store_tokens(&tokens)?;

        tracing::info!("OAuth tokens obtained and stored successfully");
        Ok(tokens)
    }

    pub async fn get_valid_token(&self) -> Result<String> {
        let mut tokens = self.token_manager.get_tokens()?;

        // Check if token is expired
        if let Some(expires_at) = tokens.expires_at {
            let now = chrono::Utc::now().timestamp();
            // Refresh if token expires in less than 5 minutes
            if expires_at - now < 300 {
                tracing::info!("Access token expired or expiring soon, refreshing...");
                tokens = self.refresh_token(&tokens).await?;
            }
        }

        Ok(tokens.access_token)
    }

    async fn refresh_token(&self, tokens: &OAuthTokens) -> Result<OAuthTokens> {
        let client = self.oauth_client.as_ref()
            .ok_or_else(|| UvcadError::OAuthError("OAuth client not initialized".to_string()))?;

        let refresh_token = tokens.refresh_token.as_ref()
            .ok_or_else(|| UvcadError::OAuthError("No refresh token available".to_string()))?;

        let token_result = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
            .request_async(async_http_client)
            .await
            .map_err(|e| UvcadError::OAuthError(format!("Token refresh failed: {}", e)))?;

        let new_tokens = OAuthTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token()
                .map(|t| t.secret().clone())
                .or_else(|| Some(refresh_token.clone())), // Keep old refresh token if not provided
            expires_at: token_result.expires_in().map(|d| {
                (chrono::Utc::now() + chrono::Duration::seconds(d.as_secs() as i64)).timestamp()
            }),
        };

        // Store updated tokens
        self.token_manager.store_tokens(&new_tokens)?;

        tracing::info!("Access token refreshed successfully");
        Ok(new_tokens)
    }

    pub fn is_authenticated(&self) -> bool {
        self.token_manager.has_tokens()
    }

    pub fn logout(&self) -> Result<()> {
        self.token_manager.delete_tokens()
    }
}
