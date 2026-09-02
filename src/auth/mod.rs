mod oauth;
mod store;

use std::io::{self, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use oauth2::TokenResponse;

use oauth::MobileVerifyResponse;
use store::StoredCredentials;

const TOKEN_EXPIRY_SKEW: Duration = Duration::from_secs(60);

/// A resolved, ready-to-use Canvas session: the domain to hit and a valid
/// bearer token for it.
pub struct Session {
    pub domain: String,
    pub access_token: String,
}

fn normalize_domain(domain: &str) -> &str {
    domain
        .trim_start_matches("https://")
        .trim_start_matches("http://")
}

fn login_hint(profile: &str) -> String {
    format!("run `paintbrush login --profile {profile} --domain <domain>`")
}

/// Logs in to `domain` via browser OAuth and stores the resulting
/// credentials under `profile` (or under the domain itself, if no profile
/// name is given). Registers the profile, making it the default if it's the
/// first one ever created.
pub fn login(profile: Option<&str>, domain: &str) -> Result<()> {
    let domain = normalize_domain(domain);
    let profile = profile.unwrap_or(domain);

    let verify = oauth::mobile_verify(domain)?;
    let client = oauth::build_client(&verify, domain)?;
    let url = oauth::authorize_url(&client);

    println!("Open this URL to authorize paintbrush:\n{url}");
    if open::that(url.as_str()).is_err() {
        eprintln!("warning: couldn't open a browser automatically; open the URL above manually");
    }

    println!(
        "\nAfter you log in and approve access, your browser will land on a page \
         that says \"Page Not Found\" at sso.canvaslms.com — that's expected. \
         Copy the value of the `code` parameter from that page's URL."
    );
    print!("Paste the code here: ");
    io::stdout().flush()?;
    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim().to_string();

    let token = oauth::exchange_code(&client, code)?;
    save_token_response(profile, domain, &verify, &token)?;
    crate::profile::register(profile)?;

    println!("Logged in to {domain} as profile '{profile}'.");
    Ok(())
}

/// Returns a valid session for `profile`, refreshing the stored token if it
/// has expired. This is the entry point Canvas API commands call to get a
/// usable domain + token.
pub fn ensure_valid_token(profile: &str) -> Result<Session> {
    let stored = store::load(profile)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no stored credentials for profile '{profile}'; {}",
            login_hint(profile)
        )
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_secs();

    match stored.expires_at {
        // No known expiry (observed for tokens from the borrowed Android
        // client) — nothing to proactively refresh against; a future
        // caller hitting a 401 is the only signal that this needs renewing.
        None => {
            return Ok(Session {
                domain: stored.domain,
                access_token: stored.access_token,
            });
        }
        Some(expires_at) if expires_at > now + TOKEN_EXPIRY_SKEW.as_secs() => {
            return Ok(Session {
                domain: stored.domain,
                access_token: stored.access_token,
            });
        }
        Some(_) => {}
    }

    let verify = MobileVerifyResponse {
        client_id: stored.client_id.clone(),
        client_secret: stored.client_secret.clone(),
        protocol: stored.protocol.clone(),
    };
    let client = oauth::build_client(&verify, &stored.domain)?;

    let token = oauth::exchange_refresh(&client, stored.refresh_token.clone()).map_err(|_| {
        anyhow::anyhow!(
            "failed to refresh your stored credentials; {}",
            login_hint(profile)
        )
    })?;

    let refresh_token = token
        .refresh_token()
        .map(|t| t.secret().clone())
        .unwrap_or(stored.refresh_token);
    let access_token = token.access_token().secret().clone();

    store::save(
        profile,
        &StoredCredentials {
            domain: stored.domain.clone(),
            protocol: stored.protocol,
            client_id: stored.client_id,
            client_secret: stored.client_secret,
            access_token: access_token.clone(),
            refresh_token,
            expires_at: expires_at(&token)?,
        },
    )?;

    Ok(Session {
        domain: stored.domain,
        access_token,
    })
}

/// Fetches the rendered HTML at `target_url` (which must be on the
/// profile's Canvas domain) as an authenticated web session, not a plain
/// `/api/v1` call — needed for content (e.g. quizzes) that Canvas only
/// serves as rendered HTML.
///
/// Reuses `profile`'s stored session cookies when they're still valid. If
/// there are none yet, or Canvas no longer honors them (detected by the
/// response landing off-domain, e.g. redirected to an SSO login page), a
/// fresh session is minted via Canvas's `/login/session_token` endpoint
/// (exchanging the OAuth access token for a one-time, 30-second-lived
/// token that redeems into a real session) and the resulting cookies are
/// persisted back to the profile for next time.
pub fn fetch_html(profile: &str, target_url: &str) -> Result<String> {
    let session = ensure_valid_token(profile)?;

    let cookie_store = load_cookie_store(profile)?;
    let agent = ureq::AgentBuilder::new()
        .cookie_store(cookie_store)
        .redirects(10)
        .build();

    if let Some(html) = try_fetch(&agent, target_url, &session.domain)? {
        save_cookie_store(profile, &agent)?;
        return Ok(html);
    }

    establish_web_session(&agent, &session, target_url)?;

    let html = try_fetch(&agent, target_url, &session.domain)?.ok_or_else(|| {
        anyhow::anyhow!(
            "still redirected off {} after establishing a session",
            session.domain
        )
    })?;
    save_cookie_store(profile, &agent)?;
    Ok(html)
}

/// `GET`s `target_url`, returning its body only if we ended up authenticated
/// (the final URL, after redirects, is still on `domain` — an SSO/login
/// bounce lands elsewhere).
fn try_fetch(agent: &ureq::Agent, target_url: &str, domain: &str) -> Result<Option<String>> {
    let resp = agent
        .get(target_url)
        .call()
        .context("request to the target URL failed")?;
    let landed_on_domain = resp
        .get_url()
        .parse::<oauth2::url::Url>()
        .ok()
        .and_then(|u| u.host_str().map(|h| h == domain))
        .unwrap_or(false);
    let body = resp
        .into_string()
        .context("response body wasn't valid UTF-8")?;
    Ok(landed_on_domain.then_some(body))
}

/// Mints a fresh, one-time `session_token` from the OAuth access token and
/// redeems it against `agent`'s cookie jar, establishing a real Canvas web
/// session good for `target_url` (and the rest of the domain).
fn establish_web_session(agent: &ureq::Agent, session: &Session, target_url: &str) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct AuthedSession {
        session_url: String,
    }

    let authed: AuthedSession = agent
        .get(&format!("https://{}/login/session_token", session.domain))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .query("return_to", target_url)
        .call()
        .context("session_token request failed")?
        .into_json()
        .context("unexpected response from the session_token endpoint")?;

    agent
        .get(&authed.session_url)
        .call()
        .context("failed to redeem the session_token")?;
    Ok(())
}

fn load_cookie_store(profile: &str) -> Result<cookie_store::CookieStore> {
    match store::load_cookies(profile)? {
        // Canvas's session cookie is non-persistent (no Expires attribute) —
        // `load_all` (paired with `save_incl_expired_and_nonpersistent` below)
        // is what keeps it around across separate CLI invocations.
        Some(json) => cookie_store::serde::json::load_all(json.as_bytes()).map_err(|e| {
            anyhow::anyhow!("stored cookies are corrupt ({e}); run `paintbrush login` again")
        }),
        None => Ok(cookie_store::CookieStore::default()),
    }
}

fn save_cookie_store(profile: &str, agent: &ureq::Agent) -> Result<()> {
    let mut buf = Vec::new();
    cookie_store::serde::json::save_incl_expired_and_nonpersistent(&agent.cookie_store(), &mut buf)
        .map_err(|e| anyhow::anyhow!("failed to serialize session cookies: {e}"))?;
    let json = String::from_utf8(buf).context("session cookies serialized as non-UTF8")?;
    store::save_cookies(profile, &json)
}

/// Deletes any stored credentials for `profile`. A no-op if none exist.
pub fn forget(profile: &str) -> Result<()> {
    store::delete(profile)
}

/// The domain a profile is logged into, if it has stored credentials.
pub fn domain_for(profile: &str) -> Result<Option<String>> {
    Ok(store::load(profile)?.map(|c| c.domain))
}

fn save_token_response(
    profile: &str,
    domain: &str,
    verify: &MobileVerifyResponse,
    token: &oauth2::basic::BasicTokenResponse,
) -> Result<()> {
    let refresh_token = token
        .refresh_token()
        .context("Canvas did not issue a refresh token on login")?
        .secret()
        .clone();

    store::save(
        profile,
        &StoredCredentials {
            domain: domain.to_string(),
            protocol: verify.protocol.clone(),
            client_id: verify.client_id.clone(),
            client_secret: verify.client_secret.clone(),
            access_token: token.access_token().secret().clone(),
            refresh_token,
            expires_at: expires_at(token)?,
        },
    )
}

/// `None` if Canvas didn't report an `expires_in` for this token — observed
/// behavior for tokens issued through the borrowed Android client, not an
/// error case.
fn expires_at(token: &oauth2::basic::BasicTokenResponse) -> Result<Option<u64>> {
    let Some(expires_in) = token.expires_in() else {
        return Ok(None);
    };
    let expires_at = SystemTime::now() + expires_in;
    Ok(Some(expires_at.duration_since(UNIX_EPOCH)?.as_secs()))
}
