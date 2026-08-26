use anyhow::{Context, Result};

use crate::{auth, profile};

/// Prints the rendered HTML of the Canvas page at `url`, or opens it in the
/// browser if `web` is set, for the selected profile.
pub fn view(profile_arg: Option<&str>, url: &str, web: bool) -> Result<()> {
    if web {
        println!("Opening {url} in your browser...");
        return open::that(url).context("failed to open browser");
    }

    let profile = profile::resolve(profile_arg)?;
    let html = auth::fetch_html(&profile, url)?;
    println!("{html}");
    Ok(())
}
