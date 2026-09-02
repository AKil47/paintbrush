use std::fmt::{self, Formatter};

use anyhow::Result;
use serde::Deserialize;

use crate::client::CanvasClient;
use crate::resource::{Loaded, Locator, Resource, ResourceManager, ResourceSpec};

#[derive(Deserialize)]
pub(crate) struct Announcement {
    id: i64,
    title: String,
    message: Option<String>,
    posted_at: Option<String>,
    published: bool,
    locked_for_user: bool,
    lock_explanation: Option<String>,
    html_url: String,
}

impl Resource for Announcement {
    fn fmt_row(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let posted = self.posted_at.as_deref().unwrap_or("not yet posted");
        let lock_state = if self.locked_for_user {
            "locked"
        } else {
            "unlocked"
        };
        write!(
            formatter,
            "{}\t{}\t{}\t{}",
            self.id, posted, lock_state, self.title
        )
    }

    fn fmt_detail(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let lock_state = if self.locked_for_user {
            "locked"
        } else {
            "unlocked"
        };
        writeln!(formatter, "id: {}", self.id)?;
        writeln!(formatter, "title: {}", self.title)?;
        writeln!(formatter, "published: {}", self.published)?;
        writeln!(formatter, "lock_state: {lock_state}")?;
        if let Some(explanation) = &self.lock_explanation {
            writeln!(formatter, "lock_explanation: {explanation}")?;
        }
        writeln!(
            formatter,
            "posted_at: {}",
            self.posted_at.as_deref().unwrap_or("not yet posted")
        )?;
        write!(formatter, "url: {}", self.html_url)?;
        if let Some(message) = &self.message {
            write!(formatter, "\n\nmessage:\n{message}")?;
        }
        Ok(())
    }
}

pub(crate) struct AnnouncementLocator {
    course_id: u64,
    announcement_id: u64,
}

impl Locator<Announcement, CanvasClient> for AnnouncementLocator {
    fn resolve(self, client: &CanvasClient) -> Result<Announcement> {
        client.get(
            &format!(
                "/courses/{}/discussion_topics/{}",
                self.course_id, self.announcement_id
            ),
            &Vec::new(),
        )
    }

    fn web_url(&self, client: &CanvasClient) -> Result<String> {
        Ok(client.web_url(&format!(
            "/courses/{}/discussion_topics/{}",
            self.course_id, self.announcement_id
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
    announcement_id: u64,
}

impl ViewArgs {
    pub(crate) fn new(course_id: u64, announcement_id: u64) -> Self {
        Self {
            course_id,
            announcement_id,
        }
    }
}

pub(crate) struct AnnouncementSpec;

impl ResourceSpec<Announcement> for AnnouncementSpec {
    type Client = CanvasClient;
    type ListArgs = ListArgs;
    type ViewArgs = ViewArgs;
    type ListedLocator = Loaded<AnnouncementLocator, Announcement>;
    type ViewLocator = AnnouncementLocator;
    type ListIter = Vec<Self::ListedLocator>;

    fn list(client: &CanvasClient, args: ListArgs) -> Result<Self::ListIter> {
        let query = vec![(
            "context_codes[]".into(),
            format!("course_{}", args.course_id),
        )];
        let announcements: Vec<Announcement> = client.get_all("/announcements", &query)?;

        Ok(announcements
            .into_iter()
            .filter(|announcement| announcement.published)
            .map(|announcement| {
                let locator = AnnouncementLocator {
                    course_id: args.course_id,
                    announcement_id: announcement.id as u64,
                };
                Loaded::new(locator, announcement)
            })
            .collect())
    }

    fn locate(args: ViewArgs) -> AnnouncementLocator {
        AnnouncementLocator {
            course_id: args.course_id,
            announcement_id: args.announcement_id,
        }
    }
}

pub(crate) type Manager = ResourceManager<AnnouncementSpec, Announcement>;
