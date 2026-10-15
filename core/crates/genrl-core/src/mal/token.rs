use std::sync::Mutex;

use crate::error::Result;
use crate::secrets::{self, Secret};

/// A working access token plus its expiry.
#[derive(Debug, Clone, PartialEq)]
pub struct Tokens {
    pub access_token: String,
    /// Unix seconds; refreshed [`super::pkce::REFRESH_MARGIN_SECS`] before this.
    pub expires_at: i64,
}

/// Where the long-lived refresh token lives: the keyring in the app, memory
/// in tests.
pub trait TokenStore: Send + Sync {
    fn refresh_token(&self) -> Result<Option<String>>;
    fn set_refresh_token(&self, token: Option<&str>) -> Result<()>;
    fn client_secret(&self) -> Result<Option<String>> {
        Ok(None)
    }
    fn set_client_secret(&self, secret: Option<&str>) -> Result<()> {
        let _ = secret;
        Ok(())
    }
}

/// The app's store: [`crate::secrets`], i.e. the system keyring (libsecret)
/// with the `GENRL_MAL_REFRESH_TOKEN` environment override.
pub struct KeyringStore;

impl TokenStore for KeyringStore {
    fn refresh_token(&self) -> Result<Option<String>> {
        secrets::get(Secret::MalRefreshToken)
    }
    fn set_refresh_token(&self, token: Option<&str>) -> Result<()> {
        match token {
            Some(token) => secrets::set(Secret::MalRefreshToken, token),
            None => secrets::delete(Secret::MalRefreshToken),
        }
    }
    fn client_secret(&self) -> Result<Option<String>> {
        secrets::get(Secret::MalClientSecret)
    }
    fn set_client_secret(&self, secret: Option<&str>) -> Result<()> {
        match secret {
            Some(s) if !s.trim().is_empty() => secrets::set(Secret::MalClientSecret, s.trim()),
            _ => secrets::delete(Secret::MalClientSecret),
        }
    }
}

/// An in-memory store for tests and for store-agnostic callers.
#[derive(Default)]
pub struct MemoryStore {
    token: Mutex<Option<String>>,
    secret: Mutex<Option<String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with(token: &str) -> Self {
        Self {
            token: Mutex::new(Some(token.to_string())),
            secret: Mutex::new(None),
        }
    }
}

impl TokenStore for MemoryStore {
    fn refresh_token(&self) -> Result<Option<String>> {
        let guard = self.token.lock().unwrap_or_else(|e| e.into_inner());
        Ok(guard.clone())
    }
    fn set_refresh_token(&self, token: Option<&str>) -> Result<()> {
        let mut guard = self.token.lock().unwrap_or_else(|e| e.into_inner());
        *guard = token.map(str::to_string);
        Ok(())
    }
    fn client_secret(&self) -> Result<Option<String>> {
        let guard = self.secret.lock().unwrap_or_else(|e| e.into_inner());
        Ok(guard.clone())
    }
    fn set_client_secret(&self, secret: Option<&str>) -> Result<()> {
        let mut guard = self.secret.lock().unwrap_or_else(|e| e.into_inner());
        *guard = secret.map(str::to_string);
        Ok(())
    }
}
