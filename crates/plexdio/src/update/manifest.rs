//! The update manifest: one release package as served by `GET /api/v1/version`.
//!
//! The field names are Screvia's `VersionInfo` (KruxSYS sister project), so both sites share one
//! format:
//!
//! ```json
//! {"version":"0.1.0","channel":"stable","arch":"x64-setup","released_at":"2026-11-25T00:00:00Z",
//!  "notes_md":"…","download_url":"/downloads/0.1.0/Plexdio-0.1.0-win-x64-setup.exe",
//!  "sha256":"<64 hex>","size":12345678,"min_supported":null}
//! ```
//!
//! Plexdio needs `version`, `download_url`, `sha256` and `size`; the rest is optional and unknown
//! fields are ignored. `download_url` may be relative: it is resolved against the URL the
//! manifest was fetched from, so a test or staging server serves its own files.

use super::{version, Result, UpdateError};
use serde::{Deserialize, Serialize};

/// Largest package Plexdio accepts (guards against a broken or hostile server).
pub const MAX_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub released_at: Option<String>,
    #[serde(default)]
    pub notes_md: String,
    pub download_url: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub min_supported: Option<String>,
}

impl VersionInfo {
    pub fn parsed_version(&self) -> Option<version::Version> {
        version::parse(&self.version)
    }

    /// The release date part of `released_at` (`2026-11-25`), if any.
    pub fn release_date(&self) -> Option<&str> {
        self.released_at.as_deref().map(|s| s.get(..10).unwrap_or(s))
    }
}

/// Parses and validates a manifest body.
pub fn parse(body: &[u8]) -> Result<VersionInfo> {
    let info: VersionInfo = serde_json::from_slice(body).map_err(|e| UpdateError::Manifest(e.to_string()))?;
    validate(&info)?;
    Ok(info)
}

/// The checks every manifest must pass before anything is downloaded.
pub fn validate(info: &VersionInfo) -> Result<()> {
    if info.parsed_version().is_none() {
        return Err(UpdateError::Manifest(format!("\"{}\" is not a semantic version", info.version)));
    }
    if info.sha256.len() != 64 || !info.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UpdateError::Manifest("the SHA-256 is missing or malformed".into()));
    }
    if info.size == 0 || info.size > MAX_PACKAGE_BYTES {
        return Err(UpdateError::Manifest(format!("implausible package size {}", info.size)));
    }
    if info.download_url.trim().is_empty() {
        return Err(UpdateError::Manifest("no download URL".into()));
    }
    if let Some(min) = &info.min_supported {
        if version::parse(min).is_none() {
            return Err(UpdateError::Manifest(format!("min_supported \"{min}\" is not a semantic version")));
        }
    }
    Ok(())
}

/// Resolves `link` (absolute, scheme-relative, host-relative or path-relative) against `base`,
/// the URL the manifest was fetched from. Only http and https results are accepted.
pub fn resolve_url(base: &str, link: &str) -> Result<String> {
    let link = link.trim();
    let bad = || UpdateError::Manifest(format!("unusable download URL \"{link}\""));
    let (scheme, rest) = split_scheme(base).filter(|(s, _)| s == "http" || s == "https").ok_or_else(bad)?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let origin = format!("{scheme}://{}", &rest[..authority_end]);
    if rest[..authority_end].is_empty() {
        return Err(bad());
    }
    let resolved = if let Some((s, _)) = split_scheme(link) {
        if s != "http" && s != "https" {
            return Err(bad());
        }
        format!("{s}{}", &link[s.len()..])
    } else if let Some(r) = link.strip_prefix("//") {
        format!("{scheme}://{r}")
    } else if link.starts_with('/') {
        format!("{origin}{link}")
    } else if link.find(':').is_some_and(|c| link.find('/').is_none_or(|s| c < s)) {
        // "mailto:x", "file:…", "c:\…": something with a scheme we don't follow.
        return Err(bad());
    } else {
        // Relative to the manifest's folder (query and fragment of the base dropped).
        let path = &rest[authority_end..];
        let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];
        let dir = &path[..path.rfind('/').map(|i| i + 1).unwrap_or(0)];
        let dir = if dir.is_empty() { "/" } else { dir };
        format!("{origin}{dir}{link}")
    };
    if resolved.contains(char::is_whitespace) {
        return Err(bad());
    }
    Ok(resolved)
}

/// `("https", "host/path")` for `https://host/path`; schemes are lower-cased.
fn split_scheme(url: &str) -> Option<(String, &str)> {
    let i = url.find("://")?;
    let scheme = &url[..i];
    if scheme.is_empty() || !scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)) {
        return None;
    }
    Some((scheme.to_ascii_lowercase(), &url[i + 3..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    fn json(extra: &str) -> String {
        format!(
            r#"{{"version":"0.1.1","download_url":"/downloads/0.1.1/Plexdio-0.1.1-win-x64-setup.exe","sha256":"{SHA}","size":1234{extra}}}"#
        )
    }

    #[test]
    fn parses_screvia_style_version_info() {
        let body = format!(
            r#"{{"version":"0.1.0","channel":"stable","arch":"x64-setup","released_at":"2026-11-25T00:00:00Z",
                "notes_md":"- first","download_url":"/downloads/0.1.0/a.exe","sha256":"{SHA}","size":42,
                "min_supported":null,"file_name":"a.exe","notes_html":"<ul></ul>","update_available":true}}"#
        );
        let info = parse(body.as_bytes()).unwrap();
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.channel, "stable");
        assert_eq!(info.arch, "x64-setup");
        assert_eq!(info.release_date(), Some("2026-11-25"));
        assert_eq!(info.notes_md, "- first");
        assert_eq!(info.size, 42);
        assert_eq!(info.min_supported, None);
    }

    #[test]
    fn only_the_four_required_fields_are_needed() {
        let info = parse(json("").as_bytes()).unwrap();
        assert_eq!(info.channel, "");
        assert_eq!(info.released_at, None);
        assert_eq!(info.notes_md, "");
    }

    #[test]
    fn rejects_missing_or_bad_fields() {
        assert!(parse(br#"{"version":"0.1.1"}"#).is_err());
        assert!(parse(b"not json").is_err());
        assert!(parse(json("").replace("0.1.1\"", "latest\"").as_bytes()).is_err());
        assert!(parse(json("").replace(SHA, "abc").as_bytes()).is_err());
        assert!(parse(json("").replace(SHA, &"z".repeat(64)).as_bytes()).is_err());
        assert!(parse(json("").replace("1234", "0").as_bytes()).is_err());
        assert!(parse(json("").replace("1234", "99999999999").as_bytes()).is_err());
        assert!(parse(json(r#","min_supported":"soon""#).as_bytes()).is_err());
        assert!(parse(json(r#","min_supported":"0.1.0""#).as_bytes()).is_ok());
    }

    #[test]
    fn relative_urls_resolve_against_the_fetched_manifest_url() {
        let base = "https://plexdio.kruxsys.com/api/v1/version?channel=stable&current=0.1.0&arch=x64";
        assert_eq!(
            resolve_url(base, "/downloads/0.1.1/P.exe").unwrap(),
            "https://plexdio.kruxsys.com/downloads/0.1.1/P.exe"
        );
        assert_eq!(resolve_url(base, "P.zip").unwrap(), "https://plexdio.kruxsys.com/api/v1/P.zip");
        assert_eq!(resolve_url(base, "//cdn.example/x.exe").unwrap(), "https://cdn.example/x.exe");
        assert_eq!(resolve_url(base, "https://other.example/x.exe").unwrap(), "https://other.example/x.exe");
    }

    #[test]
    fn an_update_url_override_keeps_its_host_and_port() {
        let base = "http://192.168.1.20:8765/api/v1/version?channel=stable&arch=x64-setup";
        assert_eq!(
            resolve_url(base, "/downloads/0.1.1/Plexdio-0.1.1-win-x64-setup.exe").unwrap(),
            "http://192.168.1.20:8765/downloads/0.1.1/Plexdio-0.1.1-win-x64-setup.exe"
        );
        assert_eq!(resolve_url("http://h:1", "x.zip").unwrap(), "http://h:1/x.zip");
    }

    #[test]
    fn other_schemes_are_refused() {
        let base = "https://plexdio.kruxsys.com/api/v1/version";
        assert!(resolve_url(base, "file:///C:/Windows/x.exe").is_err());
        assert!(resolve_url(base, "ftp://h/x.exe").is_err());
        assert!(resolve_url(base, "javascript:alert(1)").is_err());
        assert!(resolve_url(base, "/a b.exe").is_err());
        assert!(resolve_url("not a url", "/x.exe").is_err());
    }
}
