use crate::models::conflict::ConflictResolution;
use crate::utils::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub file_path: String,
    pub local_hash: Option<String>,
    pub gdrive_hash: Option<String>,
    pub smb_hash: Option<String>,
}

pub struct ConflictResolver {}

impl ConflictResolver {
    pub fn new() -> Self {
        Self {}
    }

    pub fn resolve_conflict(
        &self,
        conflict: &Conflict,
        resolution: ConflictResolution,
    ) -> Result<ResolvedConflict> {
        // Determine which version to keep based on resolution strategy
        let source = match resolution {
            ConflictResolution::KeepLocal => ConflictSource::Local,
            ConflictResolution::KeepGoogleDrive => ConflictSource::GoogleDrive,
            ConflictResolution::KeepSmb => ConflictSource::Smb,
            ConflictResolution::KeepBoth => ConflictSource::KeepAll,
        };

        Ok(ResolvedConflict {
            file_path: conflict.file_path.clone(),
            source,
            resolution,
        })
    }

    pub fn detect_conflicts(
        &self,
        local_hash: Option<&str>,
        gdrive_hash: Option<&str>,
        smb_hash: Option<&str>,
    ) -> Option<Conflict> {
        let hashes: Vec<Option<&str>> = vec![local_hash, gdrive_hash, smb_hash];

        // If all hashes that exist are the same, no conflict
        let unique_hashes: Vec<&&str> = hashes
            .iter()
            .filter_map(|h| h.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if unique_hashes.len() <= 1 {
            return None;
        }

        // Multiple different hashes = conflict
        Some(Conflict {
            file_path: String::new(),
            local_hash: local_hash.map(String::from),
            gdrive_hash: gdrive_hash.map(String::from),
            smb_hash: smb_hash.map(String::from),
        })
    }
}

#[derive(Debug)]
pub enum ConflictSource {
    Local,
    GoogleDrive,
    Smb,
    KeepAll,
}

#[derive(Debug)]
pub struct ResolvedConflict {
    pub file_path: String,
    pub source: ConflictSource,
    pub resolution: ConflictResolution,
}
