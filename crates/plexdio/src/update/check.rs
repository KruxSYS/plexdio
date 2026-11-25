//! The update check: one GET of the manifest and a local semver decision.

use super::manifest::{self, VersionInfo};
use super::{version, Channel, Result, UpdateError, USER_AGENT};
use std::io::Read;
use std::time::Duration;

/// What the check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    UpToDate,
    UpdateAvailable,
    /// This version is below the release's `min_supported`.
    UpdateRequired,
    /// The server has nothing for this channel and package (404).
    NoReleasePublished,
}

#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub status: Status,
    /// The release the server offered (absent for `NoReleasePublished`).
    pub release: Option<VersionInfo>,
    /// `release.download_url` resolved against the manifest URL.
    pub download_url: Option<String>,
}

impl CheckOutcome {
    pub fn has_update(&self) -> bool {
        matches!(self.status, Status::UpdateAvailable | Status::UpdateRequired)
    }
}

/// `{base}/api/v1/version?channel=…&current=…&arch=…`, Screvia's endpoint.
pub fn check_url(base: &str, channel: Channel, current: &version::Version, arch: &str) -> String {
    format!(
        "{}/api/v1/version?channel={}&current={}&arch={}",
        base.trim().trim_end_matches('/'),
        channel.as_str(),
        urlencoding::encode(&current.to_string()),
        urlencoding::encode(arch)
    )
}

/// Compares a release with this version.
pub fn evaluate(info: &VersionInfo, current: &version::Version) -> Status {
    let Some(latest) = info.parsed_version() else { return Status::UpToDate };
    if let Some(min) = info.min_supported.as_deref().and_then(version::parse) {
        if version::compare(current, &min) == std::cmp::Ordering::Less {
            return Status::UpdateRequired;
        }
    }
    if version::is_newer(&latest, current) {
        Status::UpdateAvailable
    } else {
        Status::UpToDate
    }
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .user_agent(USER_AGENT)
        .build()
        .into()
}

/// Asks the server for the newest release of `channel`/`arch`. Blocking.
pub fn check(base: &str, channel: Channel, current: &version::Version, arch: &str) -> Result<CheckOutcome> {
    let url = check_url(base, channel, current, arch);
    let resp = agent(Duration::from_secs(20))
        .get(&url)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    match resp.status().as_u16() {
        200 => {}
        404 => return Ok(CheckOutcome { status: Status::NoReleasePublished, release: None, download_url: None }),
        s => return Err(UpdateError::Http(s)),
    }
    let mut body = Vec::new();
    resp.into_body()
        .into_reader()
        .take(1 << 20)
        .read_to_end(&mut body)
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    let info = manifest::parse(&body)?;
    let download_url = manifest::resolve_url(&url, &info.download_url)?;
    Ok(CheckOutcome { status: evaluate(&info, current), release: Some(info), download_url: Some(download_url) })
}

/// Agent for package downloads: a long overall limit (slow links), but a stalled connection
/// still fails.
pub(super) fn download_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_global(Some(Duration::from_secs(30 * 60)))
        .http_status_as_error(false)
        .user_agent(USER_AGENT)
        .build()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(v: &str, min: Option<&str>) -> VersionInfo {
        VersionInfo {
            version: v.into(),
            channel: "stable".into(),
            arch: "x64".into(),
            released_at: None,
            notes_md: String::new(),
            download_url: "/x".into(),
            sha256: "0".repeat(64),
            size: 1,
            min_supported: min.map(Into::into),
        }
    }

    fn v(s: &str) -> version::Version {
        version::parse(s).unwrap()
    }

    #[test]
    fn the_check_url_sends_only_channel_version_and_package() {
        assert_eq!(
            check_url("https://plexdio.kruxsys.com/", Channel::Stable, &v("0.1.0"), "x64-setup"),
            "https://plexdio.kruxsys.com/api/v1/version?channel=stable&current=0.1.0&arch=x64-setup"
        );
        assert_eq!(
            check_url("http://h:8", Channel::Beta, &v("0.2.0-beta.1"), "x64"),
            "http://h:8/api/v1/version?channel=beta&current=0.2.0-beta.1&arch=x64"
        );
    }

    #[test]
    fn newer_same_and_older_releases() {
        assert_eq!(evaluate(&info("0.1.1", None), &v("0.1.0")), Status::UpdateAvailable);
        assert_eq!(evaluate(&info("0.1.0", None), &v("0.1.0")), Status::UpToDate);
        assert_eq!(evaluate(&info("0.0.9", None), &v("0.1.0")), Status::UpToDate);
    }

    #[test]
    fn below_min_supported_is_required() {
        assert_eq!(evaluate(&info("0.3.0", Some("0.2.0")), &v("0.1.0")), Status::UpdateRequired);
        assert_eq!(evaluate(&info("0.3.0", Some("0.1.0")), &v("0.1.0")), Status::UpdateAvailable);
    }
}
