//! Optional update check against the latest GitHub release.
//!
//! The check runs at startup unless the user has turned it off in the settings. It asks
//! the GitHub API for the latest release, sends no data besides a generic
//! user agent, and never downloads or installs anything: the app only tells the
//! user that a newer version exists and links to the release page and to the file that
//! suits this computer. The user's browser does the downloading, when they press the link.

use std::time::Duration;

use serde::Deserialize;

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/isaul32/cashflow/releases/latest";

/// Every address taken from GitHub's answer must be under this one, so a strange answer cannot
/// send the user anywhere else.
const RELEASES: &str = "https://github.com/isaul32/cashflow/releases/";

/// A published release that is newer than the running version.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Version number without the leading `v`, e.g. `0.2.0`.
    pub version: String,
    /// The release page to open in the browser.
    pub url: String,
    /// The file for this computer (installer, disk image, ...), if the release has one.
    pub download: Option<String>,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

/// The end of the file name that suits this computer, e.g. `-windows-x64-setup.exe`.
/// `None` when the release has no build for it (e.g. an Intel Mac).
fn wanted_suffix(os: &str, arch: &str, portable: bool, deb: bool) -> Option<String> {
    match os {
        // The Windows build is x64; it also runs on ARM.
        "windows" => Some(if portable { "-windows-x64-portable.zip" } else { "-windows-x64-setup.exe" }.to_string()),
        "macos" => (arch == "aarch64").then(|| "-macos-arm64.dmg".to_string()),
        "linux" => {
            let arch = match arch {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                _ => return None,
            };
            Some(format!("-linux-{arch}.{}", if deb { "deb" } else { "AppImage" }))
        }
        _ => None,
    }
}

/// The suffix for the program that is running: the portable zip when the `portable` marker is
/// next to it, the .deb when it was installed under /usr, otherwise the installer or AppImage.
fn current_suffix() -> Option<String> {
    let exe = std::env::current_exe().ok();
    let portable = exe.as_ref().and_then(|e| e.parent()).is_some_and(|dir| dir.join("portable").is_file());
    let deb = std::env::var_os("APPIMAGE").is_none() && exe.as_ref().is_some_and(|e| e.starts_with("/usr/"));
    wanted_suffix(std::env::consts::OS, std::env::consts::ARCH, portable, deb)
}

/// The download address of the file ending in `suffix`, if the release has it.
fn pick_download(assets: &[ApiAsset], suffix: &str) -> Option<String> {
    assets
        .iter()
        .find(|a| a.name.starts_with("cashflow-") && a.name.ends_with(suffix))
        .map(|a| a.browser_download_url.clone())
        .filter(|url| url.starts_with(&format!("{RELEASES}download/")))
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

/// Reads the API response and returns the release if it is newer than `current`. `suffix` picks
/// the file to link to (see [`wanted_suffix`]).
fn newer_release(json: &str, current: &str, suffix: Option<&str>) -> Result<Option<Release>, String> {
    let api: ApiRelease = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if api.draft || api.prerelease || !is_newer(&api.tag_name, current) {
        return Ok(None);
    }
    if !api.html_url.starts_with(RELEASES) {
        return Err("the release address is not one of this project's".to_string());
    }
    Ok(Some(Release {
        version: api.tag_name.trim_start_matches(['v', 'V']).to_string(),
        url: api.html_url,
        download: suffix.and_then(|s| pick_download(&api.assets, s)),
    }))
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
    newer_release(&body, current, current_suffix().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        let json = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/isaul32/cashflow/releases/tag/v0.2.0"}"#;
        let r = newer_release(json, "0.1.1", None).unwrap().unwrap();
        assert_eq!(r.version, "0.2.0");
        assert!(r.url.ends_with("v0.2.0"));
        assert_eq!(newer_release(json, "0.2.0", None).unwrap(), None);
    }

    #[test]
    fn ignores_drafts_and_prereleases() {
        let pre = r#"{"tag_name":"v9.0.0","html_url":"u","prerelease":true}"#;
        assert_eq!(newer_release(pre, "0.1.1", None).unwrap(), None);
        assert!(newer_release("not json", "0.1.1", None).is_err());
    }

    fn release_json(html_url: &str) -> String {
        let base = "https://github.com/isaul32/cashflow/releases/download/v0.2.0";
        json!({
            "tag_name": "v0.2.0",
            "html_url": html_url,
            "assets": [
                {"name": "SHA256SUMS.txt", "browser_download_url": format!("{base}/SHA256SUMS.txt")},
                {"name": "cashflow-0.2.0-windows-x64-setup.exe", "browser_download_url": format!("{base}/cashflow-0.2.0-windows-x64-setup.exe")},
                {"name": "cashflow-0.2.0-windows-x64-portable.zip", "browser_download_url": format!("{base}/cashflow-0.2.0-windows-x64-portable.zip")},
                {"name": "cashflow-0.2.0-macos-arm64.dmg", "browser_download_url": format!("{base}/cashflow-0.2.0-macos-arm64.dmg")},
                {"name": "cashflow-0.2.0-linux-x64.AppImage", "browser_download_url": format!("{base}/cashflow-0.2.0-linux-x64.AppImage")},
                {"name": "cashflow-0.2.0-linux-x64.deb", "browser_download_url": format!("{base}/cashflow-0.2.0-linux-x64.deb")},
                {"name": "cashflow-0.2.0-linux-arm64.deb", "browser_download_url": format!("{base}/cashflow-0.2.0-linux-arm64.deb")},
            ],
        })
        .to_string()
    }

    const PAGE: &str = "https://github.com/isaul32/cashflow/releases/tag/v0.2.0";

    #[test]
    fn the_file_that_suits_each_computer_is_picked() {
        let json = release_json(PAGE);
        let file = |os, arch, portable, deb| {
            let suffix = wanted_suffix(os, arch, portable, deb);
            newer_release(&json, "0.1.0", suffix.as_deref()).unwrap().unwrap().download
        };
        let base = "https://github.com/isaul32/cashflow/releases/download/v0.2.0/cashflow-0.2.0-";
        assert_eq!(file("windows", "x86_64", false, false), Some(format!("{base}windows-x64-setup.exe")));
        assert_eq!(file("windows", "aarch64", false, false), Some(format!("{base}windows-x64-setup.exe")), "x64 runs on ARM");
        assert_eq!(file("windows", "x86_64", true, false), Some(format!("{base}windows-x64-portable.zip")));
        assert_eq!(file("macos", "aarch64", false, false), Some(format!("{base}macos-arm64.dmg")));
        assert_eq!(file("linux", "x86_64", false, false), Some(format!("{base}linux-x64.AppImage")));
        assert_eq!(file("linux", "x86_64", false, true), Some(format!("{base}linux-x64.deb")));
        assert_eq!(file("linux", "aarch64", false, true), Some(format!("{base}linux-arm64.deb")));
    }

    #[test]
    fn no_file_means_only_the_release_page() {
        let json = release_json(PAGE);
        // No Intel Mac build and no arm64 AppImage in this release, and unknown systems have none.
        for (os, arch) in [("macos", "x86_64"), ("linux", "aarch64"), ("freebsd", "x86_64"), ("linux", "riscv64")] {
            let suffix = wanted_suffix(os, arch, false, false);
            let release = newer_release(&json, "0.1.0", suffix.as_deref()).unwrap().unwrap();
            assert_eq!((release.download, release.url.as_str()), (None, PAGE), "{os} {arch}");
        }
        // A release without any files, or no wish at all.
        let bare = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/isaul32/cashflow/releases/tag/v0.2.0"}"#;
        assert_eq!(newer_release(bare, "0.1.0", Some("-windows-x64-setup.exe")).unwrap().unwrap().download, None);
        assert_eq!(newer_release(&release_json(PAGE), "0.1.0", None).unwrap().unwrap().download, None);
    }

    #[test]
    fn addresses_outside_the_project_are_never_used() {
        let evil_page = release_json("https://evil.example/releases/tag/v0.2.0");
        assert!(newer_release(&evil_page, "0.1.0", None).is_err());
        let evil_file = json!({
            "tag_name": "v0.2.0",
            "html_url": PAGE,
            "assets": [{"name": "cashflow-0.2.0-windows-x64-setup.exe", "browser_download_url": "https://evil.example/setup.exe"}],
        })
        .to_string();
        let release = newer_release(&evil_file, "0.1.0", Some("-windows-x64-setup.exe")).unwrap().unwrap();
        assert_eq!(release.download, None, "a file from somewhere else is not offered");
        // And a file that merely ends the same way but is not ours.
        let other = json!({
            "tag_name": "v0.2.0",
            "html_url": PAGE,
            "assets": [{"name": "malware-setup.exe", "browser_download_url": "https://github.com/isaul32/cashflow/releases/download/v0.2.0/malware-setup.exe"}],
        })
        .to_string();
        assert_eq!(newer_release(&other, "0.1.0", Some("-setup.exe")).unwrap().unwrap().download, None);
    }
}
