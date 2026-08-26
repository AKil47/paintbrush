use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{auth, profile};

#[derive(Deserialize)]
struct Announcement {
    id: i64,
    title: String,
    message: Option<String>,
    posted_at: Option<String>,
    published: bool,
    locked_for_user: bool,
    lock_explanation: Option<String>,
    html_url: String,
}

/// Lists the published announcements in `course_id`, for `profile_arg` (or
/// the default profile). Uses Canvas's default date range (posted in
/// roughly the last 14 days). Each line notes whether the announcement is
/// currently locked for the user (e.g. closed for comments).
pub fn list(profile_arg: Option<&str>, course_id: u64) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    let announcements: Vec<Announcement> =
        ureq::get(&format!("https://{}/api/v1/announcements", session.domain))
            .set("Authorization", &format!("Bearer {}", session.access_token))
            .query("context_codes[]", &format!("course_{course_id}"))
            .query("per_page", "100")
            .call()
            .context("request to /api/v1/announcements failed")?
            .into_json()
            .context("unexpected response body from /api/v1/announcements")?;

    for announcement in announcements.into_iter().filter(|a| a.published) {
        let posted = announcement.posted_at.as_deref().unwrap_or("not yet posted");
        let lock_state = if announcement.locked_for_user {
            "locked"
        } else {
            "unlocked"
        };
        println!(
            "{}\t{}\t{}\t{}",
            announcement.id, posted, lock_state, announcement.title
        );
    }

    Ok(())
}

/// Prints full details for one announcement, or opens it in the browser if
/// `web` is set.
pub fn view(
    profile_arg: Option<&str>,
    course_id: u64,
    announcement_id: u64,
    web: bool,
) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    if web {
        let url = format!(
            "https://{}/courses/{course_id}/discussion_topics/{announcement_id}",
            session.domain
        );
        println!("Opening {url} in your browser...");
        return open::that(&url).context("failed to open browser");
    }

    let announcement: Announcement = ureq::get(&format!(
        "https://{}/api/v1/courses/{course_id}/discussion_topics/{announcement_id}",
        session.domain
    ))
    .set("Authorization", &format!("Bearer {}", session.access_token))
    .call()
    .context("request to the announcement endpoint failed")?
    .into_json()
    .context("unexpected response body from the announcement endpoint")?;

    let lock_state = if announcement.locked_for_user {
        "locked"
    } else {
        "unlocked"
    };

    println!("id: {}", announcement.id);
    println!("title: {}", announcement.title);
    println!("published: {}", announcement.published);
    println!("lock_state: {lock_state}");
    if let Some(explanation) = &announcement.lock_explanation {
        println!("lock_explanation: {explanation}");
    }
    println!(
        "posted_at: {}",
        announcement.posted_at.as_deref().unwrap_or("not yet posted")
    );
    println!("url: {}", announcement.html_url);
    if let Some(message) = &announcement.message {
        println!("\nmessage:\n{message}");
    }

    Ok(())
}
