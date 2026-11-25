//! Self-update of a portable copy (the zip with a `portable` marker), as Screvia's portable
//! build does it, but simpler because Plexdio is one exe:
//!
//! 1. Download the zip into the exe's own folder, so the final rename stays on one volume.
//! 2. Check size and SHA-256 against the manifest ([`super::download`]) before opening it.
//! 3. Extract only `plexdio.exe` (as `plexdio.exe.new`). An entry with an absolute or `..` path
//!    rejects the whole package.
//! 4. Rename the running exe to `<exe>.old` (Windows allows renaming a running exe), move the new
//!    one into place. If that fails, the old exe is put back.
//! 5. The app relaunches with the same arguments and exits; the next start deletes `.old`.
//!
//! `Data\` and the `portable` marker are never touched.

use super::{download, manifest::VersionInfo, Result, UpdateError};
use std::io::Read;
use std::path::{Path, PathBuf};

/// The exe inside the portable zip.
pub const EXE_NAME: &str = "plexdio.exe";
/// Command-line flag of a relaunch after an update: wait for the old process to exit.
pub const AFTER_UPDATE_FLAG: &str = "--after-update";

/// Normalised entry name, or an error when it could write outside the target folder.
fn safe_name(name: &str) -> Result<String> {
    let n = name.replace('\\', "/");
    let bad = n.starts_with('/')
        || n.contains(':')
        || n.contains('\0')
        || n.split('/').any(|part| part == "..");
    if bad {
        return Err(UpdateError::BadPackage(format!("unsafe entry \"{name}\"")));
    }
    Ok(n)
}

/// Index of the `plexdio.exe` entry (at the top or in one `Plexdio/` folder). Every entry name
/// is checked; any unsafe one rejects the package. Other safe entries (the marker, README) are
/// ignored, never extracted.
pub fn pick_exe_entry(names: &[String]) -> Result<usize> {
    let mut found = None;
    for (i, name) in names.iter().enumerate() {
        let n = safe_name(name)?.to_ascii_lowercase();
        if n == EXE_NAME || n == format!("plexdio/{EXE_NAME}") {
            if found.is_some() {
                return Err(UpdateError::BadPackage("more than one plexdio.exe".into()));
            }
            found = Some(i);
        }
    }
    found.ok_or_else(|| UpdateError::BadPackage("no plexdio.exe in the zip".into()))
}

/// Writes the zip's `plexdio.exe` to `out`.
pub fn extract_exe(zip_path: &Path, out: &Path, max_bytes: u64) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| UpdateError::BadPackage(e.to_string()))?;
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index_raw(i).and_then(|e| e.name().map(|n| n.to_string())))
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| UpdateError::BadPackage(e.to_string()))?;
    let idx = pick_exe_entry(&names)?;
    let mut entry = zip.by_index(idx).map_err(|e| UpdateError::BadPackage(e.to_string()))?;
    let mut bytes = Vec::new();
    (&mut entry).take(max_bytes + 1).read_to_end(&mut bytes).map_err(|e| UpdateError::BadPackage(e.to_string()))?;
    if bytes.len() as u64 > max_bytes || !bytes.starts_with(b"MZ") {
        return Err(UpdateError::BadPackage("plexdio.exe is not a Windows program".into()));
    }
    std::fs::write(out, bytes)?;
    Ok(())
}

/// `<exe>.old` next to `exe`.
pub fn old_path(exe: &Path) -> PathBuf {
    sibling(exe, ".old")
}

fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let mut name = exe.file_name().map(|n| n.to_os_string()).unwrap_or_else(|| EXE_NAME.into());
    name.push(suffix);
    exe.with_file_name(name)
}

/// Puts `new_exe` in the place of `exe`, keeping the old one as `<exe>.old`. On failure the old
/// exe is restored.
pub fn swap_in(exe: &Path, new_exe: &Path) -> Result<()> {
    let old = old_path(exe);
    if old.exists() {
        std::fs::remove_file(&old)?;
    }
    std::fs::rename(exe, &old)?;
    if let Err(e) = std::fs::rename(new_exe, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(e.into());
    }
    Ok(())
}

/// Undoes [`swap_in`] (when the relaunch fails): the `.old` exe goes back in place.
pub fn rollback(exe: &Path) -> Result<()> {
    let old = old_path(exe);
    if !old.exists() {
        return Ok(());
    }
    if exe.exists() {
        std::fs::remove_file(exe)?;
    }
    std::fs::rename(&old, exe)?;
    Ok(())
}

/// Removes what an update leaves next to the exe: `<exe>.old` (after the restart), `<exe>.new`
/// and partial downloads (`Plexdio-*-portable.zip.partial`). A complete zip is left alone: it
/// may be the user's own download. Returns false while a file can't be deleted yet (the old
/// process may still be exiting).
pub fn cleanup(exe: &Path) -> bool {
    let mut all = true;
    for p in [old_path(exe), sibling(exe, ".new")] {
        if p.exists() && std::fs::remove_file(&p).is_err() {
            all = false;
        }
    }
    if let Some(dir) = exe.parent() {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("Plexdio-") && name.ends_with("-portable.zip.partial") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }
    all
}

/// Can we create files in `dir`?
pub fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".plexdio-write-test-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Steps 1–4 for the running exe `exe`. On success the new exe is in place and the caller
/// relaunches (and calls [`rollback`] if that fails).
pub fn apply(release: &VersionInfo, url: &str, exe: &Path, progress: &mut dyn FnMut(u64, u64)) -> Result<()> {
    let dir = exe.parent().ok_or(UpdateError::NotWritable)?;
    if !is_writable(dir) {
        return Err(UpdateError::NotWritable);
    }
    let zip_name = download::file_name_from_url(url)
        .filter(|n| n.to_ascii_lowercase().ends_with(".zip"))
        .unwrap_or_else(|| format!("Plexdio-{}-win-x64-portable.zip", release.version));
    let zip_path = dir.join(zip_name);
    download::download(url, release.size, &release.sha256, &zip_path, progress)?;
    let new_exe = sibling(exe, ".new");
    let extracted = extract_exe(&zip_path, &new_exe, super::manifest::MAX_PACKAGE_BYTES);
    let _ = std::fs::remove_file(&zip_path);
    if let Err(e) = extracted {
        let _ = std::fs::remove_file(&new_exe);
        return Err(e);
    }
    let swapped = swap_in(exe, &new_exe);
    if swapped.is_err() {
        let _ = std::fs::remove_file(&new_exe);
    }
    swapped
}

/// Arguments for the relaunched exe: this process's arguments (without an earlier
/// `--after-update`) plus `--after-update`.
pub fn relaunch_args(current: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut args: Vec<String> = current.into_iter().filter(|a| a != AFTER_UPDATE_FLAG).collect();
    args.push(AFTER_UPDATE_FLAG.into());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("plexdio-portable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn finds_the_exe_in_the_release_layout() {
        let n = names(&["Plexdio/", "Plexdio/plexdio.exe", "Plexdio/portable", "Plexdio/README.txt", "Plexdio/LICENSE.txt"]);
        assert_eq!(pick_exe_entry(&n).unwrap(), 1);
        assert_eq!(pick_exe_entry(&names(&["PLEXDIO.EXE"])).unwrap(), 0);
    }

    #[test]
    fn path_traversal_rejects_the_package() {
        for evil in ["../plexdio.exe", "Plexdio/../../x.dll", "/plexdio.exe", "C:/Windows/x.exe", "..\\plexdio.exe", "a\\..\\..\\b"] {
            let n = names(&["Plexdio/plexdio.exe", evil]);
            assert!(matches!(pick_exe_entry(&n), Err(UpdateError::BadPackage(_))), "{evil}");
        }
    }

    #[test]
    fn missing_or_duplicate_exe_is_rejected() {
        assert!(pick_exe_entry(&names(&["Plexdio/README.txt"])).is_err());
        assert!(pick_exe_entry(&names(&["Plexdio/other/plexdio.exe"])).is_err());
        assert!(pick_exe_entry(&names(&["plexdio.exe", "Plexdio/plexdio.exe"])).is_err());
    }

    #[test]
    fn extracts_only_the_exe() {
        let d = temp("extract");
        let zip = d.join("p.zip");
        make_zip(&zip, &[("Plexdio/plexdio.exe", b"MZ new exe"), ("Plexdio/portable", b""), ("Plexdio/README.txt", b"hi")]);
        let out = d.join("plexdio.exe.new");
        extract_exe(&zip, &out, 1 << 20).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"MZ new exe");
        assert!(!d.join("Plexdio").exists() && !d.join("README.txt").exists());
    }

    #[test]
    fn a_non_program_is_refused() {
        let d = temp("notexe");
        let zip = d.join("p.zip");
        make_zip(&zip, &[("Plexdio/plexdio.exe", b"#!/bin/sh")]);
        assert!(extract_exe(&zip, &d.join("x"), 1 << 20).is_err());
        std::fs::write(&zip, b"not a zip").unwrap();
        assert!(extract_exe(&zip, &d.join("x"), 1 << 20).is_err());
    }

    #[test]
    fn swap_keeps_data_and_marker_and_can_roll_back() {
        let d = temp("swap");
        let exe = d.join("plexdio.exe");
        std::fs::write(&exe, b"MZ old").unwrap();
        std::fs::write(d.join("portable"), b"").unwrap();
        std::fs::create_dir_all(d.join("Data/config")).unwrap();
        std::fs::write(d.join("Data/config/config.json"), b"{}").unwrap();
        let new = d.join("plexdio.exe.new");
        std::fs::write(&new, b"MZ new").unwrap();

        swap_in(&exe, &new).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"MZ new");
        assert_eq!(std::fs::read(old_path(&exe)).unwrap(), b"MZ old");
        assert!(!new.exists());
        assert!(d.join("portable").exists());
        assert_eq!(std::fs::read(d.join("Data/config/config.json")).unwrap(), b"{}");

        rollback(&exe).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"MZ old");
        assert!(!old_path(&exe).exists());
    }

    #[test]
    fn a_failed_swap_restores_the_old_exe() {
        let d = temp("swapfail");
        let exe = d.join("plexdio.exe");
        std::fs::write(&exe, b"MZ old").unwrap();
        // The new exe is missing: the second rename fails.
        assert!(swap_in(&exe, &d.join("missing.new")).is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"MZ old");
    }

    #[test]
    fn cleanup_removes_the_old_exe_and_leftovers() {
        let d = temp("cleanup");
        let exe = d.join("plexdio.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        std::fs::write(old_path(&exe), b"MZ").unwrap();
        std::fs::write(d.join("Plexdio-0.1.1-win-x64-portable.zip.partial"), b"x").unwrap();
        std::fs::write(d.join("Plexdio-0.1.0-win-x64-portable.zip"), b"the user's own download").unwrap();
        std::fs::write(d.join("portable"), b"").unwrap();
        assert!(cleanup(&exe));
        assert!(d.join("Plexdio-0.1.0-win-x64-portable.zip").exists());
        assert!(!old_path(&exe).exists());
        assert!(!d.join("Plexdio-0.1.1-win-x64-portable.zip.partial").exists());
        assert!(exe.exists() && d.join("portable").exists());
    }

    #[test]
    fn relaunch_args_add_the_flag_once() {
        let a = relaunch_args(["--update-url".to_string(), "http://h".into(), AFTER_UPDATE_FLAG.into()]);
        assert_eq!(a, vec!["--update-url", "http://h", AFTER_UPDATE_FLAG]);
    }
}
