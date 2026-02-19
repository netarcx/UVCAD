use crate::core::file_hasher;
use crate::providers::traits::{FileMetadata, StorageProvider};
use crate::utils::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct LocalFsProvider {
    root_path: PathBuf,
}

impl LocalFsProvider {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    /// Convert a relative path to an absolute path under root_path.
    fn to_absolute(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root_path.join(path)
        }
    }

    /// Convert an absolute path to a relative path from root_path.
    /// This ensures all providers use consistent relative path keys.
    fn to_relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root_path)
            .unwrap_or(path)
            .to_path_buf()
    }

    /// Get file metadata for an absolute path, returning a relative path in the result.
    async fn get_file_metadata_absolute(&self, absolute_path: &Path) -> Result<Option<FileMetadata>> {
        match fs::metadata(absolute_path).await {
            Ok(metadata) => {
                let modified = metadata.modified()?;
                let modified_dt: DateTime<Utc> = modified.into();

                let hash = if metadata.is_file() {
                    Some(file_hasher::compute_file_hash(absolute_path)?)
                } else {
                    None
                };

                Ok(Some(FileMetadata {
                    path: self.to_relative(absolute_path),
                    size: metadata.len(),
                    modified: modified_dt,
                    hash,
                    exists: true,
                }))
            }
            Err(_) => Ok(None),
        }
    }
}

#[async_trait]
impl StorageProvider for LocalFsProvider {
    fn name(&self) -> &str {
        "local_fs"
    }

    async fn list_files(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let full_path = self.to_absolute(path);
        let mut files = Vec::new();

        let mut entries = fs::read_dir(&full_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let file_type = entry.file_type().await?;

            if file_type.is_file() {
                if let Some(metadata) = self.get_file_metadata_absolute(&entry_path).await? {
                    files.push(metadata);
                }
            } else if file_type.is_dir() {
                // Recursively list subdirectories using the absolute path
                let subfiles = self.list_files(&entry_path).await?;
                files.extend(subfiles);
            }
        }

        Ok(files)
    }

    async fn get_metadata(&self, path: &Path) -> Result<Option<FileMetadata>> {
        let full_path = self.to_absolute(path);
        self.get_file_metadata_absolute(&full_path).await
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        let full_path = self.to_absolute(path);
        Ok(full_path.exists())
    }

    async fn download(&self, path: &Path, dest: &Path) -> Result<PathBuf> {
        let full_path = self.to_absolute(path);
        fs::copy(&full_path, dest).await?;
        Ok(dest.to_path_buf())
    }

    async fn upload(&self, source: &Path, dest: &Path) -> Result<()> {
        let full_dest = self.to_absolute(dest);

        // Ensure parent directory exists
        if let Some(parent) = full_dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(source, &full_dest).await?;
        Ok(())
    }

    async fn delete(&self, path: &Path) -> Result<()> {
        let full_path = self.to_absolute(path);
        fs::remove_file(&full_path).await?;
        Ok(())
    }

    async fn initialize(&mut self) -> Result<()> {
        // Ensure root directory exists
        if !self.root_path.exists() {
            fs::create_dir_all(&self.root_path).await?;
        }
        Ok(())
    }

    async fn test_connection(&self) -> Result<bool> {
        Ok(self.root_path.exists() && self.root_path.is_dir())
    }
}
