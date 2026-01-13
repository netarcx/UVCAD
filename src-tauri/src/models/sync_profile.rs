use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProfile {
    pub id: Option<i64>,
    pub name: String,
    pub local_path: String,
    pub gdrive_folder_id: Option<String>,
    pub smb_share_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_sync_at: Option<DateTime<Utc>>,
}

impl SyncProfile {
    pub fn new(name: String, local_path: String) -> Self {
        Self {
            id: None,
            name,
            local_path,
            gdrive_folder_id: None,
            smb_share_path: None,
            created_at: Utc::now(),
            last_sync_at: None,
        }
    }
}
