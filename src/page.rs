use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{auth, profile};

#[derive(Deserialize)]
struct WikiPage {
    title: String,
    body: Option<String>,
}

/// If `url` addresses a Canvas wiki page (`/courses/:id/pages/:page_url`),
/// returns its `(course_id, page_url)`. Canvas only ever server-renders an
/// empty mount div for these (the real title/body is fetched and rendered
/// client-side by JS), so scraping the page's HTML gets you nothing useful —
/// the Pages API returns the actual content instead.
fn wiki_page_ids(url: &str) -> Option<(u64, &str)> {
    let path = url.split_once("://")?.1.split_once('/')?.1;
    let mut segments = path.trim_start_matches('/').split('/');
    match (segments.next(), segments.next(), segments.next(), segments.next()) {
        (Some("courses"), Some(course_id), Some("pages"), Some(page_url)) => {
            Some((course_id.parse().ok()?, page_url))
        }
        _ => None,
    }
}

/// Prints the Canvas page at `url`, or opens it in the browser if `web` is
/// set, for the selected profile. Wiki pages are fetched via the Pages API
/// (their real content); everything else falls back to printing whatever
/// HTML Canvas's web session serves for that URL.
pub fn view(profile_arg: Option<&str>, url: &str, web: bool) -> Result<()> {
    if web {
        println!("Opening {url} in your browser...");
        return open::that(url).context("failed to open browser");
    }

    let profile = profile::resolve(profile_arg)?;

    if let Some((course_id, page_url)) = wiki_page_ids(url) {
        let session = auth::ensure_valid_token(&profile)?;
        let page: WikiPage = ureq::get(&format!(
            "https://{}/api/v1/courses/{course_id}/pages/{page_url}",
            session.domain
        ))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .context("request to the page endpoint failed")?
        .into_json()
        .context("unexpected response body from the page endpoint")?;

        println!("title: {}", page.title);
        println!("\n{}", page.body.as_deref().unwrap_or(""));
        return Ok(());
    }

    let html = auth::fetch_html(&profile, url)?;
    println!("{html}");
    Ok(())
}
