use crate::core::auth_manager::AuthManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub provider: String,
    pub email: Option<String>,
}

#[tauri::command]
pub async fn google_auth() -> Result<String, String> {
    tracing::info!("Starting Google OAuth flow...");

    let mut manager = AuthManager::new().map_err(|e| e.to_string())?;

    match manager.authenticate().await {
        Ok(tokens) => {
            tracing::info!("Google authentication successful!");
            Ok(format!(
                "Successfully authenticated! Token expires at: {:?}",
                tokens.expires_at
            ))
        }
        Err(e) => {
            tracing::error!("Google authentication failed: {}", e);
            Err(e.to_string())
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
        email: None,
    })
}

#[tauri::command]
pub async fn logout() -> Result<String, String> {
    tracing::info!("Logging out...");

    let manager = AuthManager::new().map_err(|e| e.to_string())?;
    manager.logout().map_err(|e| e.to_string())?;

    Ok("Logged out successfully".to_string())
}
