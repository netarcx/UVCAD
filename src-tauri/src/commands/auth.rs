use crate::core::auth_manager::AuthManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub provider: String,
    pub email: Option<String>,
}

// Global auth manager instance
static AUTH_MANAGER: Lazy<Arc<Mutex<Option<AuthManager>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

#[tauri::command]
pub async fn start_google_auth(client_id: String, client_secret: String) -> Result<String, String> {
    tracing::info!("Starting Google OAuth flow...");

    let mut manager = AuthManager::new().map_err(|e| e.to_string())?;

    // Initialize OAuth client
    manager.initialize_oauth(client_id, client_secret).map_err(|e| e.to_string())?;

    // Generate auth URL
    let auth_url = manager.start_auth_flow().await.map_err(|e| e.to_string())?;

    // Store manager BEFORE opening browser to avoid race condition
    // where the callback arrives before the manager is stored
    *AUTH_MANAGER.lock().await = Some(manager);

    // Open browser to auth URL
    if let Err(e) = open::that(&auth_url) {
        tracing::warn!("Failed to open browser: {}", e);
        return Ok(format!("Please open this URL in your browser:\n\n{}", auth_url));
    }

    tracing::info!("Browser opened, waiting for callback...");

    Ok("Authentication started. Please complete the process in your browser.".to_string())
}

#[tauri::command]
pub async fn complete_google_auth() -> Result<String, String> {
    tracing::info!("Completing Google OAuth flow...");

    // Take the manager out of the lock so we don't hold it across the await.
    // This allows other auth commands (get_auth_status, logout) to proceed.
    let manager = {
        let mut lock = AUTH_MANAGER.lock().await;
        lock.take().ok_or_else(|| "OAuth flow not started".to_string())?
    };

    // Wait for callback and exchange code for tokens (no lock held)
    let result = manager.complete_auth_flow().await.map_err(|e| e.to_string());

    match result {
        Ok(tokens) => {
            tracing::info!("Google authentication successful!");
            Ok(format!("Successfully authenticated! Token expires at: {:?}", tokens.expires_at))
        }
        Err(e) => {
            // Auth failed — don't leave a stale manager
            tracing::error!("Google authentication failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_auth_status() -> Result<AuthStatus, String> {
    tracing::debug!("Checking authentication status...");

    let manager = AuthManager::new().map_err(|e| e.to_string())?;

    Ok(AuthStatus {
        is_authenticated: manager.is_authenticated(),
        provider: "google_drive".to_string(),
        email: None, // TODO: Get user email from Google API
    })
}

#[tauri::command]
pub async fn logout() -> Result<String, String> {
    tracing::info!("Logging out...");

    let manager = AuthManager::new().map_err(|e| e.to_string())?;
    manager.logout().map_err(|e| e.to_string())?;

    // Clear stored manager
    *AUTH_MANAGER.lock().await = None;

    Ok("Logged out successfully".to_string())
}

/// Get an AuthManager instance for use in sync operations
pub fn get_auth_manager() -> Arc<std::sync::Mutex<AuthManager>> {
    // Create a new auth manager - it will load tokens from keyring
    let manager = AuthManager::new().unwrap_or_else(|e| {
        tracing::error!("Failed to create auth manager: {}", e);
        panic!("Failed to create auth manager");
    });

    Arc::new(std::sync::Mutex::new(manager))
}
