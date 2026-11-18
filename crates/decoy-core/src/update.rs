//! Looks for a newer release on GitHub.
//!
//! One small request to GitHub's public API. Nothing about the user is sent;
//! the app only reads the tag of the latest release and never downloads or
//! installs anything by itself.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http;

/// The repository whose releases are checked.
pub const REPO: &str = "mountain-beaver/decoy";

/// A release newer than the running version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Update {
    /// Without the leading `v`.
    pub version: String,
    /// The release page on GitHub.
    pub url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// The latest release, if it is newer than `current`.
pub fn check(current: &str) -> Result<Option<Update>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    match http::get_bytes(&url, Duration::from_secs(10), 256 * 1024) {
        Ok(body) => Ok(parse(&body, current)),
        // No release yet.
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

fn parse(body: &[u8], current: &str) -> Option<Update> {
    let release: Release = serde_json::from_slice(body).ok()?;
    if release.draft || release.prerelease {
        return None;
    }
    let version = release.tag_name.trim().trim_start_matches('v').to_string();
    // The link is opened in the browser later; accept only this repository's pages.
    let expected = format!("https://github.com/{REPO}/releases/");
    if !release.html_url.starts_with(&expected) || !is_newer(&version, current) {
        return None;
    }
    Some(Update {
        version,
        url: release.html_url,
    })
}

/// `1.2.3` style comparison; anything after `-` or `+` is ignored.
fn is_newer(candidate: &str, current: &str) -> bool {
    fn numbers(v: &str) -> Option<Vec<u64>> {
        let core = v.split(['-', '+']).next().unwrap_or(v);
        let parts: Vec<u64> = core
            .split('.')
            .map(|p| p.parse().ok())
            .collect::<Option<_>>()?;
        (!parts.is_empty() && parts.len() <= 4).then_some(parts)
    }
    match (numbers(candidate), numbers(current)) {
        (Some(mut a), Some(mut b)) => {
            let len = a.len().max(b.len());
            a.resize(len, 0);
            b.resize(len, 0);
            a > b
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, url: &str, extra: &str) -> Vec<u8> {
        format!(r#"{{"tag_name":"{tag}","html_url":"{url}"{extra},"body":"notes"}}"#).into_bytes()
    }

    const PAGE: &str = "https://github.com/mountain-beaver/decoy/releases/tag/v0.2.0";

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(is_newer("1.0.1", "1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("nightly", "0.1.0"));
        assert!(!is_newer("", "0.1.0"));
    }

    #[test]
    fn offers_a_newer_release() {
        let got = parse(&release("v0.2.0", PAGE, ""), "0.1.0").unwrap();
        assert_eq!(got.version, "0.2.0");
        assert_eq!(got.url, PAGE);
    }

    #[test]
    fn ignores_the_same_or_an_older_release() {
        assert_eq!(parse(&release("v0.2.0", PAGE, ""), "0.2.0"), None);
        assert_eq!(parse(&release("v0.2.0", PAGE, ""), "0.3.0"), None);
    }

    #[test]
    fn ignores_drafts_and_prereleases() {
        assert_eq!(
            parse(&release("v0.2.0", PAGE, r#","draft":true"#), "0.1.0"),
            None
        );
        assert_eq!(
            parse(&release("v0.2.0", PAGE, r#","prerelease":true"#), "0.1.0"),
            None
        );
    }

    #[test]
    fn ignores_links_that_leave_the_repository() {
        for url in [
            "https://example.com/mountain-beaver/decoy/releases/tag/v9",
            "https://github.com/someone-else/decoy/releases/tag/v9",
            "file:///etc/passwd",
        ] {
            assert_eq!(parse(&release("v9.0.0", url, ""), "0.1.0"), None);
        }
    }

    #[test]
    fn ignores_garbage() {
        assert_eq!(parse(b"<html>rate limited</html>", "0.1.0"), None);
        assert_eq!(parse(b"{}", "0.1.0"), None);
    }
}
