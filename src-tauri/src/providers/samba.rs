use crate::core::file_hasher;
use crate::providers::traits::{FileMetadata, StorageProvider};
use crate::utils::error::{Result, UvcadError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct SambaProvider {
    share_path: PathBuf,
    mounted: bool,
}

impl SambaProvider {
    pub fn new(share_path: PathBuf) -> Self {
        Self {
            share_path,
            mounted: false,
        }
    }

    /// Convert a relative path to an absolute path under share_path.
    fn to_absolute(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.share_path.join(path)
        }
    }

    /// Convert an absolute path to a relative path from share_path.
    fn to_relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.share_path)
            .unwrap_or(path)
            .to_path_buf()
    }

    async fn check_mount(&self) -> Result<bool> {
        // SMB shares are mounted as regular directories on both macOS (/Volumes/...)
        // and Windows (\\server\share or mapped drives). Check accessibility.
        Ok(self.share_path.exists() && self.share_path.is_dir())
    }

    /// Recursively list files under the given absolute directory path.
    fn list_files_recursive<'a>(&'a self, dir: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileMetadata>>> + Send + 'a>> {
        Box::pin(async move {
            let mut files = Vec::new();

            let mut entries = fs::read_dir(dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                let file_type = entry.file_type().await?;

                if file_type.is_file() {
                    match fs::metadata(&entry_path).await {
                        Ok(metadata) => {
                            let modified: DateTime<Utc> = metadata.modified()?.into();
                            let hash = file_hasher::compute_file_hash(&entry_path).ok();

                            files.push(FileMetadata {
                                path: self.to_relative(&entry_path),
                                size: metadata.len(),
                                modified,
                                hash,
                                exists: true,
                            });
                        }
                        Err(e) => {
                            tracing::warn!("Failed to get metadata for {}: {}", entry_path.display(), e);
                        }
                    }
                } else if file_type.is_dir() {
                    match self.list_files_recursive(&entry_path).await {
                        Ok(subfiles) => files.extend(subfiles),
                        Err(e) => {
                            tracing::warn!("Failed to list directory {}: {}", entry_path.display(), e);
                        }
                    }
                }
            }

            Ok(files)
        })
    }
}

#[async_trait]
impl StorageProvider for SambaProvider {
    fn name(&self) -> &str {
        "samba"
    }

    async fn list_files(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        if !self.mounted {
            return Err(UvcadError::SmbNotAccessible("SMB share not mounted".to_string()));
        }

        let full_path = self.to_absolute(path);
        self.list_files_recursive(&full_path).await
    }

    async fn get_metadata(&self, path: &Path) -> Result<Option<FileMetadata>> {
        if !self.mounted {
            return Err(UvcadError::SmbNotAccessible("SMB share not mounted".to_string()));
        }

        let full_path = self.to_absolute(path);
        match fs::metadata(&full_path).await {
            Ok(metadata) => {
                let modified: DateTime<Utc> = metadata.modified()?.into();
                let hash = if metadata.is_file() {
                    file_hasher::compute_file_hash(&full_path).ok()
                } else {
                    None
                };

                Ok(Some(FileMetadata {
                    path: self.to_relative(&full_path),
                    size: metadata.len(),
                    modified,
                    hash,
                    exists: true,
                }))
            }
            Err(_) => Ok(None),
        }
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
        self.mounted = self.check_mount().await?;

        if !self.mounted {
            return Err(UvcadError::SmbNotAccessible(
                format!("SMB share not accessible at: {}", self.share_path.display())
            ));
        }

        Ok(())
    }

    async fn test_connection(&self) -> Result<bool> {
        self.check_mount().await
    }
}
