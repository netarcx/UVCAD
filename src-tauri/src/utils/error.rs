use thiserror::Error;

#[derive(Error, Debug)]
pub enum UvcadError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("File conflict detected: {path}")]
    ConflictDetected { path: String },

    #[error("SMB share not accessible: {0}")]
    SmbNotAccessible(String),

    #[error("Hash mismatch for file: {path}")]
    HashMismatch { path: String },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("Token storage error: {0}")]
    TokenStorageError(#[from] keyring::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Sync failed: {0}")]
    SyncFailed(String),
}

pub type Result<T> = std::result::Result<T, UvcadError>;

// Implement conversion to String for Tauri command results
impl From<UvcadError> for String {
    fn from(error: UvcadError) -> Self {
        error.to_string()
    }
}
