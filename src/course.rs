use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{auth, profile};

#[derive(Deserialize)]
struct Course {
    id: i64,
    name: String,
    course_code: String,
    workflow_state: String,
    default_view: Option<String>,
    start_at: Option<String>,
    end_at: Option<String>,
}

/// Lists the logged-in user's courses for `profile_arg` (or the default
/// profile). Each course's `id` is what future commands (e.g. listing
/// assignments) will take to reference it.
pub fn list(profile_arg: Option<&str>) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    let courses: Vec<Course> = ureq::get(&format!("https://{}/api/v1/courses", session.domain))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .query("per_page", "100")
        .call()
        .context("request to /api/v1/courses failed")?
        .into_json()
        .context("unexpected response body from /api/v1/courses")?;

    for course in courses {
        println!("{}\t{}\t{}", course.id, course.course_code, course.name);
    }

    Ok(())
}

/// Prints full details for one course, or opens it in the browser if `web`
/// is set.
pub fn view(profile_arg: Option<&str>, course_id: u64, web: bool) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    let url = format!("https://{}/courses/{course_id}", session.domain);
    if web {
        println!("Opening {url} in your browser...");
        return open::that(&url).context("failed to open browser");
    }

    let course: Course = ureq::get(&format!(
        "https://{}/api/v1/courses/{course_id}",
        session.domain
    ))
    .set("Authorization", &format!("Bearer {}", session.access_token))
    .call()
    .context("request to the course endpoint failed")?
    .into_json()
    .context("unexpected response body from the course endpoint")?;

    println!("id: {}", course.id);
    println!("name: {}", course.name);
    println!("course_code: {}", course.course_code);
    println!("workflow_state: {}", course.workflow_state);
    println!(
        "default_view: {}",
        course.default_view.as_deref().unwrap_or("-")
    );
    println!("start_at: {}", course.start_at.as_deref().unwrap_or("-"));
    println!("end_at: {}", course.end_at.as_deref().unwrap_or("-"));
    println!("url: {url}");

    Ok(())
}
