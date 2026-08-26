use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::auth;

/// The set of known profile names and which one is the default, persisted
/// as JSON. Profiles' actual credentials live in the OS keychain
/// (`auth::store`), keyed by the same profile name — this file only tracks
/// names and the default, so listing/removing profiles doesn't require
/// touching secrets unnecessarily.
#[derive(Serialize, Deserialize, Default)]
struct ProfilesFile {
    default: Option<String>,
    profiles: BTreeSet<String>,
}

fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "paintbrush")
        .context("couldn't determine a config directory for this platform")?;
    Ok(dirs.config_dir().join("profiles.json"))
}

fn load() -> Result<ProfilesFile> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(ProfilesFile::default());
    }
    let json = fs::read_to_string(&path).context("failed to read profiles config")?;
    serde_json::from_str(&json).context("profiles config is corrupt")
}

fn save(file: &ProfilesFile) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }
    let json = serde_json::to_string_pretty(file)?;
    fs::write(&path, json).context("failed to write profiles config")
}

/// Registers `name` as a known profile, if not already present. The first
/// profile ever registered becomes the default.
pub fn register(name: &str) -> Result<()> {
    let mut file = load()?;
    if file.profiles.insert(name.to_string()) && file.default.is_none() {
        file.default = Some(name.to_string());
    }
    save(&file)
}

/// Resolves the profile to use for a command: `explicit` if given
/// (`--profile`), otherwise the configured default.
pub fn resolve(explicit: Option<&str>) -> Result<String> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }
    load()?.default.ok_or_else(|| {
        anyhow!(
            "no default profile set and no --profile given; run `paintbrush login --domain <domain>` \
             or `paintbrush profile default <name>`"
        )
    })
}

/// Prints all configured profiles, marking the default with `*`.
pub fn list() -> Result<()> {
    let file = load()?;
    if file.profiles.is_empty() {
        println!("No profiles configured; run `paintbrush login --domain <domain>`.");
        return Ok(());
    }
    for name in &file.profiles {
        let marker = if file.default.as_deref() == Some(name.as_str()) {
            "*"
        } else {
            " "
        };
        let domain = auth::domain_for(name)?.unwrap_or_else(|| "?".to_string());
        println!("{marker} {name}\t{domain}");
    }
    Ok(())
}

/// Removes a profile and its stored credentials.
pub fn remove(name: &str) -> Result<()> {
    let mut file = load()?;
    if !file.profiles.remove(name) {
        return Err(anyhow!("no such profile: {name}"));
    }
    if file.default.as_deref() == Some(name) {
        file.default = None;
    }
    save(&file)?;
    auth::forget(name)?;
    println!("Removed profile '{name}'.");
    Ok(())
}

/// Sets the default profile used when `--profile` isn't passed.
pub fn set_default(name: &str) -> Result<()> {
    let mut file = load()?;
    if !file.profiles.contains(name) {
        return Err(anyhow!("no such profile: {name}"));
    }
    file.default = Some(name.to_string());
    save(&file)?;
    println!("Default profile set to '{name}'.");
    Ok(())
}
