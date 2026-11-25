//! Filesystem locations.
//!
//! Where Plexdio keeps its data depends on how it was installed ([`InstallMode`]):
//! - **Installed** (an `installed` marker next to the exe, written by the installer) and
//!   **Standalone** (no marker: a bare exe, a dev build): config in the roaming profile
//!   (`%APPDATA%\Plexdio`), caches and logs in the local profile (`%LOCALAPPDATA%\Plexdio`).
//! - **Portable** (a `portable` marker next to the exe, shipped in the portable zip): everything
//!   in a `Data` folder next to the exe (`Data\config`, `Data\cache`).
//! - `PLEXDIO_HOME` overrides all of them (tests and side-by-side runs).
//!
//! The Plex token is not a file here in any mode: it stays in the Windows Credential Manager.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const APP_DIR: &str = "Plexdio";
/// Folder name used before the app was renamed from Plexify.
const LEGACY_APP_DIR: &str = "Plexify";
/// Marker file next to the exe of the portable zip.
pub const PORTABLE_MARKER: &str = "portable";
/// Marker file the installer puts next to the exe.
pub const INSTALLED_MARKER: &str = "installed";
/// Data folder of a portable copy, next to the exe.
pub const PORTABLE_DATA_DIR: &str = "Data";

/// How this copy of Plexdio was installed; decides the data folders and how it updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    /// Installed by the setup (`installed` marker): updates by running the new installer.
    Installed,
    /// From the portable zip (`portable` marker): data next to the exe, updates by swapping the exe.
    Portable,
    /// A bare exe without a marker: profile folders like an installed copy, updates via the website.
    Standalone,
    /// `PLEXDIO_HOME` is set.
    Custom,
}

impl InstallMode {
    pub fn label(self) -> &'static str {
        match self {
            InstallMode::Installed => "Installed",
            InstallMode::Portable => "Portable",
            InstallMode::Standalone => "Standalone exe",
            InstallMode::Custom => "Custom data folder (PLEXDIO_HOME)",
        }
    }
}

/// The data folders for one [`InstallMode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub mode: InstallMode,
    pub config: PathBuf,
    pub cache: PathBuf,
}

/// Picks the layout. `exe_dir` is the folder of the running exe, `home` the `PLEXDIO_HOME`
/// value, `config_base`/`cache_base` the profile folders (`%APPDATA%`, `%LOCALAPPDATA%`).
/// The portable marker wins over the installed one (the installer deletes `portable`, so both
/// only exist when someone copied the marker in by hand, which asks for portable).
pub fn resolve(exe_dir: Option<&Path>, home: Option<PathBuf>, config_base: Option<PathBuf>, cache_base: Option<PathBuf>) -> Layout {
    if let Some(h) = home {
        return Layout { mode: InstallMode::Custom, config: h.join("config"), cache: h.join("cache") };
    }
    let has = |name: &str| exe_dir.is_some_and(|d| d.join(name).is_file());
    if let (true, Some(dir)) = (has(PORTABLE_MARKER), exe_dir) {
        let data = dir.join(PORTABLE_DATA_DIR);
        return Layout { mode: InstallMode::Portable, config: data.join("config"), cache: data.join("cache") };
    }
    let mode = if has(INSTALLED_MARKER) { InstallMode::Installed } else { InstallMode::Standalone };
    Layout {
        mode,
        config: config_base.map(|d| d.join(APP_DIR)).unwrap_or_else(|| PathBuf::from(".plexdio/config")),
        cache: cache_base.map(|d| d.join(APP_DIR)).unwrap_or_else(|| PathBuf::from(".plexdio/cache")),
    }
}

/// Folder of the running exe (None if Windows can't tell).
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

/// The layout of this process. `PLEXDIO_HOME` is read on every call (tests set it); the marker
/// lookup is done once.
pub fn layout() -> Layout {
    if let Some(h) = home_override() {
        return resolve(None, Some(h), None, None);
    }
    static MARKERS: OnceLock<Layout> = OnceLock::new();
    MARKERS.get_or_init(|| resolve(exe_dir().as_deref(), None, dirs::config_dir(), dirs::cache_dir())).clone()
}

pub fn install_mode() -> InstallMode {
    layout().mode
}

/// Move the config and cache folders of a pre-rename install (`Plexify`) to the new name, so
/// settings, the library cache and the saved session survive the rename. Call before anything
/// opens files under these folders (including the logger). Only for the profile folders: skipped
/// with `PLEXDIO_HOME` and for portable copies.
pub fn migrate_legacy_dirs() {
    if !matches!(install_mode(), InstallMode::Installed | InstallMode::Standalone) {
        return;
    }
    for base in [dirs::config_dir(), dirs::cache_dir()].into_iter().flatten() {
        let (old, new) = (base.join(LEGACY_APP_DIR), base.join(APP_DIR));
        if old.is_dir() && !new.exists() {
            // The logger is not up yet; a failure just means starting fresh.
            let _ = std::fs::rename(&old, &new);
        }
    }
}

fn home_override() -> Option<PathBuf> {
    std::env::var_os("PLEXDIO_HOME").filter(|v| !v.is_empty()).map(PathBuf::from)
}

pub fn config_dir() -> PathBuf {
    layout().config
}

pub fn cache_dir() -> PathBuf {
    layout().cache
}

pub fn log_dir() -> PathBuf {
    cache_dir().join("logs")
}

pub fn image_cache_dir() -> PathBuf {
    cache_dir().join("images")
}

pub fn audio_cache_dir() -> PathBuf {
    cache_dir().join("audio")
}

/// Neural-network models unpacked from the exe (Ride's beat tracker).
pub fn model_dir() -> PathBuf {
    cache_dir().join("models")
}

pub fn db_dir() -> PathBuf {
    cache_dir().join("db")
}

/// Total size in bytes of all regular files below `dir` (0 if missing).
pub fn dir_size(dir: &std::path::Path) -> u64 {
    let mut total = 0;
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    for entry in rd.flatten() {
        match entry.metadata() {
            Ok(m) if m.is_dir() => total += dir_size(&entry.path()),
            Ok(m) => total += m.len(),
            Err(_) => {}
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("plexdio-paths-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn profile() -> (Option<PathBuf>, Option<PathBuf>) {
        (Some(PathBuf::from("/profile/Roaming")), Some(PathBuf::from("/profile/Local")))
    }

    #[test]
    fn bare_exe_is_standalone_in_the_profile() {
        let exe = temp("bare");
        let (c, l) = profile();
        let got = resolve(Some(&exe), None, c, l);
        assert_eq!(got.mode, InstallMode::Standalone);
        assert_eq!(got.config, PathBuf::from("/profile/Roaming/Plexdio"));
        assert_eq!(got.cache, PathBuf::from("/profile/Local/Plexdio"));
    }

    #[test]
    fn installed_marker_keeps_the_profile_folders() {
        let exe = temp("installed");
        std::fs::write(exe.join(INSTALLED_MARKER), "").unwrap();
        let (c, l) = profile();
        let got = resolve(Some(&exe), None, c, l);
        assert_eq!(got.mode, InstallMode::Installed);
        assert_eq!(got.config, PathBuf::from("/profile/Roaming/Plexdio"));
        assert_eq!(got.cache, PathBuf::from("/profile/Local/Plexdio"));
    }

    #[test]
    fn portable_marker_puts_data_next_to_the_exe() {
        let exe = temp("portable");
        std::fs::write(exe.join(PORTABLE_MARKER), "").unwrap();
        let (c, l) = profile();
        let got = resolve(Some(&exe), None, c, l);
        assert_eq!(got.mode, InstallMode::Portable);
        assert_eq!(got.config, exe.join("Data").join("config"));
        assert_eq!(got.cache, exe.join("Data").join("cache"));
    }

    #[test]
    fn portable_wins_over_installed() {
        let exe = temp("both");
        std::fs::write(exe.join(PORTABLE_MARKER), "").unwrap();
        std::fs::write(exe.join(INSTALLED_MARKER), "").unwrap();
        let (c, l) = profile();
        assert_eq!(resolve(Some(&exe), None, c, l).mode, InstallMode::Portable);
    }

    #[test]
    fn a_marker_folder_is_not_a_marker() {
        let exe = temp("dir-marker");
        std::fs::create_dir_all(exe.join(PORTABLE_MARKER)).unwrap();
        let (c, l) = profile();
        assert_eq!(resolve(Some(&exe), None, c, l).mode, InstallMode::Standalone);
    }

    #[test]
    fn plexdio_home_overrides_every_marker() {
        let exe = temp("home");
        std::fs::write(exe.join(PORTABLE_MARKER), "").unwrap();
        let (c, l) = profile();
        let got = resolve(Some(&exe), Some(PathBuf::from("/h")), c, l);
        assert_eq!(got.mode, InstallMode::Custom);
        assert_eq!(got.config, PathBuf::from("/h/config"));
        assert_eq!(got.cache, PathBuf::from("/h/cache"));
    }

    #[test]
    fn no_profile_falls_back_to_a_local_folder() {
        let got = resolve(None, None, None, None);
        assert_eq!(got.mode, InstallMode::Standalone);
        assert_eq!(got.config, PathBuf::from(".plexdio/config"));
    }
}
