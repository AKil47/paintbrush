use anyhow::{Context, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "paintbrush";

/// Credentials for one profile, persisted as a single JSON blob in the OS
/// keychain, keyed by profile name.
#[derive(Serialize, Deserialize)]
pub struct StoredCredentials {
    pub domain: String,
    /// "https", from mobile_verify — needed to hit the right token endpoint later.
    pub protocol: String,
    pub client_id: String,
    /// The borrowed Android app credentials; reused for refresh so we don't
    /// re-hit mobile_verify.
    pub client_secret: String,
    pub access_token: String,
    /// Canvas does not rotate this on refresh; stored once at login.
    pub refresh_token: String,
    /// Unix seconds, if Canvas reported an `expires_in` for this token.
    /// Observed to be absent for tokens issued through the borrowed Android
    /// client — treat `None` as "no known expiry" rather than guessing one.
    pub expires_at: Option<u64>,
}

pub fn save(profile: &str, credentials: &StoredCredentials) -> Result<()> {
    let entry = Entry::new(SERVICE, profile).context("failed to open OS keychain entry")?;
    let json = serde_json::to_string(credentials)?;
    entry
        .set_password(&json)
        .context("failed to save credentials to OS keychain")?;
    Ok(())
}

pub fn load(profile: &str) -> Result<Option<StoredCredentials>> {
    let entry = Entry::new(SERVICE, profile).context("failed to open OS keychain entry")?;
    match entry.get_password() {
        Ok(json) => Ok(Some(serde_json::from_str(&json).context(
            "stored credentials are corrupt; run `paintbrush login` again",
        )?)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("failed to read credentials from OS keychain"),
    }
}

pub fn delete(profile: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, profile).context("failed to open OS keychain entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("failed to delete credentials from OS keychain"),
    }
}
