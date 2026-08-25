use anyhow::{Context, Result};
use serde::Deserialize;

use crate::auth;

#[derive(Deserialize)]
struct User {
    id: i64,
    name: String,
}

/// Fetches and prints the logged-in user for `domain`, proving the stored
/// token actually works against the Canvas API.
pub fn whoami(domain: &str) -> Result<()> {
    let token = auth::ensure_valid_token(domain)?;

    let user: User = ureq::get(&format!("https://{domain}/api/v1/users/self"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .context("request to /api/v1/users/self failed")?
        .into_json()
        .context("unexpected response body from /api/v1/users/self")?;

    println!("Logged in as {} (id: {})", user.name, user.id);
    Ok(())
}
