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

    fn normalize_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root_path.join(path)
        }
    }
}

#[async_trait]
impl StorageProvider for LocalFsProvider {
    fn name(&self) -> &str {
        "local_fs"
    }

    async fn list_files(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let full_path = self.normalize_path(path);
        let mut files = Vec::new();

        let mut entries = fs::read_dir(&full_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(metadata) = self.get_metadata(&path).await? {
                    files.push(metadata);
                }
            } else if path.is_dir() {
                // Recursively list subdirectories
                let subfiles = self.list_files(&path).await?;
                files.extend(subfiles);
            }
        }

        Ok(files)
    }

    async fn get_metadata(&self, path: &Path) -> Result<Option<FileMetadata>> {
        let full_path = self.normalize_path(path);

        match fs::metadata(&full_path).await {
            Ok(metadata) => {
                let modified = metadata.modified()?;
                let modified_dt: DateTime<Utc> = modified.into();

                let hash = if metadata.is_file() {
                    Some(file_hasher::compute_file_hash(&full_path)?)
                } else {
                    None
                };

                Ok(Some(FileMetadata {
                    path: full_path,
                    size: metadata.len(),
                    modified: modified_dt,
                    hash,
                    exists: true,
                }))
            }
            Err(_) => Ok(None),
        }
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        let full_path = self.normalize_path(path);
        Ok(full_path.exists())
    }

    async fn download(&self, path: &Path, dest: &Path) -> Result<PathBuf> {
        let full_path = self.normalize_path(path);
        fs::copy(&full_path, dest).await?;
        Ok(dest.to_path_buf())
    }

    async fn upload(&self, source: &Path, dest: &Path) -> Result<()> {
        let full_dest = self.normalize_path(dest);

        // Ensure parent directory exists
        if let Some(parent) = full_dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(source, &full_dest).await?;
        Ok(())
    }

    async fn delete(&self, path: &Path) -> Result<()> {
        let full_path = self.normalize_path(path);
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
