//! Provider API keys live only in the OS credential store (Windows Credential
//! Manager through `keyring`'s `windows-native` backend). SQLite settings hold
//! the reference string, never the secret.

use anyhow::{anyhow, Result};

pub const KEYRING_SERVICE: &str = "VoicePartner";
const MAX_SECRET_CHARS: usize = 1024;

/// A secret that cannot be printed, serialised or logged by accident.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

pub trait CredentialStore: Send + Sync {
    fn get(&self, reference: &str) -> Result<Option<Secret>>;
    fn set(&self, reference: &str, secret: &str) -> Result<()>;
    fn delete(&self, reference: &str) -> Result<()>;
}

pub struct SystemCredentialStore;

impl SystemCredentialStore {
    fn entry(reference: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, reference)
            .map_err(|e| anyhow!("Credential store unavailable: {}", e))
    }
}

impl CredentialStore for SystemCredentialStore {
    fn get(&self, reference: &str) -> Result<Option<Secret>> {
        match Self::entry(reference)?.get_password() {
            Ok(secret) => Ok(Some(Secret::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("Credential store read failed: {}", e)),
        }
    }

    fn set(&self, reference: &str, secret: &str) -> Result<()> {
        Self::entry(reference)?
            .set_password(secret)
            .map_err(|e| anyhow!("Credential store write failed: {}", e))
    }

    fn delete(&self, reference: &str) -> Result<()> {
        match Self::entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow!("Credential store delete failed: {}", e)),
        }
    }
}

/// Trim and reject values that could not be a header-safe API key.
pub fn validate_secret(raw: &str) -> Result<String, String> {
    let secret = raw.trim();
    if secret.is_empty() {
        return Err("API key is empty".into());
    }
    if secret.chars().count() > MAX_SECRET_CHARS {
        return Err("API key is too long".into());
    }
    if secret
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || !c.is_ascii())
    {
        return Err("API key contains unsupported characters".into());
    }
    Ok(secret.to_string())
}

#[cfg(test)]
#[derive(Default)]
pub struct MemoryCredentialStore {
    entries: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
impl MemoryCredentialStore {
    pub fn references(&self) -> Vec<String> {
        self.entries.lock().unwrap().keys().cloned().collect()
    }
}

#[cfg(test)]
impl CredentialStore for MemoryCredentialStore {
    fn get(&self, reference: &str) -> Result<Option<Secret>> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .get(reference)
            .cloned()
            .map(Secret::new))
    }

    fn set(&self, reference: &str, secret: &str) -> Result<()> {
        self.entries
            .lock()
            .unwrap()
            .insert(reference.into(), secret.into());
        Ok(())
    }

    fn delete(&self, reference: &str) -> Result<()> {
        self.entries.lock().unwrap().remove(reference);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_never_print() {
        let secret = Secret::new("sk-live-value".into());
        assert_eq!(format!("{:?}", secret), "Secret([redacted])");
    }

    /// Guards against the keyring crate's silent mock fallback. Building an
    /// entry does not read or write Credential Manager.
    #[cfg(windows)]
    #[test]
    fn system_store_uses_windows_credential_manager() {
        let entry = SystemCredentialStore::entry("backend-check@localhost").unwrap();
        let credential = entry
            .get_credential()
            .downcast_ref::<keyring::windows::WinCredential>()
            .expect("keyring must use the Windows credential backend");
        assert!(credential.target_name.contains(KEYRING_SERVICE));
    }

    #[test]
    fn validation_trims_and_rejects_header_injection() {
        assert_eq!(validate_secret("  sk-abc  ").unwrap(), "sk-abc");
        assert!(validate_secret("").is_err());
        assert!(validate_secret("sk-abc\r\nX-Evil: 1").is_err());
        assert!(validate_secret("sk abc").is_err());
    }
}
