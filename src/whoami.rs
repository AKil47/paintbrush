use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{auth, profile};

#[derive(Deserialize)]
struct User {
    id: i64,
    name: String,
}

/// Fetches and prints the logged-in user for `profile_arg` (or the default
/// profile), proving the stored token actually works against the Canvas API.
pub fn whoami(profile_arg: Option<&str>) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    let user: User = ureq::get(&format!("https://{}/api/v1/users/self", session.domain))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .context("request to /api/v1/users/self failed")?
        .into_json()
        .context("unexpected response body from /api/v1/users/self")?;

    println!("Logged in as {} (id: {})", user.name, user.id);
    Ok(())
}
