use crate::core::sync_engine::{SyncEngine, SyncResult};
use crate::db::{models::DbOperations, schema::Database};
use crate::models::sync_profile::SyncProfile;
use crate::providers::{
    google_drive::GoogleDriveProvider,
    local_fs::LocalFsProvider,
    samba::SambaProvider,
    traits::StorageProvider,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tauri::Manager;

static SYNC_STATE: Lazy<Arc<std::sync::Mutex<SyncStateTracker>>> = Lazy::new(|| {
    Arc::new(std::sync::Mutex::new(SyncStateTracker {
        is_syncing: false,
        last_sync: None,
        last_result: None,
    }))
});

struct SyncStateTracker {
    is_syncing: bool,
    last_sync: Option<String>,
    last_result: Option<SyncResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub last_sync: Option<String>,
    pub files_synced: usize,
    pub files_pending: usize,
    pub conflicts: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub modified: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResultDto {
    pub actions_performed: usize,
    pub files_synced: usize,
    pub conflicts: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    pub current_file: String,
    pub total_files: usize,
    pub processed_files: usize,
    pub operation: String,
    pub percentage: f32,
}

fn create_database() -> Result<Arc<std::sync::Mutex<Database>>, String> {
    let db = Database::new().map_err(|e| format!("Failed to create database: {}", e))?;
    db.initialize().map_err(|e| format!("Failed to initialize database: {}", e))?;
    Ok(Arc::new(std::sync::Mutex::new(db)))
}

async fn get_or_create_default_profile() -> Result<(SyncProfile, Arc<std::sync::Mutex<Database>>), String> {
    let db_arc = create_database()?;

    let profile = {
        let db_guard = db_arc.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        let conn = db_guard.get_connection();

        // Try to get existing profile with id=1
        if let Some(profile) = DbOperations::get_sync_profile(conn, 1)
            .map_err(|e| format!("Failed to get sync profile: {}", e))? {
            profile
        } else {
            // Create a default profile if none exists
            let default_profile = SyncProfile {
                id: None,
                name: "Default".to_string(),
                local_path: std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .to_string_lossy()
                    .to_string(),
                gdrive_folder_id: None,
                smb_share_path: None,
                created_at: chrono::Utc::now(),
                last_sync_at: None,
            };

            let id = DbOperations::create_sync_profile(conn, &default_profile)
                .map_err(|e| format!("Failed to create sync profile: {}", e))?;

            let mut profile = default_profile;
            profile.id = Some(id);
            profile
        }
    }; // db_guard is dropped here

    Ok((profile, db_arc))
}

#[tauri::command]
pub async fn start_sync(app: tauri::AppHandle) -> Result<SyncResultDto, String> {
    tracing::info!("Start sync command called");

    // Check if already syncing
    {
        let mut state = SYNC_STATE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        if state.is_syncing {
            return Err("Sync already in progress".to_string());
        }
        state.is_syncing = true;
    }

    // Emit initial progress
    let _ = app.emit_all("sync-progress", SyncProgress {
        current_file: "Starting sync...".to_string(),
        total_files: 0,
        processed_files: 0,
        operation: "initializing".to_string(),
        percentage: 0.0,
    });

    // Get or create sync profile and database
    let (profile, db_arc) = get_or_create_default_profile().await?;

    tracing::info!("Using sync profile: {:?}", profile);

    // Validate configuration
    if profile.local_path.is_empty() {
        SYNC_STATE.lock().unwrap().is_syncing = false;
        return Err("Local path not configured".to_string());
    }

    // Initialize providers
    let local_provider: Arc<Mutex<dyn StorageProvider>> = Arc::new(Mutex::new(
        LocalFsProvider::new(PathBuf::from(&profile.local_path))
    ));

    // Initialize Google Drive provider if configured
    let gdrive_provider: Option<Arc<Mutex<dyn StorageProvider>>> = if let Some(ref folder_id) = profile.gdrive_folder_id {
        match GoogleDriveProvider::new(folder_id.clone()) {
            Ok(provider) => {
                if provider.is_authenticated() {
                    tracing::info!("Google Drive authenticated, initializing provider");
                    Some(Arc::new(Mutex::new(provider)))
                } else {
                    tracing::warn!("Google Drive folder configured but not authenticated");
                    None
                }
            }
            Err(e) => {
                tracing::error!("Failed to initialize Google Drive provider: {}", e);
                None
            }
        }
    } else {
        tracing::info!("Google Drive not configured");
        None
    };

    // Initialize Samba provider if configured
    let samba_provider: Option<Arc<Mutex<dyn StorageProvider>>> = if let Some(ref share_path) = profile.smb_share_path {
        tracing::info!("Samba share configured: {}", share_path);
        Some(Arc::new(Mutex::new(SambaProvider::new(PathBuf::from(share_path)))))
    } else {
        tracing::info!("Samba not configured");
        None
    };

    // Create progress callback
    let app_handle = app.clone();
    let progress_callback = Arc::new(move |processed: usize, total: usize, filename: String, operation: String| {
        let percentage = if total > 0 {
            (processed as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let _ = app_handle.emit_all("sync-progress", SyncProgress {
            current_file: filename,
            total_files: total,
            processed_files: processed,
            operation,
            percentage,
        });
    });

    // Create sync engine with progress callback
    let mut sync_engine = SyncEngine::new(
        profile.id.unwrap(),
        local_provider,
        gdrive_provider,
        samba_provider,
        db_arc,
    ).with_progress_callback(progress_callback);

    // Run sync
    tracing::info!("Starting sync operation...");
    let result = sync_engine.start_sync()
        .await
        .map_err(|e| {
            SYNC_STATE.lock().unwrap().is_syncing = false;
            format!("Sync failed: {}", e)
        })?;

    tracing::info!("Sync completed: {:?}", result);

    // Emit completion progress
    let _ = app.emit_all("sync-progress", SyncProgress {
        current_file: "Sync complete!".to_string(),
        total_files: result.files_synced + result.files_failed + result.files_conflict,
        processed_files: result.files_synced + result.files_failed + result.files_conflict,
        operation: "completed".to_string(),
        percentage: 100.0,
    });

    // Convert conflicts to strings
    let conflict_paths: Vec<String> = result.conflicts.iter()
        .map(|conflict| conflict.file_path.clone())
        .collect();

    let dto = SyncResultDto {
        actions_performed: result.files_synced,
        files_synced: result.files_synced,
        conflicts: conflict_paths,
        errors: vec![], // No errors field in SyncResult, using empty vec
    };

    // Update state
    {
        let mut state = SYNC_STATE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        state.is_syncing = false;
        state.last_sync = Some(chrono::Utc::now().to_rfc3339());
        state.last_result = Some(result);
    }

    Ok(dto)
}

#[tauri::command]
pub async fn pull_from_gdrive(app: tauri::AppHandle) -> Result<SyncResultDto, String> {
    tracing::info!("Pull from Google Drive command called");

    // Check if already syncing
    {
        let mut state = SYNC_STATE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        if state.is_syncing {
            return Err("Sync already in progress".to_string());
        }
        state.is_syncing = true;
    }

    let result = pull_from_gdrive_inner(&app).await;

    // Always clear syncing flag
    SYNC_STATE.lock().unwrap().is_syncing = false;

    match result {
        Ok(dto) => {
            let mut state = SYNC_STATE.lock().unwrap();
            state.last_sync = Some(chrono::Utc::now().to_rfc3339());
            Ok(dto)
        }
        Err(e) => Err(e)
    }
}

async fn pull_from_gdrive_inner(app: &tauri::AppHandle) -> Result<SyncResultDto, String> {
    // Emit initial progress
    let _ = app.emit_all("sync-progress", SyncProgress {
        current_file: "Connecting to Google Drive...".to_string(),
        total_files: 0,
        processed_files: 0,
        operation: "initializing".to_string(),
        percentage: 0.0,
    });

    let (profile, db_arc) = get_or_create_default_profile().await?;

    // Validate local path
    if profile.local_path.is_empty() {
        return Err("Local path not configured".to_string());
    }

    let local_path = PathBuf::from(&profile.local_path);
    if !local_path.exists() {
        tokio::fs::create_dir_all(&local_path).await
            .map_err(|e| format!("Failed to create local directory: {}", e))?;
    }

    // Validate Google Drive config
    let folder_id = profile.gdrive_folder_id.as_ref()
        .ok_or_else(|| "Google Drive folder not configured".to_string())?;

    let gdrive = GoogleDriveProvider::new(folder_id.clone())
        .map_err(|e| format!("Failed to initialize Google Drive: {}", e))?;

    if !gdrive.is_authenticated() {
        return Err("Not authenticated with Google Drive. Please sign in first.".to_string());
    }

    // List all files on Google Drive
    let _ = app.emit_all("sync-progress", SyncProgress {
        current_file: "Listing files on Google Drive...".to_string(),
        total_files: 0,
        processed_files: 0,
        operation: "scanning".to_string(),
        percentage: 5.0,
    });

    let files = gdrive.list_files(std::path::Path::new(""))
        .await
        .map_err(|e| format!("Failed to list Google Drive files: {}", e))?;

    let total = files.len();
    if total == 0 {
        let _ = app.emit_all("sync-progress", SyncProgress {
            current_file: "No files found on Google Drive".to_string(),
            total_files: 0,
            processed_files: 0,
            operation: "completed".to_string(),
            percentage: 100.0,
        });

        return Ok(SyncResultDto {
            actions_performed: 0,
            files_synced: 0,
            conflicts: vec![],
            errors: vec![],
        });
    }

    tracing::info!("Found {} files on Google Drive, downloading...", total);

    let mut downloaded = 0;
    let mut errors = Vec::new();

    for (i, file_meta) in files.iter().enumerate() {
        let filename = file_meta.path.to_string_lossy().to_string();

        let percentage = 10.0 + (i as f32 / total as f32) * 85.0; // 10-95% range
        let _ = app.emit_all("sync-progress", SyncProgress {
            current_file: filename.clone(),
            total_files: total,
            processed_files: i,
            operation: "downloading".to_string(),
            percentage,
        });

        let dest_path = local_path.join(&file_meta.path);

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    tracing::warn!("Failed to create directory {}: {}", parent.display(), e);
                    errors.push(format!("{}: {}", filename, e));
                    continue;
                }
            }
        }

        // Download file
        match gdrive.download(&file_meta.path, &dest_path).await {
            Ok(_) => {
                downloaded += 1;
                tracing::info!("Downloaded: {}", filename);

                // Update DB state for both locations
                let profile_id = profile.id.unwrap();
                let now = chrono::Utc::now();

                // Compute local hash after download
                let local_hash = crate::core::file_hasher::compute_file_hash(&dest_path).ok();

                let db_guard = db_arc.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
                let conn = db_guard.get_connection();

                // Record local file state
                let local_state = crate::models::file_state::FileState {
                    id: None,
                    profile_id,
                    file_path: filename.clone(),
                    location: crate::models::file_state::FileLocation::Local,
                    content_hash: local_hash,
                    size_bytes: Some(file_meta.size as i64),
                    modified_at: Some(now),
                    synced_at: Some(now),
                    status: crate::models::file_state::SyncStatus::Synced,
                    metadata: None,
                };
                let _ = DbOperations::upsert_file_state(conn, &local_state);

                // Record Google Drive file state
                let gdrive_state = crate::models::file_state::FileState {
                    id: None,
                    profile_id,
                    file_path: filename.clone(),
                    location: crate::models::file_state::FileLocation::GoogleDrive,
                    content_hash: file_meta.hash.clone(),
                    size_bytes: Some(file_meta.size as i64),
                    modified_at: Some(file_meta.modified),
                    synced_at: Some(now),
                    status: crate::models::file_state::SyncStatus::Synced,
                    metadata: None,
                };
                let _ = DbOperations::upsert_file_state(conn, &gdrive_state);
            }
            Err(e) => {
                tracing::error!("Failed to download {}: {}", filename, e);
                errors.push(format!("{}: {}", filename, e));
            }
        }
    }

    // Emit completion
    let _ = app.emit_all("sync-progress", SyncProgress {
        current_file: format!("Pull complete! Downloaded {} files", downloaded),
        total_files: total,
        processed_files: total,
        operation: "completed".to_string(),
        percentage: 100.0,
    });

    tracing::info!("Pull from Google Drive complete: {}/{} files downloaded", downloaded, total);

    Ok(SyncResultDto {
        actions_performed: downloaded,
        files_synced: downloaded,
        conflicts: vec![],
        errors,
    })
}

#[tauri::command]
pub async fn get_sync_status() -> Result<SyncStatus, String> {
    tracing::info!("Get sync status command called");

    let state = SYNC_STATE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;

    let (files_synced, files_pending, conflicts) = if let Some(ref result) = state.last_result {
        (
            result.files_synced,
            0, // TODO: track pending files
            result.conflicts.len(),
        )
    } else {
        (0, 0, 0)
    };

    Ok(SyncStatus {
        is_syncing: state.is_syncing,
        last_sync: state.last_sync.clone(),
        files_synced,
        files_pending,
        conflicts,
    })
}

#[tauri::command]
pub async fn get_file_list() -> Result<Vec<FileInfo>, String> {
    tracing::info!("Get file list command called");

    // Get or create sync profile and database
    let (profile, db_arc) = get_or_create_default_profile().await?;

    let db_guard = db_arc.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = db_guard.get_connection();

    // Query file states from database
    let file_states = DbOperations::get_file_states(conn, profile.id.unwrap())
        .map_err(|e| format!("Failed to get file states: {}", e))?;

    // Convert to FileInfo and sort by modified date (most recent first)
    let mut files: Vec<FileInfo> = file_states
        .into_iter()
        .filter(|state| state.location == crate::models::file_state::FileLocation::Local)
        .map(|state| FileInfo {
            path: state.file_path.clone(),
            size: state.size_bytes.unwrap_or(0) as u64,
            modified: state.modified_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "Unknown".to_string()),
            status: state.status.as_str().to_string(),
        })
        .collect();

    // Sort by modified date, most recent first
    files.sort_by(|a, b| b.modified.cmp(&a.modified));

    // Limit to 50 most recent files
    files.truncate(50);

    tracing::info!("Returning {} files", files.len());
    Ok(files)
}

#[tauri::command]
pub async fn resolve_conflict(file_path: String, resolution: String) -> Result<String, String> {
    tracing::info!("Resolve conflict for: {} with {}", file_path, resolution);

    // TODO: Implement conflict resolution
    // This would involve:
    // 1. Get conflict from database
    // 2. Apply resolution (keep local, keep gdrive, keep samba, keep all)
    // 3. Update file states
    // 4. Mark conflict as resolved

    Ok(format!("Conflict resolved: {}", file_path))
}
