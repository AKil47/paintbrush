use anyhow::Result;
use serde::Deserialize;

use crate::client::{CanvasClient, Query};

#[derive(Deserialize)]
struct User {
    id: i64,
    name: String,
}

/// Fetches and prints the logged-in user for `profile_arg` (or the default
/// profile), proving the stored token actually works against the Canvas API.
pub fn whoami(profile_arg: Option<&str>) -> Result<()> {
    let client = CanvasClient::connect(profile_arg)?;
    let user: User = client.get("/users/self", &Query::new())?;

    println!("Logged in as {} (id: {})", user.name, user.id);
    Ok(())
}
