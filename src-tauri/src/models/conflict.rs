use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolution {
    KeepLocal,
    KeepGoogleDrive,
    KeepSmb,
    KeepBoth,
}

impl ConflictResolution {
    pub fn as_str(&self) -> &str {
        match self {
            ConflictResolution::KeepLocal => "keep_local",
            ConflictResolution::KeepGoogleDrive => "keep_gdrive",
            ConflictResolution::KeepSmb => "keep_smb",
            ConflictResolution::KeepBoth => "keep_both",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "keep_local" => Some(ConflictResolution::KeepLocal),
            "keep_gdrive" => Some(ConflictResolution::KeepGoogleDrive),
            "keep_smb" => Some(ConflictResolution::KeepSmb),
            "keep_both" => Some(ConflictResolution::KeepBoth),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: Option<i64>,
    pub profile_id: i64,
    pub file_path: String,
    pub detected_at: DateTime<Utc>,
    pub resolved: bool,
    pub resolution: Option<ConflictResolution>,
    pub local_hash: Option<String>,
    pub gdrive_hash: Option<String>,
    pub smb_hash: Option<String>,
    pub local_modified: Option<DateTime<Utc>>,
    pub gdrive_modified: Option<DateTime<Utc>>,
    pub smb_modified: Option<DateTime<Utc>>,
    pub local_size: Option<i64>,
    pub gdrive_size: Option<i64>,
    pub smb_size: Option<i64>,
}

impl Conflict {
    pub fn new(profile_id: i64, file_path: String) -> Self {
        Self {
            id: None,
            profile_id,
            file_path,
            detected_at: Utc::now(),
            resolved: false,
            resolution: None,
            local_hash: None,
            gdrive_hash: None,
            smb_hash: None,
            local_modified: None,
            gdrive_modified: None,
            smb_modified: None,
            local_size: None,
            gdrive_size: None,
            smb_size: None,
        }
    }
}
