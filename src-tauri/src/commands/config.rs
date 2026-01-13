use crate::db::{models::DbOperations, schema::Database};
use crate::models::sync_profile::SyncProfile;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub local_path: Option<String>,
    pub gdrive_folder_id: Option<String>,
    pub smb_share_path: Option<String>,
}

fn get_config_database() -> Result<Database, String> {
    let db = Database::new().map_err(|e| format!("Failed to create database: {}", e))?;
    db.initialize().map_err(|e| format!("Failed to initialize database: {}", e))?;
    Ok(db)
}

#[tauri::command]
pub async fn get_config() -> Result<AppConfig, String> {
    tracing::info!("Get config command called");

    let db = get_config_database()?;
    let conn = db.get_connection();

    // Try to get existing profile with id=1
    if let Some(profile) = DbOperations::get_sync_profile(conn, 1)
        .map_err(|e| format!("Failed to get sync profile: {}", e))? {
        return Ok(AppConfig {
            local_path: Some(profile.local_path),
            gdrive_folder_id: profile.gdrive_folder_id,
            smb_share_path: profile.smb_share_path,
        });
    }

    // Return empty config if no profile exists
    Ok(AppConfig {
        local_path: None,
        gdrive_folder_id: None,
        smb_share_path: None,
    })
}

#[tauri::command]
pub async fn update_config(config: AppConfig) -> Result<String, String> {
    tracing::info!("Update config command called: {:?}", config);

    // Validate local path if provided
    if let Some(ref path) = config.local_path {
        if !Path::new(path).exists() {
            return Err(format!("Local path does not exist: {}", path));
        }
        if !Path::new(path).is_dir() {
            return Err(format!("Local path is not a directory: {}", path));
        }
    } else {
        return Err("Local path is required".to_string());
    }

    let db = get_config_database()?;
    let conn = db.get_connection();

    // Try to get existing profile
    if let Some(mut profile) = DbOperations::get_sync_profile(conn, 1)
        .map_err(|e| format!("Failed to get sync profile: {}", e))? {
        // Update existing profile
        profile.local_path = config.local_path.unwrap();
        profile.gdrive_folder_id = config.gdrive_folder_id;
        profile.smb_share_path = config.smb_share_path;

        // Update in database
        conn.execute(
            "UPDATE sync_profiles SET local_path = ?1, gdrive_folder_id = ?2, smb_share_path = ?3 WHERE id = ?4",
            rusqlite::params![
                profile.local_path,
                profile.gdrive_folder_id,
                profile.smb_share_path,
                profile.id.unwrap(),
            ],
        ).map_err(|e| format!("Failed to update sync profile: {}", e))?;
    } else {
        // Create new profile
        let new_profile = SyncProfile {
            id: None,
            name: "Default".to_string(),
            local_path: config.local_path.unwrap(),
            gdrive_folder_id: config.gdrive_folder_id,
            smb_share_path: config.smb_share_path,
            created_at: chrono::Utc::now(),
            last_sync_at: None,
        };

        DbOperations::create_sync_profile(conn, &new_profile)
            .map_err(|e| format!("Failed to create sync profile: {}", e))?;
    }

    Ok("Configuration saved successfully".to_string())
}

#[tauri::command]
pub async fn test_smb_connection(path: String) -> Result<bool, String> {
    tracing::info!("Test SMB connection: {}", path);

    // Test if path exists and is accessible
    let smb_path = Path::new(&path);

    if !smb_path.exists() {
        return Ok(false);
    }

    if !smb_path.is_dir() {
        return Err("Path exists but is not a directory".to_string());
    }

    // Try to read the directory
    match std::fs::read_dir(smb_path) {
        Ok(_) => Ok(true),
        Err(e) => {
            tracing::warn!("Failed to read SMB directory: {}", e);
            Ok(false)
        }
    }
}
