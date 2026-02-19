use crate::utils::error::Result;
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "com.uvcad.app";

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

pub struct TokenManager {
    entry: Entry,
}

impl TokenManager {
    pub fn new(provider: &str) -> Result<Self> {
        let entry = Entry::new(SERVICE_NAME, provider)?;
        Ok(Self { entry })
    }

    pub fn store_tokens(&self, tokens: &OAuthTokens) -> Result<()> {
        let json = serde_json::to_string(tokens)?;
        self.entry.set_password(&json)?;
        Ok(())
    }

    pub fn get_tokens(&self) -> Result<OAuthTokens> {
        let json = self.entry.get_password()?;
        let tokens = serde_json::from_str(&json)?;
        Ok(tokens)
    }

    pub fn delete_tokens(&self) -> Result<()> {
        self.entry.delete_password()?;
        Ok(())
    }

    pub fn has_tokens(&self) -> bool {
        self.entry.get_password().is_ok()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

pub struct CredentialManager {
    entry: Entry,
}

impl CredentialManager {
    pub fn new(provider: &str) -> Result<Self> {
        let key = format!("{}_credentials", provider);
        let entry = Entry::new(SERVICE_NAME, &key)?;
        Ok(Self { entry })
    }

    pub fn store_credentials(&self, creds: &OAuthCredentials) -> Result<()> {
        let json = serde_json::to_string(creds)?;
        self.entry.set_password(&json)?;
        Ok(())
    }

    pub fn get_credentials(&self) -> Result<OAuthCredentials> {
        let json = self.entry.get_password()?;
        let creds = serde_json::from_str(&json)?;
        Ok(creds)
    }

    pub fn delete_credentials(&self) -> Result<()> {
        self.entry.delete_password()?;
        Ok(())
    }
}
