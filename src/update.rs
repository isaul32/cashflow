//! Optional update check against the latest GitHub release.
//!
//! The check only runs when the user has turned it on in the settings. It asks
//! the GitHub API for the latest release, sends no data besides a generic
//! user agent, and never downloads or installs anything: the app only tells the
//! user that a newer version exists and links to the release page.

use std::time::Duration;

use serde::Deserialize;

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/isaul32/cashflow/releases/latest";

/// A published release that is newer than the running version.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Version number without the leading `v`, e.g. `0.2.0`.
    pub version: String,
    /// The release page to open in the browser.
    pub url: String,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Parses "v1.2.3", "1.2" or "1.2.3-beta" into comparable numbers. A
/// pre-release suffix is ignored: only the numeric core counts.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    let patch = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// True when `candidate` is a higher version than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

/// Reads the API response and returns the release if it is newer than `current`.
fn newer_release(json: &str, current: &str) -> Result<Option<Release>, String> {
    let api: ApiRelease = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if api.draft || api.prerelease || !is_newer(&api.tag_name, current) {
        return Ok(None);
    }
    Ok(Some(Release { version: api.tag_name.trim_start_matches(['v', 'V']).to_string(), url: api.html_url }))
}

/// Asks GitHub for the latest release. Blocking: run it off the UI thread.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .user_agent("cashflow-update-check")
        .build()
        .into();
    let body = agent
        .get(LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    newer_release(&body, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("v0.1.2", "0.1.1"));
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(!is_newer("v0.1.1", "0.1.1"));
        assert!(!is_newer("v0.1.0", "0.1.1"));
        assert!(!is_newer("garbage", "0.1.1"));
    }

    #[test]
    fn accepts_the_latest_release_only_when_newer() {
        let json = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/x/y/releases/tag/v0.2.0"}"#;
        let r = newer_release(json, "0.1.1").unwrap().unwrap();
        assert_eq!(r.version, "0.2.0");
        assert!(r.url.ends_with("v0.2.0"));
        assert_eq!(newer_release(json, "0.2.0").unwrap(), None);
    }

    #[test]
    fn ignores_drafts_and_prereleases() {
        let pre = r#"{"tag_name":"v9.0.0","html_url":"u","prerelease":true}"#;
        assert_eq!(newer_release(pre, "0.1.1").unwrap(), None);
        assert!(newer_release("not json", "0.1.1").is_err());
    }
}
