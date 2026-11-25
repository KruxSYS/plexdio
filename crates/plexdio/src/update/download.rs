//! Package downloads, verified against the manifest's size and SHA-256 before they are used.
//!
//! The file is written as `<name>.partial` and renamed only after both checks pass. On any
//! mismatch or error the partial file is deleted, so a bad download never lies around under its
//! final name.

use super::{check::download_agent, Result, UpdateError};
use ring::digest;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Lower-case hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(digest::digest(&digest::SHA256, bytes).as_ref())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Copies `src` into `dst`, refusing more than `expected_size` bytes, and checks the total size
/// and SHA-256 at the end. `progress(done, total)` is called as data arrives.
pub fn copy_verified(
    mut src: impl Read,
    mut dst: impl Write,
    expected_size: u64,
    expected_sha256: &str,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<()> {
    let mut ctx = digest::Context::new(&digest::SHA256);
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = src.read(&mut buf).map_err(|e| UpdateError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > expected_size {
            return Err(UpdateError::SizeMismatch { got: total, expected: expected_size });
        }
        ctx.update(&buf[..n]);
        dst.write_all(&buf[..n])?;
        progress(total, expected_size);
    }
    dst.flush()?;
    if total != expected_size {
        return Err(UpdateError::SizeMismatch { got: total, expected: expected_size });
    }
    if !hex(ctx.finish().as_ref()).eq_ignore_ascii_case(expected_sha256.trim()) {
        return Err(UpdateError::HashMismatch);
    }
    Ok(())
}

/// Checks a file already on disk (e.g. before running a downloaded installer again).
pub fn verify_file(path: &Path, expected_size: u64, expected_sha256: &str) -> Result<()> {
    let f = std::fs::File::open(path)?;
    copy_verified(f, std::io::sink(), expected_size, expected_sha256, &mut |_, _| {})
}

/// Downloads `url` to `dest`, verified. Blocking.
pub fn download(
    url: &str,
    expected_size: u64,
    expected_sha256: &str,
    dest: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<PathBuf> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = partial_path(dest);
    let result = (|| {
        let resp = download_agent().get(url).call().map_err(|e| UpdateError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            return Err(UpdateError::Http(status));
        }
        if let Some(len) = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok()) {
            if len != expected_size {
                return Err(UpdateError::SizeMismatch { got: len, expected: expected_size });
            }
        }
        let file = std::fs::File::create(&partial)?;
        copy_verified(resp.into_body().into_reader(), std::io::BufWriter::new(file), expected_size, expected_sha256, progress)?;
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }
        std::fs::rename(&partial, dest)?;
        Ok(dest.to_path_buf())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".partial");
    dest.with_file_name(name)
}

/// The last path segment of a download URL, kept only if it is a plain file name.
pub fn file_name_from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let name = urlencoding::decode(path.rsplit('/').next()?).ok()?.into_owned();
    let ok = !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    ok.then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"Plexdio test package";

    #[test]
    fn known_sha256() {
        assert_eq!(sha256_hex(b"test"), "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
    }

    #[test]
    fn verify_success() {
        let mut out = Vec::new();
        let mut seen = 0;
        copy_verified(DATA, &mut out, DATA.len() as u64, &sha256_hex(DATA), &mut |d, _| seen = d).unwrap();
        assert_eq!(out, DATA);
        assert_eq!(seen, DATA.len() as u64);
        // Upper-case hex in a manifest is fine too.
        copy_verified(DATA, std::io::sink(), DATA.len() as u64, &sha256_hex(DATA).to_uppercase(), &mut |_, _| {}).unwrap();
    }

    #[test]
    fn verify_fails_on_a_different_hash() {
        let mut wrong = sha256_hex(DATA);
        wrong.replace_range(0..1, if wrong.starts_with('0') { "1" } else { "0" });
        let e = copy_verified(DATA, std::io::sink(), DATA.len() as u64, &wrong, &mut |_, _| {}).unwrap_err();
        assert!(matches!(e, UpdateError::HashMismatch), "{e}");
    }

    #[test]
    fn verify_fails_on_short_or_long_data() {
        let sha = sha256_hex(DATA);
        let e = copy_verified(DATA, std::io::sink(), DATA.len() as u64 + 1, &sha, &mut |_, _| {}).unwrap_err();
        assert!(matches!(e, UpdateError::SizeMismatch { .. }), "{e}");
        let e = copy_verified(DATA, std::io::sink(), DATA.len() as u64 - 1, &sha, &mut |_, _| {}).unwrap_err();
        assert!(matches!(e, UpdateError::SizeMismatch { .. }), "{e}");
    }

    #[test]
    fn verify_file_on_disk() {
        let p = std::env::temp_dir().join(format!("plexdio-verify-{}.bin", std::process::id()));
        std::fs::write(&p, DATA).unwrap();
        assert!(verify_file(&p, DATA.len() as u64, &sha256_hex(DATA)).is_ok());
        assert!(matches!(verify_file(&p, DATA.len() as u64, &"0".repeat(64)), Err(UpdateError::HashMismatch)));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn file_names_from_urls() {
        assert_eq!(
            file_name_from_url("http://h/downloads/0.1.1/Plexdio-0.1.1-win-x64-setup.exe?x=1").as_deref(),
            Some("Plexdio-0.1.1-win-x64-setup.exe")
        );
        assert_eq!(file_name_from_url("http://h/downloads/..%2F..%2Fevil.exe"), None);
        assert_eq!(file_name_from_url("http://h/downloads/"), None);
        assert_eq!(file_name_from_url("http://h/a%20b.exe"), None);
    }
}
