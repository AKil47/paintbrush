use std::fmt::{self, Formatter};

use anyhow::Result;
use serde::Deserialize;

use crate::client::{CanvasClient, Query};
use crate::resource::{Loaded, Locator, Resource, ResourceManager, ResourceSpec};

#[derive(Deserialize)]
pub(crate) struct Course {
    id: i64,
    name: String,
    course_code: String,
    workflow_state: String,
    default_view: Option<String>,
    start_at: Option<String>,
    end_at: Option<String>,
    #[serde(skip)]
    html_url: String,
}

impl Resource for Course {
    fn fmt_row(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}\t{}\t{}",
            self.id, self.course_code, self.name
        )
    }

    fn fmt_detail(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "id: {}", self.id)?;
        writeln!(formatter, "name: {}", self.name)?;
        writeln!(formatter, "course_code: {}", self.course_code)?;
        writeln!(formatter, "workflow_state: {}", self.workflow_state)?;
        writeln!(
            formatter,
            "default_view: {}",
            self.default_view.as_deref().unwrap_or("-")
        )?;
        writeln!(
            formatter,
            "start_at: {}",
            self.start_at.as_deref().unwrap_or("-")
        )?;
        writeln!(
            formatter,
            "end_at: {}",
            self.end_at.as_deref().unwrap_or("-")
        )?;
        write!(formatter, "url: {}", self.html_url)
    }
}

pub(crate) struct CourseLocator {
    id: u64,
}

impl Locator<Course, CanvasClient> for CourseLocator {
    fn resolve(self, client: &CanvasClient) -> Result<Course> {
        let mut course: Course = client.get(&format!("/courses/{}", self.id), &Query::new())?;
        course.html_url = client.web_url(&format!("/courses/{}", self.id));
        Ok(course)
    }

    fn web_url(&self, client: &CanvasClient) -> Result<String> {
        Ok(client.web_url(&format!("/courses/{}", self.id)))
    }
}

pub(crate) struct ListArgs;

pub(crate) struct ViewArgs {
    id: u64,
}

impl ViewArgs {
    pub(crate) fn new(id: u64) -> Self {
        Self { id }
    }
}

pub(crate) struct CourseSpec;

impl ResourceSpec<Course> for CourseSpec {
    type Client = CanvasClient;
    type ListArgs = ListArgs;
    type ViewArgs = ViewArgs;
    type ListedLocator = Loaded<CourseLocator, Course>;
    type ViewLocator = CourseLocator;
    type ListIter = Vec<Self::ListedLocator>;

    fn list(client: &CanvasClient, _args: ListArgs) -> Result<Self::ListIter> {
        let courses: Vec<Course> = client.get_all("/courses", &Query::new())?;
        Ok(courses
            .into_iter()
            .map(|mut course| {
                let locator = CourseLocator {
                    id: course.id as u64,
                };
                course.html_url = client.web_url(&format!("/courses/{}", course.id));
                Loaded::new(locator, course)
            })
            .collect())
    }

    fn locate(args: ViewArgs) -> CourseLocator {
        CourseLocator { id: args.id }
    }
}

pub(crate) type Manager = ResourceManager<CourseSpec, Course>;

#[cfg(test)]
mod tests {
    use super::Course;
    use crate::resource::Resource;

    #[test]
    fn renders_rows_and_details() {
        let course = Course {
            id: 42,
            name: "Rust 101".into(),
            course_code: "RS101".into(),
            workflow_state: "available".into(),
            default_view: Some("modules".into()),
            start_at: None,
            end_at: None,
            html_url: "https://canvas.example/courses/42".into(),
        };

        assert_eq!(course.row().to_string(), "42\tRS101\tRust 101");
        assert!(
            course
                .detail()
                .to_string()
                .contains("workflow_state: available")
        );
    }
}
