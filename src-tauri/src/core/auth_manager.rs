use crate::core::credentials;
use crate::core::oauth_server::OAuthCallbackServer;
use crate::utils::error::{Result, UvcadError};
use crate::utils::keyring::{CredentialManager, OAuthCredentials, OAuthTokens, TokenManager};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use oauth2::reqwest::async_http_client;

pub struct AuthManager {
    token_manager: TokenManager,
    credential_manager: CredentialManager,
    oauth_client: Option<BasicClient>,
}

impl AuthManager {
    pub fn new() -> Result<Self> {
        let token_manager = TokenManager::new("google_drive")?;
        let credential_manager = CredentialManager::new("google_drive")?;

        Ok(Self {
            token_manager,
            credential_manager,
            oauth_client: None,
        })
    }

    /// Build a BasicClient from client_id and client_secret.
    fn build_oauth_client(client_id: &str, client_secret: &str) -> Result<BasicClient> {
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| UvcadError::OAuthError(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| UvcadError::OAuthError(format!("Invalid token URL: {}", e)))?;

        let redirect_url = RedirectUrl::new("http://127.0.0.1:8080/oauth/callback".to_string())
            .map_err(|e| UvcadError::OAuthError(format!("Invalid redirect URL: {}", e)))?;

        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        Ok(client)
    }

    /// Ensure oauth_client is initialized. Loads credentials from:
    /// 1. Already-initialized client (no-op)
    /// 2. Stored credentials in keyring
    /// 3. Compile-time embedded defaults
    fn ensure_oauth_client(&mut self) -> Result<()> {
        if self.oauth_client.is_some() {
            return Ok(());
        }

        let (client_id, client_secret) = if let Ok(creds) = self.credential_manager.get_credentials() {
            (creds.client_id, creds.client_secret)
        } else {
            (
                credentials::default_client_id().to_string(),
                credentials::default_client_secret().to_string(),
            )
        };

        self.oauth_client = Some(Self::build_oauth_client(&client_id, &client_secret)?);
        Ok(())
    }

    /// Complete OAuth flow in a single call:
    /// 1. Build OAuth client from embedded credentials
    /// 2. Generate PKCE challenge + auth URL
    /// 3. Start callback server BEFORE opening browser (fixes race condition)
    /// 4. Open browser
    /// 5. Wait for callback (5 min timeout)
    /// 6. Verify CSRF, exchange code for tokens
    /// 7. Store tokens + credentials in keyring
    pub async fn authenticate(&mut self) -> Result<OAuthTokens> {
        let client_id = credentials::default_client_id().to_string();
        let client_secret = credentials::default_client_secret().to_string();

        let client = Self::build_oauth_client(&client_id, &client_secret)?;

        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate auth URL
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/drive".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Start callback server BEFORE opening browser (eliminates race condition)
        let server = OAuthCallbackServer::new(8080);

        // Open browser
        if let Err(e) = open::that(auth_url.as_str()) {
            tracing::warn!("Failed to open browser: {}", e);
            return Err(UvcadError::OAuthError(format!(
                "Failed to open browser. Please open this URL manually:\n{}",
                auth_url
            )));
        }

        tracing::info!("Browser opened for OAuth, waiting for callback...");

        // Wait for callback (5 min timeout is in OAuthCallbackServer)
        let callback = server.wait_for_callback().await?;

        // Verify CSRF token
        if callback.state != *csrf_token.secret() {
            return Err(UvcadError::OAuthError("CSRF token mismatch".to_string()));
        }

        // Exchange authorization code for tokens
        let token_result = client
            .exchange_code(AuthorizationCode::new(callback.code))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.secret().clone()))
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

        // Store tokens in keyring
        self.token_manager.store_tokens(&tokens)?;

        // Store credentials in keyring for future token refresh
        self.credential_manager.store_credentials(&OAuthCredentials {
            client_id,
            client_secret,
        })?;

        // Cache the client for immediate use
        self.oauth_client = Some(client);

        tracing::info!("OAuth tokens obtained and stored successfully");
        Ok(tokens)
    }

    /// Get a valid access token, refreshing if expired.
    pub async fn get_valid_token(&mut self) -> Result<String> {
        let tokens = self.token_manager.get_tokens()?;

        // Check if token is expired or expiring within 5 minutes
        if let Some(expires_at) = tokens.expires_at {
            let now = chrono::Utc::now().timestamp();
            if expires_at - now < 300 {
                tracing::info!("Access token expired or expiring soon, refreshing...");
                let new_tokens = self.refresh_token(&tokens).await?;
                return Ok(new_tokens.access_token);
            }
        }

        Ok(tokens.access_token)
    }

    /// Refresh an expired token using stored credentials.
    async fn refresh_token(&mut self, tokens: &OAuthTokens) -> Result<OAuthTokens> {
        self.ensure_oauth_client()?;

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
                .or_else(|| Some(refresh_token.clone())),
            expires_at: token_result.expires_in().map(|d| {
                (chrono::Utc::now() + chrono::Duration::seconds(d.as_secs() as i64)).timestamp()
            }),
        };

        self.token_manager.store_tokens(&new_tokens)?;
        tracing::info!("Access token refreshed successfully");
        Ok(new_tokens)
    }

    pub fn is_authenticated(&self) -> bool {
        self.token_manager.has_tokens()
    }

    pub fn logout(&self) -> Result<()> {
        self.token_manager.delete_tokens()?;
        let _ = self.credential_manager.delete_credentials();
        Ok(())
    }
}
