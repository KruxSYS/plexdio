//! Update checks and updates against plexdio.kruxsys.com.
//!
//! Mirrors Screvia's update client (KruxSYS sister project):
//! - `GET {base}/api/v1/version?channel=&current=&arch=` returns one [`manifest::VersionInfo`] or
//!   404 when nothing is published. Only those three values and a `Plexdio/<version>` user agent
//!   are sent.
//! - The decision is made here with semver ([`version`]).
//! - Downloads are rejected unless their size and SHA-256 match the manifest ([`download`]).
//!
//! An installed copy (`installed` marker) asks for `arch=x64-setup` and runs the new installer
//! silently ([`installer`]). A portable copy asks for `arch=x64` (the zip) and swaps its own exe
//! ([`portable`]). A bare exe ("standalone") is pointed at the download page.
//!
//! Everything here is blocking: call it from a worker thread, never the UI thread.

pub mod check;
pub mod download;
pub mod installer;
pub mod manifest;
pub mod portable;
pub mod schedule;
pub mod version;

use crate::paths::InstallMode;
use serde::{Deserialize, Serialize};

/// Where Plexdio looks for updates unless `--update-url` / `PLEXDIO_UPDATE_URL` says otherwise.
pub const DEFAULT_BASE_URL: &str = "https://plexdio.kruxsys.com";

/// Release channel. Beta users also get stable releases (the server's beta manifest always
/// points at the newest of both).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Channel::Stable => "Stable",
            Channel::Beta => "Beta",
        }
    }
}

/// The package this copy updates from: the installer for installed copies, the zip otherwise.
pub fn arch_for(mode: InstallMode) -> &'static str {
    match mode {
        InstallMode::Installed => "x64-setup",
        _ => "x64",
    }
}

/// How this copy applies an update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    /// Download the installer, verify it, run it silently, exit.
    RunInstaller,
    /// Download the zip, verify it, swap plexdio.exe, relaunch.
    SwapExe,
    /// Open the download page in the browser.
    OpenDownloadPage,
}

pub fn action_for(mode: InstallMode) -> UpdateAction {
    match mode {
        InstallMode::Installed => UpdateAction::RunInstaller,
        InstallMode::Portable => UpdateAction::SwapExe,
        InstallMode::Standalone | InstallMode::Custom => UpdateAction::OpenDownloadPage,
    }
}

/// `{base}/download`: the website's download page.
pub fn download_page(base: &str) -> String {
    format!("{}/download", base.trim_end_matches('/'))
}

/// `{base}/changelog#v<version>`: the release notes of one version.
pub fn release_notes_page(base: &str, version: &str) -> String {
    format!("{}/changelog#v{}", base.trim_end_matches('/'), version)
}

/// Why an update check or update failed. The texts are shown in Settings → About.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Could not reach the update server: {0}")]
    Network(String),
    #[error("The update server answered HTTP {0}")]
    Http(u16),
    #[error("The update information is invalid: {0}")]
    Manifest(String),
    #[error("The download did not match its SHA-256; nothing was installed")]
    HashMismatch,
    #[error("The download has the wrong size ({got} bytes, expected {expected}); nothing was installed")]
    SizeMismatch { got: u64, expected: u64 },
    #[error("The update package is not a Plexdio portable zip: {0}")]
    BadPackage(String),
    #[error("Plexdio's folder is not writable, so it can't update itself here")]
    NotWritable,
    #[error("{0}")]
    Io(String),
}

impl From<std::io::Error> for UpdateError {
    fn from(e: std::io::Error) -> Self {
        UpdateError::Io(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, UpdateError>;

/// User agent of every update request: `Plexdio/<version>`.
pub const USER_AGENT: &str = concat!("Plexdio/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_copies_use_the_installer_package() {
        assert_eq!(arch_for(InstallMode::Installed), "x64-setup");
        assert_eq!(arch_for(InstallMode::Portable), "x64");
        assert_eq!(arch_for(InstallMode::Standalone), "x64");
        assert_eq!(action_for(InstallMode::Installed), UpdateAction::RunInstaller);
        assert_eq!(action_for(InstallMode::Portable), UpdateAction::SwapExe);
        assert_eq!(action_for(InstallMode::Standalone), UpdateAction::OpenDownloadPage);
    }

    #[test]
    fn site_links() {
        assert_eq!(download_page("https://plexdio.kruxsys.com/"), "https://plexdio.kruxsys.com/download");
        assert_eq!(release_notes_page("http://h:1", "0.1.1"), "http://h:1/changelog#v0.1.1");
    }

    #[test]
    fn channel_serialises_lowercase() {
        assert_eq!(serde_json::to_string(&Channel::Beta).unwrap(), "\"beta\"");
        assert_eq!(serde_json::from_str::<Channel>("\"stable\"").unwrap(), Channel::Stable);
    }
}
