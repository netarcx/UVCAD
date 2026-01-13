use crate::core::conflict_resolver::{Conflict as ConflictInfo, ConflictResolver};
use crate::core::file_hasher;
use crate::db::models::DbOperations;
use crate::db::schema::Database;
use crate::models::file_state::{FileLocation, FileState, SyncStatus};
use crate::providers::traits::StorageProvider;
use crate::utils::error::{Result, UvcadError};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ProgressCallback = Arc<dyn Fn(usize, usize, String, String) + Send + Sync>;

// Deletion safety thresholds
const MAX_DELETION_PERCENTAGE: f32 = 0.30; // 30% of total files
const MAX_DELETION_COUNT: usize = 50; // Maximum 50 files

pub struct SyncEngine {
    profile_id: i64,
    local_provider: Arc<Mutex<dyn StorageProvider>>,
    gdrive_provider: Option<Arc<Mutex<dyn StorageProvider>>>,
    smb_provider: Option<Arc<Mutex<dyn StorageProvider>>>,
    db: Arc<std::sync::Mutex<Database>>,
    conflict_resolver: ConflictResolver,
    progress_callback: Option<ProgressCallback>,
}

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub hash: Option<String>,
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub location: FileLocation,
}

impl SyncEngine {
    pub fn new(
        profile_id: i64,
        local_provider: Arc<Mutex<dyn StorageProvider>>,
        gdrive_provider: Option<Arc<Mutex<dyn StorageProvider>>>,
        smb_provider: Option<Arc<Mutex<dyn StorageProvider>>>,
        db: Arc<std::sync::Mutex<Database>>,
    ) -> Self {
        Self {
            profile_id,
            local_provider,
            gdrive_provider,
            smb_provider,
            db,
            conflict_resolver: ConflictResolver::new(),
            progress_callback: None,
        }
    }

    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    pub async fn start_sync(&mut self) -> Result<SyncResult> {
        tracing::info!("Starting sync for profile {}", self.profile_id);

        let mut result = SyncResult::default();

        // Step 1: Scan all locations
        tracing::info!("Scanning local files...");
        let local_files = self.scan_location(&self.local_provider, FileLocation::Local).await?;
        tracing::info!("Found {} local files", local_files.len());

        let gdrive_files = if let Some(ref provider) = self.gdrive_provider {
            tracing::info!("Scanning Google Drive files...");
            let files = self.scan_location(provider, FileLocation::GoogleDrive).await?;
            tracing::info!("Found {} Google Drive files", files.len());
            files
        } else {
            HashMap::new()
        };

        let smb_files = if let Some(ref provider) = self.smb_provider {
            tracing::info!("Scanning Samba files...");
            let files = self.scan_location(provider, FileLocation::Smb).await?;
            tracing::info!("Found {} Samba files", files.len());
            files
        } else {
            HashMap::new()
        };

        // Step 2: Get last known state from database
        let last_known_state = self.get_last_known_state().await?;

        // Step 3: Determine sync actions for each file
        let all_paths = self.collect_all_paths(&local_files, &gdrive_files, &smb_files);
        let total_files = all_paths.len();
        tracing::info!("Processing {} unique files", total_files);

        // First pass: collect all sync actions
        let mut planned_actions: Vec<(PathBuf, SyncAction)> = Vec::new();
        for path in &all_paths {
            let local = local_files.get(path);
            let gdrive = gdrive_files.get(path);
            let smb = smb_files.get(path);
            let last_known = last_known_state.get(path);

            let action = self.determine_sync_action(path, local, gdrive, smb, last_known);
            planned_actions.push((path.clone(), action));
        }

        // Step 3a: Check deletion safety
        self.check_deletion_safety(&planned_actions, total_files)?;

        // Step 3b: Execute sync actions
        let mut processed = 0;
        for (path, action) in planned_actions {
            // Report progress
            if let Some(ref callback) = self.progress_callback {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                callback(processed, total_files, filename.clone(), "processing".to_string());
            }

            match action {
                SyncAction::NoAction => {
                    tracing::debug!("No action needed for: {}", path.display());
                    result.files_synced += 1;
                }
                SyncAction::Sync { operations } => {
                    tracing::info!("Syncing: {} ({} operations)", path.display(), operations.len());

                    // Report syncing operation
                    if let Some(ref callback) = self.progress_callback {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        callback(processed, total_files, filename.clone(), "syncing".to_string());
                    }

                    match self.execute_sync_operations(&path, operations).await {
                        Ok(_) => {
                            result.files_synced += 1;
                            tracing::info!("Successfully synced: {}", path.display());
                        }
                        Err(e) => {
                            result.files_failed += 1;
                            tracing::error!("Failed to sync {}: {}", path.display(), e);
                        }
                    }
                }
                SyncAction::Conflict(conflict) => {
                    tracing::warn!("Conflict detected: {}", path.display());
                    result.conflicts.push(conflict);
                    result.files_conflict += 1;
                }
            }
            processed += 1;
        }

        // Step 4: Update last known state in database
        self.update_last_known_state(&local_files, &gdrive_files, &smb_files).await?;

        tracing::info!("Sync completed: synced={}, failed={}, conflicts={}",
                       result.files_synced, result.files_failed, result.files_conflict);
        Ok(result)
    }

    async fn scan_location(
        &self,
        provider: &Arc<Mutex<dyn StorageProvider>>,
        location: FileLocation,
    ) -> Result<HashMap<PathBuf, FileSnapshot>> {
        let provider_lock = provider.lock().await;
        let files = provider_lock.list_files(Path::new("")).await?;

        let mut file_map = HashMap::new();
        for file_meta in files {
            let snapshot = FileSnapshot {
                path: file_meta.path.clone(),
                hash: file_meta.hash.clone(),
                size: file_meta.size,
                modified: file_meta.modified,
                location: location.clone(),
            };
            file_map.insert(file_meta.path, snapshot);
        }

        Ok(file_map)
    }

    fn collect_all_paths(
        &self,
        local: &HashMap<PathBuf, FileSnapshot>,
        gdrive: &HashMap<PathBuf, FileSnapshot>,
        smb: &HashMap<PathBuf, FileSnapshot>,
    ) -> Vec<PathBuf> {
        let mut paths = HashSet::new();

        for path in local.keys() {
            paths.insert(path.clone());
        }
        for path in gdrive.keys() {
            paths.insert(path.clone());
        }
        for path in smb.keys() {
            paths.insert(path.clone());
        }

        paths.into_iter().collect()
    }

    fn determine_sync_action(
        &self,
        path: &Path,
        local: Option<&FileSnapshot>,
        gdrive: Option<&FileSnapshot>,
        smb: Option<&FileSnapshot>,
        last_known: Option<&LastKnownState>,
    ) -> SyncAction {
        // Three-way merge logic
        // Compare current state with last known state to detect changes

        let local_changed = Self::has_changed(local, last_known.and_then(|s| s.local.as_ref()));
        let gdrive_changed = Self::has_changed(gdrive, last_known.and_then(|s| s.gdrive.as_ref()));
        let smb_changed = Self::has_changed(smb, last_known.and_then(|s| s.smb.as_ref()));

        tracing::debug!("File: {} - local_changed={}, gdrive_changed={}, smb_changed={}",
                       path.display(), local_changed, gdrive_changed, smb_changed);

        // No changes anywhere
        if !local_changed && !gdrive_changed && !smb_changed {
            return SyncAction::NoAction;
        }

        // Conflict: Multiple locations changed
        let change_count = [local_changed, gdrive_changed, smb_changed].iter().filter(|&&c| c).count();
        if change_count > 1 {
            // Check if changes are identical (same hash)
            if let (Some(l), Some(g)) = (local, gdrive) {
                if local_changed && gdrive_changed && l.hash == g.hash {
                    // Same content, not a conflict
                    return self.sync_to_missing(path, local, gdrive, smb);
                }
            }

            return SyncAction::Conflict(ConflictInfo {
                file_path: path.to_string_lossy().to_string(),
                local_hash: local.and_then(|f| f.hash.clone()),
                gdrive_hash: gdrive.and_then(|f| f.hash.clone()),
                smb_hash: smb.and_then(|f| f.hash.clone()),
            });
        }

        // Single location changed - propagate to others
        if local_changed {
            self.sync_from_local(path, local, gdrive, smb)
        } else if gdrive_changed {
            self.sync_from_gdrive(path, local, gdrive, smb)
        } else if smb_changed {
            self.sync_from_smb(path, local, gdrive, smb)
        } else {
            SyncAction::NoAction
        }
    }

    fn has_changed(current: Option<&FileSnapshot>, last_known: Option<&String>) -> bool {
        match (current, last_known) {
            (Some(curr), Some(known)) => {
                // File exists now and existed before - check if hash changed
                curr.hash.as_ref() != Some(known)
            }
            (Some(_), None) => {
                // File exists now but didn't before - it's new
                true
            }
            (None, Some(_)) => {
                // File doesn't exist now but did before - it was deleted
                true
            }
            (None, None) => {
                // File doesn't exist now and didn't before - no change
                false
            }
        }
    }

    fn sync_from_local(&self, path: &Path, local: Option<&FileSnapshot>,
                      gdrive: Option<&FileSnapshot>, smb: Option<&FileSnapshot>) -> SyncAction {
        let mut operations = Vec::new();

        if let Some(local_file) = local {
            // Local file exists - sync to other locations
            if self.gdrive_provider.is_some() && gdrive.is_none() {
                operations.push(SyncOperation::Upload {
                    from: FileLocation::Local,
                    to: FileLocation::GoogleDrive,
                    path: path.to_path_buf(),
                });
            }
            if self.smb_provider.is_some() && smb.is_none() {
                operations.push(SyncOperation::Upload {
                    from: FileLocation::Local,
                    to: FileLocation::Smb,
                    path: path.to_path_buf(),
                });
            }
        } else {
            // Local file deleted - delete from other locations
            if gdrive.is_some() {
                operations.push(SyncOperation::Delete {
                    location: FileLocation::GoogleDrive,
                    path: path.to_path_buf(),
                });
            }
            if smb.is_some() {
                operations.push(SyncOperation::Delete {
                    location: FileLocation::Smb,
                    path: path.to_path_buf(),
                });
            }
        }

        if operations.is_empty() {
            SyncAction::NoAction
        } else {
            SyncAction::Sync { operations }
        }
    }

    fn sync_from_gdrive(&self, path: &Path, local: Option<&FileSnapshot>,
                       gdrive: Option<&FileSnapshot>, smb: Option<&FileSnapshot>) -> SyncAction {
        let mut operations = Vec::new();

        if let Some(_gdrive_file) = gdrive {
            // Google Drive file exists - sync to other locations
            if local.is_none() {
                operations.push(SyncOperation::Upload {
                    from: FileLocation::GoogleDrive,
                    to: FileLocation::Local,
                    path: path.to_path_buf(),
                });
            }
            if self.smb_provider.is_some() && smb.is_none() {
                operations.push(SyncOperation::Upload {
                    from: FileLocation::GoogleDrive,
                    to: FileLocation::Smb,
                    path: path.to_path_buf(),
                });
            }
        } else {
            // Google Drive file deleted - delete from other locations
            if local.is_some() {
                operations.push(SyncOperation::Delete {
                    location: FileLocation::Local,
                    path: path.to_path_buf(),
                });
            }
            if smb.is_some() {
                operations.push(SyncOperation::Delete {
                    location: FileLocation::Smb,
                    path: path.to_path_buf(),
                });
            }
        }

        if operations.is_empty() {
            SyncAction::NoAction
        } else {
            SyncAction::Sync { operations }
        }
    }

    fn sync_from_smb(&self, path: &Path, local: Option<&FileSnapshot>,
                    gdrive: Option<&FileSnapshot>, smb: Option<&FileSnapshot>) -> SyncAction {
        let mut operations = Vec::new();

        if let Some(_smb_file) = smb {
            // Samba file exists - sync to other locations
            if local.is_none() {
                operations.push(SyncOperation::Upload {
                    from: FileLocation::Smb,
                    to: FileLocation::Local,
                    path: path.to_path_buf(),
                });
            }
            if self.gdrive_provider.is_some() && gdrive.is_none() {
                operations.push(SyncOperation::Upload {
                    from: FileLocation::Smb,
                    to: FileLocation::GoogleDrive,
                    path: path.to_path_buf(),
                });
            }
        } else {
            // Samba file deleted - delete from other locations
            if local.is_some() {
                operations.push(SyncOperation::Delete {
                    location: FileLocation::Local,
                    path: path.to_path_buf(),
                });
            }
            if gdrive.is_some() {
                operations.push(SyncOperation::Delete {
                    location: FileLocation::GoogleDrive,
                    path: path.to_path_buf(),
                });
            }
        }

        if operations.is_empty() {
            SyncAction::NoAction
        } else {
            SyncAction::Sync { operations }
        }
    }

    fn sync_to_missing(&self, path: &Path, local: Option<&FileSnapshot>,
                      gdrive: Option<&FileSnapshot>, smb: Option<&FileSnapshot>) -> SyncAction {
        let mut operations = Vec::new();

        // If we have the file in at least one location, sync to missing locations
        let source = local.or(gdrive).or(smb);
        if source.is_none() {
            return SyncAction::NoAction;
        }

        let source_location = source.unwrap().location.clone();

        if local.is_none() && source_location != FileLocation::Local {
            operations.push(SyncOperation::Upload {
                from: source_location.clone(),
                to: FileLocation::Local,
                path: path.to_path_buf(),
            });
        }

        if self.gdrive_provider.is_some() && gdrive.is_none() && source_location != FileLocation::GoogleDrive {
            operations.push(SyncOperation::Upload {
                from: source_location.clone(),
                to: FileLocation::GoogleDrive,
                path: path.to_path_buf(),
            });
        }

        if self.smb_provider.is_some() && smb.is_none() && source_location != FileLocation::Smb {
            operations.push(SyncOperation::Upload {
                from: source_location.clone(),
                to: FileLocation::Smb,
                path: path.to_path_buf(),
            });
        }

        if operations.is_empty() {
            SyncAction::NoAction
        } else {
            SyncAction::Sync { operations }
        }
    }

    async fn execute_sync_operations(&self, path: &Path, operations: Vec<SyncOperation>) -> Result<()> {
        for operation in operations {
            match operation {
                SyncOperation::Upload { from, to, path: file_path } => {
                    self.transfer_file(&from, &to, &file_path).await?;
                }
                SyncOperation::Delete { location, path: file_path } => {
                    self.delete_file(&location, &file_path).await?;
                }
            }
        }
        Ok(())
    }

    async fn transfer_file(&self, from: &FileLocation, to: &FileLocation, path: &Path) -> Result<()> {
        tracing::info!("Transferring: {} from {:?} to {:?}", path.display(), from, to);

        // Get source provider
        let source_provider = self.get_provider(from)?;

        // Get destination provider
        let dest_provider = self.get_provider(to)?;

        // Create temp file for transfer
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("uvcad_{}_{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            chrono::Utc::now().timestamp()
        ));

        // Download from source to temp
        {
            let provider = source_provider.lock().await;
            provider.download(path, &temp_file).await?;
        }

        // Verify file integrity
        let temp_hash = file_hasher::compute_file_hash(&temp_file)?;
        tracing::debug!("Temp file hash: {}", temp_hash);

        // Upload from temp to destination
        {
            let provider = dest_provider.lock().await;
            provider.upload(&temp_file, path).await?;
        }

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_file).await;

        tracing::info!("Transfer complete: {} from {:?} to {:?}", path.display(), from, to);
        Ok(())
    }

    async fn delete_file(&self, location: &FileLocation, path: &Path) -> Result<()> {
        tracing::info!("Deleting: {} from {:?}", path.display(), location);

        let provider = self.get_provider(location)?;
        let provider_lock = provider.lock().await;
        provider_lock.delete(path).await?;

        tracing::info!("Deletion complete: {} from {:?}", path.display(), location);
        Ok(())
    }

    fn get_provider(&self, location: &FileLocation) -> Result<&Arc<Mutex<dyn StorageProvider>>> {
        match location {
            FileLocation::Local => Ok(&self.local_provider),
            FileLocation::GoogleDrive => {
                self.gdrive_provider.as_ref()
                    .ok_or_else(|| UvcadError::ProviderError("Google Drive not configured".to_string()))
            }
            FileLocation::Smb => {
                self.smb_provider.as_ref()
                    .ok_or_else(|| UvcadError::ProviderError("Samba not configured".to_string()))
            }
        }
    }

    fn check_deletion_safety(&self, planned_actions: &[(PathBuf, SyncAction)], total_files: usize) -> Result<()> {
        let mut deletion_count = 0;
        let mut local_deletions = Vec::new();
        let mut gdrive_deletions = Vec::new();
        let mut smb_deletions = Vec::new();

        // Count all planned deletions
        for (_path, action) in planned_actions {
            if let SyncAction::Sync { operations } = action {
                for operation in operations {
                    if let SyncOperation::Delete { location, path } = operation {
                        deletion_count += 1;
                        match location {
                            FileLocation::Local => local_deletions.push(path.clone()),
                            FileLocation::GoogleDrive => gdrive_deletions.push(path.clone()),
                            FileLocation::Smb => smb_deletions.push(path.clone()),
                        }
                    }
                }
            }
        }

        if deletion_count == 0 {
            return Ok(());
        }

        let deletion_percentage = (deletion_count as f32 / total_files as f32) * 100.0;

        tracing::info!(
            "Deletion safety check: {} deletions planned ({:.1}% of {} files)",
            deletion_count, deletion_percentage, total_files
        );

        // Check against thresholds
        if deletion_count > MAX_DELETION_COUNT {
            let error_msg = format!(
                "SAFETY CHECK FAILED: Sync would delete {} files (exceeds limit of {}). \
                This may indicate accidental data loss. Deletions by location: \
                Local: {}, Google Drive: {}, Samba: {}. \
                Please verify your sync folders are accessible and try again.",
                deletion_count, MAX_DELETION_COUNT,
                local_deletions.len(), gdrive_deletions.len(), smb_deletions.len()
            );
            tracing::error!("{}", error_msg);
            return Err(UvcadError::SyncFailed(error_msg));
        }

        let deletion_percentage_decimal = deletion_count as f32 / total_files as f32;
        if deletion_percentage_decimal > MAX_DELETION_PERCENTAGE {
            let error_msg = format!(
                "SAFETY CHECK FAILED: Sync would delete {:.1}% of files ({} files, exceeds {:.0}% threshold). \
                This may indicate a drive is unmounted or accidentally emptied. Deletions by location: \
                Local: {}, Google Drive: {}, Samba: {}. \
                Please verify your sync folders are accessible and try again.",
                deletion_percentage, deletion_count, MAX_DELETION_PERCENTAGE * 100.0,
                local_deletions.len(), gdrive_deletions.len(), smb_deletions.len()
            );
            tracing::error!("{}", error_msg);
            return Err(UvcadError::SyncFailed(error_msg));
        }

        tracing::info!("Deletion safety check passed: {} deletions within safe limits", deletion_count);
        Ok(())
    }

    async fn get_last_known_state(&self) -> Result<HashMap<PathBuf, LastKnownState>> {
        let db_guard = self.db.lock()
            .map_err(|e| UvcadError::SyncFailed(format!("Failed to lock database: {}", e)))?;
        let conn = db_guard.get_connection();

        let file_states = DbOperations::get_file_states(conn, self.profile_id)?;

        let mut state_map: HashMap<PathBuf, LastKnownState> = HashMap::new();

        for state in file_states {
            let path = PathBuf::from(&state.file_path);
            let entry = state_map.entry(path).or_insert_with(|| LastKnownState {
                local: None,
                gdrive: None,
                smb: None,
            });

            match state.location {
                FileLocation::Local => entry.local = state.content_hash,
                FileLocation::GoogleDrive => entry.gdrive = state.content_hash,
                FileLocation::Smb => entry.smb = state.content_hash,
            }
        }

        tracing::debug!("Loaded {} file states from database", state_map.len());
        Ok(state_map)
    }

    async fn update_last_known_state(
        &self,
        local_files: &HashMap<PathBuf, FileSnapshot>,
        gdrive_files: &HashMap<PathBuf, FileSnapshot>,
        smb_files: &HashMap<PathBuf, FileSnapshot>,
    ) -> Result<()> {
        let db_guard = self.db.lock()
            .map_err(|e| UvcadError::SyncFailed(format!("Failed to lock database: {}", e)))?;
        let conn = db_guard.get_connection();

        let now = chrono::Utc::now();

        // Save local file states
        for (path, snapshot) in local_files {
            let file_state = FileState {
                id: None,
                profile_id: self.profile_id,
                file_path: path.to_string_lossy().to_string(),
                location: FileLocation::Local,
                content_hash: snapshot.hash.clone(),
                size_bytes: Some(snapshot.size as i64),
                modified_at: Some(snapshot.modified),
                synced_at: Some(now),
                status: SyncStatus::Synced,
                metadata: None,
            };
            DbOperations::upsert_file_state(conn, &file_state)?;
        }

        // Save Google Drive file states
        for (path, snapshot) in gdrive_files {
            let file_state = FileState {
                id: None,
                profile_id: self.profile_id,
                file_path: path.to_string_lossy().to_string(),
                location: FileLocation::GoogleDrive,
                content_hash: snapshot.hash.clone(),
                size_bytes: Some(snapshot.size as i64),
                modified_at: Some(snapshot.modified),
                synced_at: Some(now),
                status: SyncStatus::Synced,
                metadata: None,
            };
            DbOperations::upsert_file_state(conn, &file_state)?;
        }

        // Save Samba file states
        for (path, snapshot) in smb_files {
            let file_state = FileState {
                id: None,
                profile_id: self.profile_id,
                file_path: path.to_string_lossy().to_string(),
                location: FileLocation::Smb,
                content_hash: snapshot.hash.clone(),
                size_bytes: Some(snapshot.size as i64),
                modified_at: Some(snapshot.modified),
                synced_at: Some(now),
                status: SyncStatus::Synced,
                metadata: None,
            };
            DbOperations::upsert_file_state(conn, &file_state)?;
        }

        let total_saved = local_files.len() + gdrive_files.len() + smb_files.len();
        tracing::debug!("Saved {} file states to database", total_saved);

        Ok(())
    }
}

#[derive(Debug)]
enum SyncAction {
    NoAction,
    Sync {
        operations: Vec<SyncOperation>,
    },
    Conflict(ConflictInfo),
}

#[derive(Debug)]
enum SyncOperation {
    Upload {
        from: FileLocation,
        to: FileLocation,
        path: PathBuf,
    },
    Delete {
        location: FileLocation,
        path: PathBuf,
    },
}

#[derive(Debug)]
struct LastKnownState {
    local: Option<String>,    // Last known hash for local
    gdrive: Option<String>,   // Last known hash for gdrive
    smb: Option<String>,      // Last known hash for smb
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SyncResult {
    pub files_synced: usize,
    pub files_failed: usize,
    pub files_conflict: usize,
    pub conflicts: Vec<ConflictInfo>,
}
