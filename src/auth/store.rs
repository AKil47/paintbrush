use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
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
        Ok(()) => {}
        Err(keyring::Error::NoEntry) => {}
        Err(e) => return Err(e).context("failed to delete credentials from OS keychain"),
    }
    delete_cookies(profile)
}

/// Session cookies (for `auth::fetch_html`'s web-session flow) are stored
/// separately from the rest of a profile's credentials, as a plain JSON
/// file rather than in the OS keychain: a full cookie jar routinely exceeds
/// the ~2560-byte blob limit Windows Credential Manager enforces per entry.
/// They're less sensitive than the refresh token itself (short-lived,
/// scoped to a single web session) and this mirrors how `profile.rs`
/// already stores non-secret config.
fn cookies_path(profile: &str) -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "paintbrush")
        .context("couldn't determine a config directory for this platform")?;
    Ok(dirs.config_dir().join(format!("{profile}.cookies.json")))
}

pub fn save_cookies(profile: &str, json: &str) -> Result<()> {
    let path = cookies_path(profile)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }
    fs::write(&path, json).context("failed to write session cookies")
}

pub fn load_cookies(profile: &str) -> Result<Option<String>> {
    let path = cookies_path(profile)?;
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .context("failed to read stored session cookies")
}

pub fn delete_cookies(profile: &str) -> Result<()> {
    let path = cookies_path(profile)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("failed to delete stored session cookies"),
    }
}
