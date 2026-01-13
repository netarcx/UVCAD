use crate::utils::error::Result;
use directories::ProjectDirs;
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        Ok(Self { conn })
    }

    fn get_db_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "uvcad", "UVCAD")
            .ok_or_else(|| crate::utils::error::UvcadError::InvalidConfig(
                "Failed to get project directory".to_string()
            ))?;

        let data_dir = project_dirs.data_dir();
        Ok(data_dir.join("uvcad.db"))
    }

    pub fn initialize(&self) -> Result<()> {
        self.create_tables()?;
        Ok(())
    }

    fn create_tables(&self) -> Result<()> {
        // Sync profiles table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                local_path TEXT NOT NULL,
                gdrive_folder_id TEXT,
                smb_share_path TEXT,
                created_at TEXT NOT NULL,
                last_sync_at TEXT
            )",
            [],
        )?;

        // File states table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file_states (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                profile_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                location TEXT NOT NULL,
                content_hash TEXT,
                size_bytes INTEGER,
                modified_at TEXT,
                synced_at TEXT,
                status TEXT NOT NULL,
                metadata TEXT,
                FOREIGN KEY (profile_id) REFERENCES sync_profiles(id),
                UNIQUE(profile_id, file_path, location)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_states_profile
             ON file_states(profile_id)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_states_status
             ON file_states(status)",
            [],
        )?;

        // Sync history table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                profile_id INTEGER NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL,
                files_synced INTEGER DEFAULT 0,
                files_failed INTEGER DEFAULT 0,
                error_message TEXT,
                FOREIGN KEY (profile_id) REFERENCES sync_profiles(id)
            )",
            [],
        )?;

        // Conflicts table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS conflicts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                profile_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                detected_at TEXT NOT NULL,
                resolved BOOLEAN DEFAULT FALSE,
                resolution TEXT,
                local_hash TEXT,
                gdrive_hash TEXT,
                smb_hash TEXT,
                local_modified TEXT,
                gdrive_modified TEXT,
                smb_modified TEXT,
                local_size INTEGER,
                gdrive_size INTEGER,
                smb_size INTEGER,
                FOREIGN KEY (profile_id) REFERENCES sync_profiles(id)
            )",
            [],
        )?;

        // OAuth tokens table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS oauth_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service TEXT NOT NULL UNIQUE,
                access_token TEXT NOT NULL,
                refresh_token TEXT,
                expires_at TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }
}
