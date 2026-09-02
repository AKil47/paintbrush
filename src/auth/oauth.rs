use anyhow::{Context, Result, bail};
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::url::Url;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    RedirectUrl, RefreshToken, TokenUrl,
};

const MOBILE_VERIFY_URL: &str = "https://sso.canvaslms.com/api/v1/mobile_verify.json";
// Identifies us to mobile_verify as the (Student) Android app, whose OAuth
// client credentials we're borrowing; Canvas rejects unrecognized user agents.
// Real app UAs are "candroid/{versionName} ({versionCode})" (Utils.generateUserAgent
// in ref/canvas_android) — a bare "candroid" is rejected (result: 3, UnknownUserAgent).
const MOBILE_USER_AGENT: &str = "candroid/8.5.0 (0)";

/// A `BasicClient` with the auth and token endpoints configured — the state
/// required by `authorize_url`, `exchange_code`, and `exchange_refresh`.
pub type CanvasClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

pub struct MobileVerifyResponse {
    pub client_id: String,
    pub client_secret: String,
    pub protocol: String,
}

/// Calls Canvas's (undocumented, mobile-app-only) `mobile_verify.json`
/// endpoint to obtain OAuth client credentials for `domain`. Not part of
/// RFC 6749, so hand-rolled rather than going through the `oauth2` crate.
///
/// The response is parsed as loosely-typed JSON rather than a fixed struct:
/// the exact shape of a rejection isn't documented anywhere, so on failure
/// we surface Canvas's actual response body rather than guessing at a
/// status-code convention.
pub fn mobile_verify(domain: &str) -> Result<MobileVerifyResponse> {
    let raw: serde_json::Value = ureq::get(MOBILE_VERIFY_URL)
        .query("domain", domain)
        .query("user_agent", MOBILE_USER_AGENT)
        // The Android app sends the same string as both the `user_agent` query
        // param and the real User-Agent header (MobileVerifyAPI.kt) — Canvas
        // rejects the request (result: 3) without the header too.
        .set("User-Agent", MOBILE_USER_AGENT)
        .call()
        .context("mobile_verify request failed")?
        .into_json()
        .context("mobile_verify returned a non-JSON response")?;

    let field = |name: &str| raw.get(name).and_then(|v| v.as_str());

    let (Some(client_id), Some(client_secret), Some(base_url)) = (
        field("client_id"),
        field("client_secret"),
        field("base_url"),
    ) else {
        bail!(
            "mobile_verify did not return OAuth credentials for {domain}; Canvas responded: {raw}"
        );
    };

    let protocol = base_url
        .split_once("://")
        .map(|(scheme, _)| scheme.to_string())
        .unwrap_or_else(|| "https".to_string());

    Ok(MobileVerifyResponse {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        protocol,
    })
}

/// Builds an OAuth client for `domain` using credentials from `mobile_verify`,
/// targeting Canvas's out-of-band native-app flow (no redirect listener).
pub fn build_client(mobile_verify: &MobileVerifyResponse, domain: &str) -> Result<CanvasClient> {
    let auth_url = AuthUrl::new(format!(
        "{}://{}/login/oauth2/auth",
        mobile_verify.protocol, domain
    ))?;
    let token_url = TokenUrl::new(format!(
        "{}://{}/login/oauth2/token",
        mobile_verify.protocol, domain
    ))?;
    // The Android app only uses the documented oob URI for its QR/test-domain
    // flow; for a normal production domain it hardcodes this fixed redirect
    // (BaseLoginSignInActivity.kt:483-488) — confirmed empirically that oob
    // is rejected ("redirect_uri does not match client settings") for this
    // client_id on a real production domain, while this one is accepted.
    // The page it lands on 404s (it exists only to be intercepted as a
    // mobile deep link), but the `code` is still in the URL query string.
    let redirect_url = RedirectUrl::new("https://sso.canvaslms.com/canvas/login".to_string())?;

    Ok(
        BasicClient::new(ClientId::new(mobile_verify.client_id.clone()))
            .set_client_secret(ClientSecret::new(mobile_verify.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url),
    )
}

/// Builds the URL the user should open in a browser to authorize this client.
///
/// A `CsrfToken` is generated to satisfy the builder API, but since this is
/// an out-of-band flow with no redirect listener, it's never compared
/// against anything — an accepted limitation of pasting the code back
/// manually rather than a real gap to close.
pub fn authorize_url(client: &CanvasClient) -> Url {
    let (url, _csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_extra_param("mobile", "1")
        .add_extra_param("purpose", "paintbrush")
        .url();
    url
}

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new().redirects(0).build()
}

pub fn exchange_code(client: &CanvasClient, code: String) -> Result<BasicTokenResponse> {
    client
        .exchange_code(AuthorizationCode::new(code))
        .request(&http_agent())
        .context("failed to exchange authorization code for tokens")
}

pub fn exchange_refresh(
    client: &CanvasClient,
    refresh_token: String,
) -> Result<BasicTokenResponse> {
    client
        .exchange_refresh_token(&RefreshToken::new(refresh_token))
        .request(&http_agent())
        .context("failed to refresh access token")
}
