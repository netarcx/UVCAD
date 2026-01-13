use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileLocation {
    Local,
    GoogleDrive,
    Smb,
}

impl FileLocation {
    pub fn as_str(&self) -> &str {
        match self {
            FileLocation::Local => "local",
            FileLocation::GoogleDrive => "gdrive",
            FileLocation::Smb => "smb",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "local" => Some(FileLocation::Local),
            "gdrive" => Some(FileLocation::GoogleDrive),
            "smb" => Some(FileLocation::Smb),
            _ => None,
        }
    }
}

impl FromStr for FileLocation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(FileLocation::Local),
            "gdrive" => Ok(FileLocation::GoogleDrive),
            "smb" => Ok(FileLocation::Smb),
            _ => Err(format!("Invalid file location: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Synced,
    Modified,
    Deleted,
    Conflict,
    Pending,
}

impl SyncStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SyncStatus::Synced => "synced",
            SyncStatus::Modified => "modified",
            SyncStatus::Deleted => "deleted",
            SyncStatus::Conflict => "conflict",
            SyncStatus::Pending => "pending",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "synced" => Some(SyncStatus::Synced),
            "modified" => Some(SyncStatus::Modified),
            "deleted" => Some(SyncStatus::Deleted),
            "conflict" => Some(SyncStatus::Conflict),
            "pending" => Some(SyncStatus::Pending),
            _ => None,
        }
    }
}

impl FromStr for SyncStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "synced" => Ok(SyncStatus::Synced),
            "modified" => Ok(SyncStatus::Modified),
            "deleted" => Ok(SyncStatus::Deleted),
            "conflict" => Ok(SyncStatus::Conflict),
            "pending" => Ok(SyncStatus::Pending),
            _ => Err(format!("Invalid sync status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub id: Option<i64>,
    pub profile_id: i64,
    pub file_path: String,
    pub location: FileLocation,
    pub content_hash: Option<String>,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<DateTime<Utc>>,
    pub synced_at: Option<DateTime<Utc>>,
    pub status: SyncStatus,
    pub metadata: Option<String>,
}

impl FileState {
    pub fn new(
        profile_id: i64,
        file_path: String,
        location: FileLocation,
    ) -> Self {
        Self {
            id: None,
            profile_id,
            file_path,
            location,
            content_hash: None,
            size_bytes: None,
            modified_at: None,
            synced_at: None,
            status: SyncStatus::Pending,
            metadata: None,
        }
    }
}
