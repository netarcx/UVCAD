// Database model operations
// This module provides CRUD operations for our domain models

use crate::models::{conflict::Conflict, file_state::FileState, sync_profile::SyncProfile};
use crate::utils::error::Result;
use rusqlite::{Connection, OptionalExtension};

pub struct DbOperations;

impl DbOperations {
    // Sync Profile operations
    pub fn create_sync_profile(conn: &Connection, profile: &SyncProfile) -> Result<i64> {
        conn.execute(
            "INSERT INTO sync_profiles (name, local_path, gdrive_folder_id, smb_share_path, created_at, last_sync_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                profile.name,
                profile.local_path,
                profile.gdrive_folder_id,
                profile.smb_share_path,
                profile.created_at.to_rfc3339(),
                profile.last_sync_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_sync_profile(conn: &Connection, id: i64) -> Result<Option<SyncProfile>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, local_path, gdrive_folder_id, smb_share_path, created_at, last_sync_at
             FROM sync_profiles WHERE id = ?1"
        )?;

        let profile = stmt.query_row([id], |row| {
            Ok(SyncProfile {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                local_path: row.get(2)?,
                gdrive_folder_id: row.get(3)?,
                smb_share_path: row.get(4)?,
                created_at: row.get::<_, String>(5)?.parse().unwrap(),
                last_sync_at: row.get::<_, Option<String>>(6)?
                    .and_then(|s| s.parse().ok()),
            })
        }).optional()?;

        Ok(profile)
    }

    // File State operations
    pub fn upsert_file_state(conn: &Connection, state: &FileState) -> Result<()> {
        conn.execute(
            "INSERT INTO file_states (profile_id, file_path, location, content_hash, size_bytes, modified_at, synced_at, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(profile_id, file_path, location) DO UPDATE SET
                content_hash = excluded.content_hash,
                size_bytes = excluded.size_bytes,
                modified_at = excluded.modified_at,
                synced_at = excluded.synced_at,
                status = excluded.status,
                metadata = excluded.metadata",
            rusqlite::params![
                state.profile_id,
                state.file_path,
                state.location.as_str(),
                state.content_hash,
                state.size_bytes,
                state.modified_at.map(|dt| dt.to_rfc3339()),
                state.synced_at.map(|dt| dt.to_rfc3339()),
                state.status.as_str(),
                state.metadata,
            ],
        )?;
        Ok(())
    }

    pub fn get_file_states(conn: &Connection, profile_id: i64) -> Result<Vec<FileState>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, file_path, location, content_hash, size_bytes,
                    modified_at, synced_at, status, metadata
             FROM file_states WHERE profile_id = ?1"
        )?;

        let states = stmt.query_map([profile_id], |row| {
            Ok(FileState {
                id: Some(row.get(0)?),
                profile_id: row.get(1)?,
                file_path: row.get(2)?,
                location: row.get::<_, String>(3)?.parse().unwrap_or(crate::models::file_state::FileLocation::Local),
                content_hash: row.get(4)?,
                size_bytes: row.get(5)?,
                modified_at: row.get::<_, Option<String>>(6)?
                    .and_then(|s| s.parse().ok()),
                synced_at: row.get::<_, Option<String>>(7)?
                    .and_then(|s| s.parse().ok()),
                status: row.get::<_, String>(8)?.parse().unwrap_or(crate::models::file_state::SyncStatus::Pending),
                metadata: row.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(states)
    }

    // Conflict operations
    pub fn create_conflict(conn: &Connection, conflict: &Conflict) -> Result<i64> {
        conn.execute(
            "INSERT INTO conflicts (profile_id, file_path, detected_at, resolved, resolution,
                                   local_hash, gdrive_hash, smb_hash,
                                   local_modified, gdrive_modified, smb_modified,
                                   local_size, gdrive_size, smb_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                conflict.profile_id,
                conflict.file_path,
                conflict.detected_at.to_rfc3339(),
                conflict.resolved,
                conflict.resolution.as_ref().map(|r| r.as_str()),
                conflict.local_hash,
                conflict.gdrive_hash,
                conflict.smb_hash,
                conflict.local_modified.map(|dt| dt.to_rfc3339()),
                conflict.gdrive_modified.map(|dt| dt.to_rfc3339()),
                conflict.smb_modified.map(|dt| dt.to_rfc3339()),
                conflict.local_size,
                conflict.gdrive_size,
                conflict.smb_size,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
