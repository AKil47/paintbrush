use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use url::Url;

use crate::{auth, profile};

pub type Query = Vec<(String, String)>;

/// An authenticated client for ordinary Canvas API requests.
pub struct CanvasClient {
    profile: String,
    domain: String,
    access_token: String,
    agent: ureq::Agent,
}

impl CanvasClient {
    pub fn connect(profile_arg: Option<&str>) -> Result<Self> {
        let profile = profile::resolve(profile_arg)?;
        let session = auth::ensure_valid_token(&profile)?;

        Ok(Self {
            profile,
            domain: session.domain,
            access_token: session.access_token,
            agent: ureq::AgentBuilder::new().build(),
        })
    }

    pub fn get<T>(&self, path: &str, query: &Query) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.api_url(path);
        let response = self
            .add_query(self.authorized_get(&url), query)
            .call()
            .with_context(|| format!("GET {path} failed"))?;

        response
            .into_json()
            .with_context(|| format!("unexpected response body from GET {path}"))
    }

    /// Fetches every page in a Canvas collection, following opaque `next`
    /// links rather than assuming that a large `per_page` value is complete.
    pub fn get_all<T>(&self, path: &str, query: &Query) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut url = self.api_url(path);
        let mut first_page = true;
        let mut items = Vec::new();

        loop {
            let mut request = self.authorized_get(&url);
            if first_page {
                request = request.query("per_page", "100");
                request = self.add_query(request, query);
                first_page = false;
            }

            let response = request
                .call()
                .with_context(|| format!("GET {url} failed"))?;
            let next = response.header("Link").and_then(next_link);
            let mut page: Vec<T> = response
                .into_json()
                .with_context(|| format!("unexpected response body from GET {url}"))?;
            items.append(&mut page);

            match next {
                Some(next) => {
                    validate_canvas_url(&next, &self.domain)?;
                    url = next;
                }
                None => return Ok(items),
            }
        }
    }

    pub fn web_url(&self, path: &str) -> String {
        format!("https://{}{}", self.domain, normalize_path(path))
    }

    pub fn fetch_html(&self, url: &str) -> Result<String> {
        auth::fetch_html(&self.profile, url)
    }

    fn api_url(&self, path: &str) -> String {
        format!("https://{}/api/v1{}", self.domain, normalize_path(path))
    }

    fn authorized_get(&self, url: &str) -> ureq::Request {
        self.agent
            .get(url)
            .set("Authorization", &format!("Bearer {}", self.access_token))
    }

    fn add_query(&self, mut request: ureq::Request, query: &Query) -> ureq::Request {
        for (name, value) in query {
            request = request.query(name, value);
        }
        request
    }
}

fn validate_canvas_url(url: &str, domain: &str) -> Result<()> {
    let parsed = Url::parse(url).context("Canvas returned an invalid pagination URL")?;
    anyhow::ensure!(
        parsed.scheme() == "https" && parsed.host_str() == Some(domain),
        "Canvas returned a pagination URL outside {domain}"
    );
    Ok(())
}

impl crate::resource::ClientFactory for CanvasClient {
    fn connect(profile: Option<&str>) -> Result<Self> {
        Self::connect(profile)
    }
}

fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn next_link(header: &str) -> Option<String> {
    header.split(',').find_map(|link| {
        let mut parts = link.split(';');
        let target = parts.next()?.trim();
        let is_next = parts.any(|part| part.trim() == "rel=\"next\"");
        is_next.then(|| {
            target
                .strip_prefix('<')
                .and_then(|target| target.strip_suffix('>'))
                .unwrap_or(target)
                .to_string()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{next_link, validate_canvas_url};

    #[test]
    fn finds_the_opaque_next_page_link() {
        let header = concat!(
            "<https://canvas.example/api/v1/courses?page=1>; rel=\"current\", ",
            "<https://canvas.example/api/v1/courses?opaque=abc>; rel=\"next\", ",
            "<https://canvas.example/api/v1/courses?page=9>; rel=\"last\""
        );

        assert_eq!(
            next_link(header).as_deref(),
            Some("https://canvas.example/api/v1/courses?opaque=abc")
        );
    }

    #[test]
    fn returns_none_without_a_next_page() {
        let header = "<https://canvas.example/api/v1/courses?page=1>; rel=\"current\"";
        assert_eq!(next_link(header), None);
    }

    #[test]
    fn rejects_pagination_links_outside_the_canvas_domain() {
        assert!(
            validate_canvas_url(
                "https://canvas.example/api/v1/courses?page=2",
                "canvas.example"
            )
            .is_ok()
        );
        assert!(validate_canvas_url("https://attacker.example/steal", "canvas.example").is_err());
    }
}
