use crate::utils::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

/// Metadata for a file in a storage provider
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub hash: Option<String>,
    pub exists: bool,
}

/// Common trait for all storage providers (Local FS, Google Drive, SMB)
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &str;

    /// List all files in the storage location
    async fn list_files(&self, path: &Path) -> Result<Vec<FileMetadata>>;

    /// Get metadata for a specific file
    async fn get_metadata(&self, path: &Path) -> Result<Option<FileMetadata>>;

    /// Check if a file exists
    async fn exists(&self, path: &Path) -> Result<bool>;

    /// Download a file to a local temporary location
    /// Returns the path to the temporary file
    async fn download(&self, path: &Path, dest: &Path) -> Result<PathBuf>;

    /// Upload a file from local location to this provider
    async fn upload(&self, source: &Path, dest: &Path) -> Result<()>;

    /// Delete a file
    async fn delete(&self, path: &Path) -> Result<()>;

    /// Initialize/connect to the storage provider
    async fn initialize(&mut self) -> Result<()>;

    /// Test if the connection is working
    async fn test_connection(&self) -> Result<bool>;
}
