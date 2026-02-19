use crate::core::file_hasher;
use crate::providers::traits::{FileMetadata, StorageProvider};
use crate::utils::error::{Result, UvcadError};
use crate::utils::keyring::{OAuthTokens, TokenManager};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
const DRIVE_UPLOAD_API: &str = "https://www.googleapis.com/upload/drive/v3";

#[derive(Debug, Deserialize)]
struct DriveFile {
    id: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    size: Option<String>,
    #[serde(rename = "modifiedTime")]
    modified_time: String,
    #[serde(rename = "md5Checksum")]
    md5_checksum: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileList {
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileMetadataUpload {
    name: String,
    parents: Vec<String>,
}

pub struct GoogleDriveProvider {
    folder_id: String,
    token_manager: TokenManager,
    client: reqwest::Client,
}

impl GoogleDriveProvider {
    pub fn new(folder_id: String) -> Result<Self> {
        let token_manager = TokenManager::new("google_drive")?;
        let client = reqwest::Client::new();

        Ok(Self {
            folder_id,
            token_manager,
            client,
        })
    }

    async fn get_access_token(&self) -> Result<String> {
        let tokens = self.token_manager.get_tokens()?;

        // Check if token is expired or expiring within 5 minutes
        if let Some(expires_at) = tokens.expires_at {
            let now = chrono::Utc::now().timestamp();
            if expires_at - now < 300 {
                tracing::info!("Access token expired or expiring soon, refreshing...");
                let mut auth_manager = crate::core::auth_manager::AuthManager::new()?;
                return auth_manager.get_valid_token().await;
            }
        }

        Ok(tokens.access_token)
    }

    pub fn is_authenticated(&self) -> bool {
        self.token_manager.has_tokens()
    }

    pub fn store_tokens(&self, tokens: OAuthTokens) -> Result<()> {
        self.token_manager.store_tokens(&tokens)
    }

    /// Escape a string for use in a Google Drive API query parameter.
    /// Single quotes must be escaped with a backslash.
    fn escape_drive_query(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\'', "\\'")
    }

    /// Recursively list all files under a folder, including subfolders.
    /// `prefix` is the relative path prefix for files in this folder.
    fn list_files_recursive<'a>(&'a self, folder_id: &'a str, prefix: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileMetadata>>> + Send + 'a>> {
        Box::pin(async move {
            let mut all_files = Vec::new();
            let mut page_token: Option<String> = None;

            loop {
                let file_list = self.list_files_in_folder(folder_id, page_token).await?;

                for file in file_list.files {
                    if file.mime_type == "application/vnd.google-apps.folder" {
                        // Recurse into subfolder
                        let sub_prefix = prefix.join(&file.name);
                        match self.list_files_recursive(&file.id, &sub_prefix).await {
                            Ok(sub_files) => all_files.extend(sub_files),
                            Err(e) => {
                                tracing::warn!("Failed to list subfolder '{}': {}", file.name, e);
                            }
                        }
                    } else {
                        let size = file.size
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);

                        let modified: DateTime<Utc> = file.modified_time.parse()
                            .unwrap_or_else(|_| Utc::now());

                        all_files.push(FileMetadata {
                            path: prefix.join(&file.name),
                            size,
                            modified,
                            hash: file.md5_checksum,
                            exists: true,
                        });
                    }
                }

                if file_list.next_page_token.is_none() {
                    break;
                }

                page_token = file_list.next_page_token;
            }

            Ok(all_files)
        })
    }

    /// Resolve a relative path to a DriveFile by walking the folder hierarchy.
    /// e.g. "subfolder/file.dwg" → find "subfolder" folder in root, then find "file.dwg" in it.
    async fn resolve_path(&self, path: &Path) -> Result<Option<DriveFile>> {
        let components: Vec<&str> = path.iter()
            .filter_map(|c| c.to_str())
            .collect();

        if components.is_empty() {
            return Ok(None);
        }

        let mut current_folder_id = self.folder_id.clone();

        // Walk through directory components (all except last)
        for &dir_name in &components[..components.len() - 1] {
            match self.get_item_by_name_in_folder(&current_folder_id, dir_name).await? {
                Some(folder) if folder.mime_type == "application/vnd.google-apps.folder" => {
                    current_folder_id = folder.id;
                }
                _ => return Ok(None), // Subfolder not found
            }
        }

        // Find the final file/folder in the resolved parent
        let file_name = components.last().unwrap();
        self.get_item_by_name_in_folder(&current_folder_id, file_name).await
    }

    /// Find a file or folder by name within a specific parent folder.
    async fn get_item_by_name_in_folder(&self, folder_id: &str, name: &str) -> Result<Option<DriveFile>> {
        let token = self.get_access_token().await?;

        let safe_folder_id = Self::escape_drive_query(folder_id);
        let safe_name = Self::escape_drive_query(name);
        let url = format!(
            "{}/files?q='{}'+in+parents+and+name='{}'+and+trashed=false&fields=files(id,name,mimeType,size,modifiedTime,md5Checksum)",
            DRIVE_API_BASE, safe_folder_id, safe_name
        );

        let response = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let file_list: FileList = response.json().await
            .map_err(|e| UvcadError::ProviderError(format!("Failed to parse response: {}", e)))?;

        Ok(file_list.files.into_iter().next())
    }

    /// Find the folder ID for a parent path, creating folders as needed for uploads.
    async fn resolve_or_create_parent_folder(&self, path: &Path) -> Result<String> {
        let components: Vec<&str> = path.iter()
            .filter_map(|c| c.to_str())
            .collect();

        let mut current_folder_id = self.folder_id.clone();

        // Walk/create each directory component (all except last, which is the filename)
        for &dir_name in &components[..components.len().saturating_sub(1)] {
            match self.get_item_by_name_in_folder(&current_folder_id, dir_name).await? {
                Some(folder) if folder.mime_type == "application/vnd.google-apps.folder" => {
                    current_folder_id = folder.id;
                }
                _ => {
                    // Create the subfolder
                    current_folder_id = self.create_folder(dir_name, &current_folder_id).await?;
                }
            }
        }

        Ok(current_folder_id)
    }

    /// Create a folder in Google Drive.
    async fn create_folder(&self, name: &str, parent_id: &str) -> Result<String> {
        let token = self.get_access_token().await?;

        let metadata = serde_json::json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder",
            "parents": [parent_id]
        });

        let url = format!("{}/files", DRIVE_API_BASE);

        let response = self.client
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .body(metadata.to_string())
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(UvcadError::ProviderError(format!(
                "Failed to create folder '{}': {} - {}", name, status, error_text
            )));
        }

        let file: DriveFile = response.json().await
            .map_err(|e| UvcadError::ProviderError(format!("Failed to parse response: {}", e)))?;

        tracing::info!("Created folder '{}' (ID: {})", name, file.id);
        Ok(file.id)
    }

    async fn list_files_in_folder(&self, folder_id: &str, page_token: Option<String>) -> Result<FileList> {
        let token = self.get_access_token().await?;

        let safe_folder_id = Self::escape_drive_query(folder_id);
        let mut url = format!(
            "{}/files?q='{}'+in+parents+and+trashed=false&fields=files(id,name,mimeType,size,modifiedTime,md5Checksum),nextPageToken",
            DRIVE_API_BASE, safe_folder_id
        );

        if let Some(pt) = page_token {
            url.push_str(&format!("&pageToken={}", pt));
        }

        let response = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(UvcadError::ProviderError(format!(
                "Failed to list files: {} - {}",
                status, error_text
            )));
        }

        let file_list: FileList = response.json().await
            .map_err(|e| UvcadError::ProviderError(format!("Failed to parse response: {}", e)))?;

        Ok(file_list)
    }

    async fn download_file_content(&self, file_id: &str) -> Result<Vec<u8>> {
        let token = self.get_access_token().await?;

        let url = format!("{}/files/{}?alt=media", DRIVE_API_BASE, file_id);

        let response = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(UvcadError::ProviderError(format!(
                "Failed to download file: {} - {}",
                status, error_text
            )));
        }

        let bytes = response.bytes().await
            .map_err(|e| UvcadError::NetworkError(e))?;

        Ok(bytes.to_vec())
    }

    async fn upload_file_to_folder(&self, name: &str, parent_id: &str, content: Vec<u8>) -> Result<String> {
        let token = self.get_access_token().await?;

        let metadata = FileMetadataUpload {
            name: name.to_string(),
            parents: vec![parent_id.to_string()],
        };

        // Use multipart upload
        let boundary = "===============boundary===============";
        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| UvcadError::SerializationError(e))?;

        let mut body = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
        body.extend_from_slice(metadata_json.as_bytes());
        body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(&content);
        body.extend_from_slice(format!("\r\n--{}--", boundary).as_bytes());

        let url = format!("{}/files?uploadType=multipart", DRIVE_UPLOAD_API);

        let response = self.client
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", format!("multipart/related; boundary={}", boundary))
            .body(body)
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(UvcadError::ProviderError(format!(
                "Failed to upload file: {} - {}",
                status, error_text
            )));
        }

        let file: DriveFile = response.json().await
            .map_err(|e| UvcadError::ProviderError(format!("Failed to parse response: {}", e)))?;

        Ok(file.id)
    }

    async fn update_file_content(&self, file_id: &str, content: Vec<u8>) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!("{}/files/{}?uploadType=media", DRIVE_UPLOAD_API, file_id);

        let response = self.client
            .patch(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/octet-stream")
            .body(content)
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(UvcadError::ProviderError(format!(
                "Failed to update file: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl StorageProvider for GoogleDriveProvider {
    fn name(&self) -> &str {
        "google_drive"
    }

    async fn list_files(&self, _path: &Path) -> Result<Vec<FileMetadata>> {
        self.list_files_recursive(&self.folder_id, Path::new("")).await
    }

    async fn get_metadata(&self, path: &Path) -> Result<Option<FileMetadata>> {
        if let Some(file) = self.resolve_path(path).await? {
            if file.mime_type == "application/vnd.google-apps.folder" {
                return Ok(None);
            }

            let size = file.size
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            let modified: DateTime<Utc> = file.modified_time.parse()
                .unwrap_or_else(|_| Utc::now());

            Ok(Some(FileMetadata {
                path: path.to_path_buf(),
                size,
                modified,
                hash: file.md5_checksum,
                exists: true,
            }))
        } else {
            Ok(None)
        }
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        Ok(self.get_metadata(path).await?.is_some())
    }

    async fn download(&self, path: &Path, dest: &Path) -> Result<PathBuf> {
        let file = self.resolve_path(path).await?
            .ok_or_else(|| UvcadError::FileNotFound { path: path.to_string_lossy().to_string() })?;

        let content = self.download_file_content(&file.id).await?;

        // Write to destination
        tokio::fs::write(dest, &content).await?;

        // Verify hash using MD5 (Google Drive's native hash algorithm)
        if let Some(expected_md5) = file.md5_checksum {
            let computed_md5 = file_hasher::compute_file_md5(dest)?;
            if !computed_md5.eq_ignore_ascii_case(&expected_md5) {
                return Err(UvcadError::SyncFailed(format!(
                    "Download integrity check failed for '{}': expected MD5 {}, got {}",
                    path.display(), expected_md5, computed_md5
                )));
            }
            tracing::debug!("Download integrity verified for '{}' (MD5: {})", path.display(), computed_md5);
        }

        Ok(dest.to_path_buf())
    }

    async fn upload(&self, source: &Path, dest: &Path) -> Result<()> {
        let name = dest.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| UvcadError::InvalidConfig("Invalid file path".to_string()))?;

        // Read file content
        let content = tokio::fs::read(source).await?;

        // Check if file already exists at this path
        if let Some(existing_file) = self.resolve_path(dest).await? {
            // Update existing file
            self.update_file_content(&existing_file.id, content).await?;
            tracing::info!("Updated existing file in Google Drive: {}", dest.display());
        } else {
            // Resolve or create parent folders, then upload
            let parent_id = self.resolve_or_create_parent_folder(dest).await?;
            let file_id = self.upload_file_to_folder(name, &parent_id, content).await?;
            tracing::info!("Uploaded new file to Google Drive: {} (ID: {})", dest.display(), file_id);
        }

        Ok(())
    }

    async fn delete(&self, path: &Path) -> Result<()> {
        let file = self.resolve_path(path).await?
            .ok_or_else(|| UvcadError::FileNotFound { path: path.to_string_lossy().to_string() })?;

        let token = self.get_access_token().await?;
        let url = format!("{}/files/{}", DRIVE_API_BASE, file.id);

        let response = self.client
            .delete(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| UvcadError::NetworkError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(UvcadError::ProviderError(format!(
                "Failed to delete file: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    async fn initialize(&mut self) -> Result<()> {
        // Check if we have valid credentials
        if !self.is_authenticated() {
            return Err(UvcadError::AuthenticationFailed(
                "Not authenticated with Google Drive".to_string()
            ));
        }
        Ok(())
    }

    async fn test_connection(&self) -> Result<bool> {
        if !self.is_authenticated() {
            return Ok(false);
        }

        // Try to list files to verify connection
        match self.list_files_in_folder(&self.folder_id, None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
