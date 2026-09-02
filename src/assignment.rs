use std::fmt::{self, Formatter};

use anyhow::Result;
use serde::Deserialize;

use crate::client::CanvasClient;
use crate::resource::{Loaded, Locator, Resource, ResourceManager, ResourceSpec};

#[derive(Deserialize)]
pub(crate) struct Assignment {
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
    submission: Option<Submission>,
}

#[derive(Deserialize)]
struct Submission {
    workflow_state: String,
    submitted_at: Option<String>,
}

impl Resource for Assignment {
    fn fmt_row(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let due = self.due_at.as_deref().unwrap_or("no due date");
        let points = self
            .points_possible
            .map(|points| points.to_string())
            .unwrap_or_else(|| "-".to_string());
        let lock_state = if self.locked_for_user {
            "locked"
        } else {
            "unlocked"
        };
        let submission_state = self
            .submission
            .as_ref()
            .map(|submission| submission.workflow_state.as_str())
            .unwrap_or("unsubmitted");

        write!(
            formatter,
            "{}\t{}\t{}\t{}\t{}\t{}",
            self.id, due, lock_state, submission_state, points, self.name
        )
    }

    fn fmt_detail(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let lock_state = if self.locked_for_user {
            "locked"
        } else {
            "unlocked"
        };
        writeln!(formatter, "id: {}", self.id)?;
        writeln!(formatter, "name: {}", self.name)?;
        writeln!(formatter, "published: {}", self.published)?;
        writeln!(formatter, "lock_state: {lock_state}")?;
        if let Some(explanation) = &self.lock_explanation {
            writeln!(formatter, "lock_explanation: {explanation}")?;
        }
        writeln!(
            formatter,
            "due_at: {}",
            self.due_at.as_deref().unwrap_or("-")
        )?;
        writeln!(
            formatter,
            "lock_at: {}",
            self.lock_at.as_deref().unwrap_or("-")
        )?;
        writeln!(
            formatter,
            "unlock_at: {}",
            self.unlock_at.as_deref().unwrap_or("-")
        )?;
        writeln!(
            formatter,
            "points_possible: {}",
            self.points_possible
                .map(|points| points.to_string())
                .unwrap_or_else(|| "-".to_string())
        )?;
        writeln!(formatter, "grading_type: {}", self.grading_type)?;
        writeln!(
            formatter,
            "submission_types: {}",
            self.submission_types.join(", ")
        )?;
        writeln!(
            formatter,
            "allowed_attempts: {}",
            self.allowed_attempts
                .map(|attempts| if attempts == -1 {
                    "unlimited".to_string()
                } else {
                    attempts.to_string()
                })
                .unwrap_or_else(|| "-".to_string())
        )?;
        writeln!(formatter, "url: {}", self.html_url)?;
        match &self.submission {
            Some(submission) => {
                writeln!(formatter, "submission_state: {}", submission.workflow_state)?;
                write!(
                    formatter,
                    "submitted_at: {}",
                    submission.submitted_at.as_deref().unwrap_or("-")
                )?;
            }
            None => write!(formatter, "submission_state: unsubmitted")?,
        }
        if let Some(description) = &self.description {
            write!(formatter, "\n\ndescription:\n{description}")?;
        }
        Ok(())
    }
}

pub(crate) struct AssignmentLocator {
    course_id: u64,
    assignment_id: u64,
}

impl Locator<Assignment, CanvasClient> for AssignmentLocator {
    fn resolve(self, client: &CanvasClient) -> Result<Assignment> {
        client.get(
            &format!(
                "/courses/{}/assignments/{}",
                self.course_id, self.assignment_id
            ),
            &vec![("include[]".into(), "submission".into())],
        )
    }

    fn web_url(&self, client: &CanvasClient) -> Result<String> {
        Ok(client.web_url(&format!(
            "/courses/{}/assignments/{}",
            self.course_id, self.assignment_id
        )))
    }
}

pub(crate) struct ListArgs {
    course_id: u64,
}

impl ListArgs {
    pub(crate) fn new(course_id: u64) -> Self {
        Self { course_id }
    }
}

pub(crate) struct ViewArgs {
    course_id: u64,
    assignment_id: u64,
}

impl ViewArgs {
    pub(crate) fn new(course_id: u64, assignment_id: u64) -> Self {
        Self {
            course_id,
            assignment_id,
        }
    }
}

pub(crate) struct AssignmentSpec;

impl ResourceSpec<Assignment> for AssignmentSpec {
    type Client = CanvasClient;
    type ListArgs = ListArgs;
    type ViewArgs = ViewArgs;
    type ListedLocator = Loaded<AssignmentLocator, Assignment>;
    type ViewLocator = AssignmentLocator;
    type ListIter = Vec<Self::ListedLocator>;

    fn list(client: &CanvasClient, args: ListArgs) -> Result<Self::ListIter> {
        let query = vec![("include[]".into(), "submission".into())];
        let assignments: Vec<Assignment> =
            client.get_all(&format!("/courses/{}/assignments", args.course_id), &query)?;

        Ok(assignments
            .into_iter()
            .filter(|assignment| assignment.published)
            .map(|assignment| {
                let locator = AssignmentLocator {
                    course_id: args.course_id,
                    assignment_id: assignment.id as u64,
                };
                Loaded::new(locator, assignment)
            })
            .collect())
    }

    fn locate(args: ViewArgs) -> AssignmentLocator {
        AssignmentLocator {
            course_id: args.course_id,
            assignment_id: args.assignment_id,
        }
    }
}

pub(crate) type Manager = ResourceManager<AssignmentSpec, Assignment>;
