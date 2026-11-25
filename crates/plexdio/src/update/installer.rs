//! Update of an installed copy: run the downloaded Inno Setup installer unattended (Screvia's
//! switches), then exit. The installer closes a still-running Plexdio through the Restart
//! Manager, replaces the files and starts the new version (`[Run] … Check: WizardSilent` in
//! installer/plexdio.iss). Settings live in the profile, so they carry over.

use super::{download, manifest::VersionInfo, Result, UpdateError};
use std::path::{Path, PathBuf};

/// Inno Setup switches for an unattended update.
pub fn silent_args(log_file: &Path) -> Vec<String> {
    vec![
        "/VERYSILENT".into(),
        "/SUPPRESSMSGBOXES".into(),
        "/NORESTART".into(),
        "/CLOSEAPPLICATIONS".into(),
        "/SP-".into(),
        format!("/LOG={}", log_file.display()),
    ]
}

/// One command line, quoted the way Inno Setup reads it (`/LOG="C:\a b\x.log"`).
pub fn command_line(args: &[String]) -> String {
    args.iter()
        .map(|a| {
            if !a.contains(' ') {
                a.clone()
            } else if let Some(eq) = a.find('=') {
                format!("{}\"{}\"", &a[..=eq], &a[eq + 1..])
            } else {
                format!("\"{a}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `%TEMP%\Plexdio-update`.
pub fn work_dir() -> PathBuf {
    std::env::temp_dir().join("Plexdio-update")
}

/// Removes downloaded installers (and partial downloads) from [`work_dir`]; the setup logs stay.
/// Returns false while a file is still in use.
pub fn cleanup() -> bool {
    cleanup_in(&work_dir())
}

fn cleanup_in(dir: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else { return true };
    let mut all = true;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if (name.ends_with(".exe") || name.ends_with(".partial")) && std::fs::remove_file(e.path()).is_err() {
            all = false;
        }
    }
    all
}

/// Downloads and verifies the installer. Returns its path.
pub fn fetch(release: &VersionInfo, url: &str, progress: &mut dyn FnMut(u64, u64)) -> Result<PathBuf> {
    let name = download::file_name_from_url(url)
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .unwrap_or_else(|| format!("Plexdio-{}-win-x64-setup.exe", release.version));
    let dest = work_dir().join(name);
    download::download(url, release.size, &release.sha256, &dest, progress)?;
    if !std::fs::read(&dest).map(|b| b.starts_with(b"MZ")).unwrap_or(false) {
        let _ = std::fs::remove_file(&dest);
        return Err(UpdateError::BadPackage("the installer is not a Windows program".into()));
    }
    Ok(dest)
}

/// Starts `setup` unattended. ShellExecute, so an all-users installation can ask for elevation.
pub fn launch(setup: &Path, version: &str) -> Result<()> {
    let log = work_dir().join(format!("setup-{version}.log"));
    let params = command_line(&silent_args(&log));
    log::info!("update: starting {} {params}", setup.display());
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (op, file, p) = (wide("open"), wide(&setup.to_string_lossy()), wide(&params));
        let r = ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), file.as_ptr(), p.as_ptr(), std::ptr::null(), SW_SHOWNORMAL);
        if r as isize <= 32 {
            return Err(UpdateError::Io(format!("could not start the installer (code {})", r as isize)));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err(UpdateError::Io(format!("installers only run on Windows ({})", setup.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_keeps_only_the_logs() {
        let d = std::env::temp_dir().join(format!("plexdio-inst-clean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for f in ["Plexdio-0.1.1-win-x64-setup.exe", "x.exe.partial", "setup-0.1.1.log"] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        assert!(cleanup_in(&d));
        let left: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(left, vec![std::ffi::OsString::from("setup-0.1.1.log")]);
        assert!(cleanup_in(&d.join("missing")));
    }

    #[test]
    fn silent_switches_and_quoting() {
        let args = silent_args(Path::new("C:\\Users\\A B\\AppData\\Local\\Temp\\Plexdio-update\\setup-0.1.1.log"));
        assert!(args.contains(&"/VERYSILENT".to_string()));
        assert!(args.contains(&"/CLOSEAPPLICATIONS".to_string()));
        let line = command_line(&args);
        assert!(line.starts_with("/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS /SP- /LOG=\""), "{line}");
        assert!(line.ends_with("setup-0.1.1.log\""), "{line}");
        assert_eq!(command_line(&["a b".into(), "c".into()]), "\"a b\" c");
    }
}
