use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{auth, profile};

#[derive(Deserialize)]
struct Assignment {
    id: i64,
    name: String,
    description: Option<String>,
    due_at: Option<String>,
    lock_at: Option<String>,
    unlock_at: Option<String>,
    points_possible: Option<f64>,
    grading_type: String,
    submission_types: Vec<String>,
    allowed_attempts: Option<i64>,
    published: bool,
    locked_for_user: bool,
    lock_explanation: Option<String>,
    html_url: String,
}

/// Lists the published assignments in `course_id`, for `profile_arg` (or
/// the default profile). Each line notes whether the assignment is
/// currently locked for the user (e.g. not yet available).
pub fn list(profile_arg: Option<&str>, course_id: u64) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    let assignments: Vec<Assignment> = ureq::get(&format!(
        "https://{}/api/v1/courses/{course_id}/assignments",
        session.domain
    ))
    .set("Authorization", &format!("Bearer {}", session.access_token))
    .query("per_page", "100")
    .call()
    .context("request to the assignments endpoint failed")?
    .into_json()
    .context("unexpected response body from the assignments endpoint")?;

    for assignment in assignments.into_iter().filter(|a| a.published) {
        let due = assignment.due_at.as_deref().unwrap_or("no due date");
        let points = assignment
            .points_possible
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        let lock_state = if assignment.locked_for_user {
            "locked"
        } else {
            "unlocked"
        };
        println!(
            "{}\t{}\t{}\t{}\t{}",
            assignment.id, due, lock_state, points, assignment.name
        );
    }

    Ok(())
}

/// Prints full details for one assignment, or opens it in the browser if
/// `web` is set.
pub fn view(profile_arg: Option<&str>, course_id: u64, assignment_id: u64, web: bool) -> Result<()> {
    let profile = profile::resolve(profile_arg)?;
    let session = auth::ensure_valid_token(&profile)?;

    if web {
        let url = format!(
            "https://{}/courses/{course_id}/assignments/{assignment_id}",
            session.domain
        );
        println!("Opening {url} in your browser...");
        return open::that(&url).context("failed to open browser");
    }

    let assignment: Assignment = ureq::get(&format!(
        "https://{}/api/v1/courses/{course_id}/assignments/{assignment_id}",
        session.domain
    ))
    .set("Authorization", &format!("Bearer {}", session.access_token))
    .call()
    .context("request to the assignment endpoint failed")?
    .into_json()
    .context("unexpected response body from the assignment endpoint")?;

    let lock_state = if assignment.locked_for_user {
        "locked"
    } else {
        "unlocked"
    };

    println!("id: {}", assignment.id);
    println!("name: {}", assignment.name);
    println!("published: {}", assignment.published);
    println!("lock_state: {lock_state}");
    if let Some(explanation) = &assignment.lock_explanation {
        println!("lock_explanation: {explanation}");
    }
    println!("due_at: {}", assignment.due_at.as_deref().unwrap_or("-"));
    println!("lock_at: {}", assignment.lock_at.as_deref().unwrap_or("-"));
    println!(
        "unlock_at: {}",
        assignment.unlock_at.as_deref().unwrap_or("-")
    );
    println!(
        "points_possible: {}",
        assignment
            .points_possible
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!("grading_type: {}", assignment.grading_type);
    println!(
        "submission_types: {}",
        assignment.submission_types.join(", ")
    );
    println!(
        "allowed_attempts: {}",
        assignment
            .allowed_attempts
            .map(|a| if a == -1 {
                "unlimited".to_string()
            } else {
                a.to_string()
            })
            .unwrap_or_else(|| "-".to_string())
    );
    println!("url: {}", assignment.html_url);
    if let Some(description) = &assignment.description {
        println!("\ndescription:\n{description}");
    }

    Ok(())
}
