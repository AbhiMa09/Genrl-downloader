//! Credentials live in the system keyring — never in the database, a file,
//! a log line or an export.
//!
//! The design is explicit about this: *"MAL and qBittorrent credentials go in
//! the system keyring (libsecret), never in plain files or logs."* For
//! headless and scripted runs an environment variable (e.g.
//! `GENRL_QBIT_PASSWORD`) takes precedence — an override, not a store, so a
//! script can supply the secret without touching the keyring at all.

use crate::error::{Error, Result};

/// Every secret the app holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// qBittorrent WebUI password.
    QbitPassword,
    /// MAL OAuth refresh token.
    MalRefreshToken,
}

impl Secret {
    /// Account name inside the keyring service.
    pub fn key(self) -> &'static str {
        match self {
            Secret::QbitPassword => "qbit_password",
            Secret::MalRefreshToken => "mal_refresh_token",
        }
    }

    /// Environment override for scripted or headless use.
    pub fn env_var(self) -> &'static str {
        match self {
            Secret::QbitPassword => "GENRL_QBIT_PASSWORD",
            Secret::MalRefreshToken => "GENRL_MAL_REFRESH_TOKEN",
        }
    }

    /// How to name it in a prompt or an error message.
    pub fn label(self) -> &'static str {
        match self {
            Secret::QbitPassword => "qBittorrent WebUI password",
            Secret::MalRefreshToken => "MAL refresh token",
        }
    }
}

/// One namespace for everything this app stores in the keyring.
const SERVICE: &str = "genrl-downloader";

/// Read a secret: the environment override first, then the keyring.
pub fn get(secret: Secret) -> Result<Option<String>> {
    if let Ok(value) = std::env::var(secret.env_var()) {
        if !value.is_empty() {
            return Ok(Some(value));
        }
    }
    let entry =
        keyring::Entry::new(SERVICE, secret.key()).map_err(|err| keyring_error(secret, err))?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(keyring_error(secret, err)),
    }
}

/// Store a secret in the keyring. Environment overrides are read, never
/// written back: they are supplied per run, not persisted.
pub fn set(secret: Secret, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::Other(format!("{} cannot be empty", secret.label())));
    }
    let entry =
        keyring::Entry::new(SERVICE, secret.key()).map_err(|err| keyring_error(secret, err))?;
    entry
        .set_password(value)
        .map_err(|err| keyring_error(secret, err))?;
    Ok(())
}

/// Forget a stored secret (revoke, rotate).
pub fn delete(secret: Secret) -> Result<()> {
    let entry =
        keyring::Entry::new(SERVICE, secret.key()).map_err(|err| keyring_error(secret, err))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(keyring_error(secret, err)),
    }
}

/// Every keyring failure carries the environment fallback, because the usual
/// cause is a locked or absent session keyring on a minimal distro.
fn keyring_error(secret: Secret, err: keyring::Error) -> Error {
    Error::Other(format!(
        "keyring unavailable while reading {} ({err}); unlock your desktop keyring or set {} for this run",
        secret.label(),
        secret.env_var()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_override_wins_for_this_run() {
        // The only test that touches the process environment; no other test
        // uses this variable, so parallelism is safe.
        std::env::set_var("GENRL_QBIT_PASSWORD", "from-env");
        let value = get(Secret::QbitPassword).unwrap();
        assert_eq!(value.as_deref(), Some("from-env"));
        std::env::remove_var("GENRL_QBIT_PASSWORD");
    }

    #[test]
    fn empty_means_unset_not_a_blank_password() {
        std::env::set_var("GENRL_MAL_REFRESH_TOKEN", "");
        // An empty override falls through to the keyring; on a machine with a
        // keyring this returns whatever is stored (or None), never Some("").
        if let Ok(Some(value)) = get(Secret::MalRefreshToken) {
            assert!(!value.is_empty());
        }
        std::env::remove_var("GENRL_MAL_REFRESH_TOKEN");
    }

    #[test]
    fn secrets_have_distinct_identities() {
        assert_ne!(Secret::QbitPassword.key(), Secret::MalRefreshToken.key());
        assert_ne!(Secret::QbitPassword.env_var(), Secret::MalRefreshToken.env_var());
        assert_eq!(Secret::QbitPassword.key(), "qbit_password");
    }

    #[test]
    fn refusing_to_store_empty_secrets() {
        assert!(set(Secret::QbitPassword, "").is_err());
    }

    /// Round-trips through the *real* session keyring (libsecret). Ignored by
    /// default because CI has no Secret Service; run it on a desktop box:
    ///
    /// ```text
    /// cargo test -p genrl-core keyring_round_trip -- --ignored
    /// ```
    #[test]
    #[ignore = "needs a running Secret Service"]
    fn keyring_round_trip() {
        let marker = format!("round-trip-{}", std::process::id());
        set(Secret::MalRefreshToken, &marker).expect("keyring is writable");
        assert_eq!(
            get(Secret::MalRefreshToken).unwrap().as_deref(),
            Some(marker.as_str()),
            "the keyring returns what was stored"
        );
        delete(Secret::MalRefreshToken).expect("keyring delete works");
        // Gone (or back to whatever was there before): never our marker.
        assert_ne!(
            get(Secret::MalRefreshToken).unwrap().as_deref(),
            Some(marker.as_str())
        );
    }
}
