//! The update client against a local HTTP server that answers like plexdio.kruxsys.com (the
//! nginx routes in site/nginx): `/api/v1/version` manifests and `/downloads/…` files.

use plexdio::update::{check, download, portable, version, Channel, UpdateError};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

struct Server {
    base: String,
    _thread: std::thread::JoinHandle<()>,
}

/// Serves `manifest` (None = 404, like nginx with nothing published) and `files`.
fn serve(manifest: Option<String>, files: Vec<(String, Vec<u8>)>) -> Server {
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    let s = server.clone();
    let thread = std::thread::spawn(move || {
        for req in s.incoming_requests() {
            let url = req.url().to_string();
            let path = url.split('?').next().unwrap_or("").to_string();
            let ua = req.headers().iter().find(|h| h.field.equiv("User-Agent")).map(|h| h.value.to_string()).unwrap_or_default();
            assert!(ua.starts_with("Plexdio/"), "user agent {ua}");
            let resp = if path == "/api/v1/version" {
                assert!(url.contains("channel=stable") && url.contains("current=") && url.contains("arch="), "{url}");
                match &manifest {
                    Some(m) => tiny_http::Response::from_data(m.clone().into_bytes()).with_status_code(200),
                    None => tiny_http::Response::from_data(b"{\"error\":\"No release\"}".to_vec()).with_status_code(404),
                }
            } else if let Some((_, data)) = files.iter().find(|(p, _)| *p == path) {
                tiny_http::Response::from_data(data.clone()).with_status_code(200)
            } else {
                tiny_http::Response::from_data(b"not found".to_vec()).with_status_code(404)
            };
            let _ = req.respond(resp);
        }
    });
    Server { base, _thread: thread }
}

fn manifest(ver: &str, url: &str, data: &[u8], sha: Option<&str>) -> String {
    format!(
        r#"{{"version":"{ver}","channel":"stable","arch":"x64","released_at":"2026-11-25T00:00:00Z","notes_md":"- x",
            "download_url":"{url}","sha256":"{}","size":{},"min_supported":null}}"#,
        sha.map(str::to_string).unwrap_or_else(|| download::sha256_hex(data)),
        data.len()
    )
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("plexdio-upd-it-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn current() -> version::Version {
    version::parse("0.1.0").unwrap()
}

#[test]
fn nothing_published_is_not_an_error() {
    let srv = serve(None, vec![]);
    let out = check::check(&srv.base, Channel::Stable, &current(), "x64-setup").unwrap();
    assert_eq!(out.status, check::Status::NoReleasePublished);
    assert!(out.release.is_none());
}

#[test]
fn same_version_is_up_to_date() {
    let data = b"MZ setup".to_vec();
    let srv = serve(Some(manifest("0.1.0", "/downloads/0.1.0/s.exe", &data, None)), vec![]);
    let out = check::check(&srv.base, Channel::Stable, &current(), "x64-setup").unwrap();
    assert_eq!(out.status, check::Status::UpToDate);
}

#[test]
fn finds_an_update_and_downloads_it_verified_from_the_same_server() {
    let data = b"MZ pretend installer 0.1.1".to_vec();
    let path = "/downloads/0.1.1/Plexdio-0.1.1-win-x64-setup.exe";
    let srv = serve(Some(manifest("0.1.1", path, &data, None)), vec![(path.into(), data.clone())]);
    let out = check::check(&format!("{}/", srv.base), Channel::Stable, &current(), "x64-setup").unwrap();
    assert_eq!(out.status, check::Status::UpdateAvailable);
    let url = out.download_url.unwrap();
    assert_eq!(url, format!("{}{path}", srv.base), "relative URL resolves against the override");
    let rel = out.release.unwrap();
    let dest = temp("ok").join("Plexdio-0.1.1-win-x64-setup.exe");
    let mut last = 0;
    download::download(&url, rel.size, &rel.sha256, &dest, &mut |d, _| last = d).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    assert_eq!(last, data.len() as u64);
}

#[test]
fn a_tampered_hash_aborts_and_leaves_nothing() {
    let data = b"MZ pretend installer 0.1.1".to_vec();
    let path = "/downloads/0.1.1/x.exe";
    let srv = serve(Some(manifest("0.1.1", path, &data, Some(&"ab".repeat(32)))), vec![(path.into(), data)]);
    let out = check::check(&srv.base, Channel::Stable, &current(), "x64-setup").unwrap();
    let rel = out.release.unwrap();
    let dir = temp("bad");
    let dest = dir.join("x.exe");
    let e = download::download(&out.download_url.unwrap(), rel.size, &rel.sha256, &dest, &mut |_, _| {}).unwrap_err();
    assert!(matches!(e, UpdateError::HashMismatch), "{e}");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "no file or .partial left behind");
}

#[test]
fn a_size_that_does_not_match_the_manifest_aborts() {
    let data = b"MZ 12345".to_vec();
    let path = "/downloads/0.1.1/x.exe";
    let mut m = manifest("0.1.1", path, &data, None);
    m = m.replace(&format!("\"size\":{}", data.len()), "\"size\":4");
    let srv = serve(Some(m), vec![(path.into(), data)]);
    let out = check::check(&srv.base, Channel::Stable, &current(), "x64").unwrap();
    let rel = out.release.unwrap();
    let e = download::download(&out.download_url.unwrap(), rel.size, &rel.sha256, &temp("size").join("x.exe"), &mut |_, _| {}).unwrap_err();
    assert!(matches!(e, UpdateError::SizeMismatch { .. }), "{e}");
}

#[test]
fn a_broken_manifest_is_an_error() {
    let srv = serve(Some("{\"version\":\"0.1.1\"".into()), vec![]);
    assert!(matches!(check::check(&srv.base, Channel::Stable, &current(), "x64"), Err(UpdateError::Manifest(_))));
}

#[test]
fn an_unreachable_server_is_a_network_error() {
    // A port nobody listens on.
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    drop(l);
    assert!(matches!(check::check(&base, Channel::Stable, &current(), "x64"), Err(UpdateError::Network(_))));
}

#[test]
fn portable_update_swaps_the_exe_and_keeps_data() {
    // The release zip, as ./build.sh release lays it out.
    let mut zip_bytes = Vec::new();
    {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_bytes));
        let o = zip::write::SimpleFileOptions::default();
        for (n, d) in [("Plexdio/plexdio.exe", &b"MZ plexdio 0.1.1"[..]), ("Plexdio/portable", b""), ("Plexdio/README.txt", b"read me")] {
            w.start_file(n, o).unwrap();
            w.write_all(d).unwrap();
        }
        w.finish().unwrap();
    }
    let path = "/downloads/0.1.1/Plexdio-0.1.1-win-x64-portable.zip";
    let srv = serve(Some(manifest("0.1.1", path, &zip_bytes, None)), vec![(path.into(), zip_bytes.clone())]);
    let out = check::check(&srv.base, Channel::Stable, &current(), "x64").unwrap();
    let rel = out.release.unwrap();

    let dir = temp("portable");
    let exe = dir.join("plexdio.exe");
    std::fs::write(&exe, b"MZ plexdio 0.1.0").unwrap();
    std::fs::write(dir.join("portable"), b"").unwrap();
    std::fs::create_dir_all(dir.join("Data/config")).unwrap();
    std::fs::write(dir.join("Data/config/config.json"), b"{\"volume\":0.5}").unwrap();

    portable::apply(&rel, &out.download_url.unwrap(), &exe, &mut |_, _| {}).unwrap();
    assert_eq!(std::fs::read(&exe).unwrap(), b"MZ plexdio 0.1.1");
    assert_eq!(std::fs::read(portable::old_path(&exe)).unwrap(), b"MZ plexdio 0.1.0");
    assert_eq!(std::fs::read(dir.join("Data/config/config.json")).unwrap(), b"{\"volume\":0.5}");
    assert!(dir.join("portable").exists());
    assert!(!dir.join("README.txt").exists() && !dir.join("Plexdio").exists(), "only the exe is extracted");
    assert!(!dir.join("Plexdio-0.1.1-win-x64-portable.zip").exists(), "the zip is removed");

    // Next start: the old exe goes.
    portable::cleanup(&exe);
    assert!(!portable::old_path(&exe).exists());
}

#[test]
fn portable_update_with_a_bad_hash_changes_nothing() {
    let data = b"PK not really".to_vec();
    let path = "/downloads/0.1.1/Plexdio-0.1.1-win-x64-portable.zip";
    let srv = serve(Some(manifest("0.1.1", path, &data, Some(&"cd".repeat(32)))), vec![(path.into(), data)]);
    let out = check::check(&srv.base, Channel::Stable, &current(), "x64").unwrap();
    let dir = temp("portable-bad");
    let exe = dir.join("plexdio.exe");
    std::fs::write(&exe, b"MZ plexdio 0.1.0").unwrap();
    let e = portable::apply(&out.release.unwrap(), &out.download_url.unwrap(), &exe, &mut |_, _| {}).unwrap_err();
    assert!(matches!(e, UpdateError::HashMismatch), "{e}");
    assert_eq!(std::fs::read(&exe).unwrap(), b"MZ plexdio 0.1.0");
    let left: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(left, vec![std::ffi::OsString::from("plexdio.exe")]);
}
