use crate::utils::error::Result;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const BUFFER_SIZE: usize = 8192;

/// Compute SHA-256 hash of a file
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// Compute MD5 hash of a file (for Google Drive compatibility)
pub fn compute_file_md5(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut context = md5::Context::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.consume(&buffer[..count]);
    }

    let digest = context.compute();
    Ok(format!("{:x}", digest))
}

/// Compute SHA-256 hash of bytes
pub fn compute_bytes_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Verify file hash matches expected hash
pub fn verify_file_hash(path: &Path, expected_hash: &str) -> Result<bool> {
    let actual_hash = compute_file_hash(path)?;
    Ok(actual_hash.eq_ignore_ascii_case(expected_hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_file_hash() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "test content").unwrap();

        let hash = compute_file_hash(temp_file.path()).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_compute_bytes_hash() {
        let data = b"test content";
        let hash = compute_bytes_hash(data);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_verify_file_hash() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "test content").unwrap();

        let hash = compute_file_hash(temp_file.path()).unwrap();
        assert!(verify_file_hash(temp_file.path(), &hash).unwrap());
        assert!(!verify_file_hash(temp_file.path(), "wrong_hash").unwrap());
    }
}
