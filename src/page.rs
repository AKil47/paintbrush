use anyhow::{Context, Result};
use serde::Deserialize;

use crate::client::{CanvasClient, Query};

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
    match (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) {
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

    let client = CanvasClient::connect(profile_arg)?;

    if let Some((course_id, page_url)) = wiki_page_ids(url) {
        let page: WikiPage = client.get(
            &format!("/courses/{course_id}/pages/{page_url}"),
            &Query::new(),
        )?;

        println!("title: {}", page.title);
        println!("\n{}", page.body.as_deref().unwrap_or(""));
        return Ok(());
    }

    let html = client.fetch_html(url)?;
    println!("{html}");
    Ok(())
}
