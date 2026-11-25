//! A stand-in for plexdio.kruxsys.com's release routes (site/nginx/templates/default.conf.template),
//! for testing the update flow where the real site is out of reach (the Windows test VM):
//!
//! `mockplex --releases DIR [--port N] [--bind ADDR]` serves a releases folder as written by
//! `./build.sh release --releases-dir DIR`:
//! - `/api/v1/version?channel=stable|beta&arch=x64|x64-setup` -> `DIR/updates/<channel>/<arch>.json`, or 404
//! - `/api/v1/releases` -> `DIR/api/releases.json`, or `[]`
//! - `/downloads/<ver>/<file>` -> the file, with Content-Length
//! - `/download`, `/changelog` -> a tiny HTML page (the app opens these in the browser)
//! - `/healthz` -> `ok`

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tiny_http::{Header, Response, Server};

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).expect("valid header")
}

fn query(url: &str, key: &str) -> String {
    url.split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| urlencoding::decode(v).map(|s| s.into_owned()).unwrap_or_default())
        .unwrap_or_default()
}

fn json(status: u16, body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body.into_bytes())
        .with_status_code(status)
        .with_header(header("Content-Type", "application/json"))
        .with_header(header("Cache-Control", "no-cache"))
}

/// Maps a request to a response. Separate from the socket loop for tests.
fn route(dir: &Path, url: &str) -> Result<Response<std::io::Cursor<Vec<u8>>>, PathBuf> {
    let path = url.split('?').next().unwrap_or("");
    match path {
        "/healthz" => Ok(Response::from_string("ok\n")),
        "/api/v1/version" => {
            let ch = match query(url, "channel").as_str() {
                "" | "stable" => "stable",
                "beta" => "beta",
                _ => return Ok(json(404, r#"{"error":"channel must be stable or beta."}"#.into())),
            };
            let arch = match query(url, "arch").as_str() {
                "" | "x64" => "x64",
                "x64-setup" => "x64-setup",
                _ => return Ok(json(404, r#"{"error":"arch must be x64 or x64-setup."}"#.into())),
            };
            match std::fs::read_to_string(dir.join("updates").join(ch).join(format!("{arch}.json"))) {
                Ok(body) => Ok(json(200, body)),
                Err(_) => Ok(json(404, r#"{"error":"No release has been published for this channel and package."}"#.into())),
            }
        }
        "/api/v1/releases" => Ok(json(200, std::fs::read_to_string(dir.join("api/releases.json")).unwrap_or_else(|_| "[]".into()))),
        "/download" | "/changelog" => Ok(Response::from_string(format!(
            "<!doctype html><title>Plexdio {}</title><h1>Plexdio {} (mockplex release server)</h1>",
            &path[1..],
            &path[1..]
        ))
        .with_header(header("Content-Type", "text/html; charset=utf-8"))),
        p if p.starts_with("/downloads/") => {
            let rel = &p["/downloads/".len()..];
            let rel = urlencoding::decode(rel).map(|s| s.into_owned()).unwrap_or_default();
            if rel.split('/').any(|s| s.is_empty() || s == ".." || s.starts_with('.')) || rel.contains('\\') {
                return Ok(Response::from_string("not found").with_status_code(404));
            }
            Err(dir.join("downloads").join(rel))
        }
        _ => Ok(Response::from_string("not found").with_status_code(404)),
    }
}

/// Starts the release server on a background thread; returns its base URL.
pub fn serve(dir: PathBuf, bind: &str, port: u16, verbose: bool) -> std::io::Result<String> {
    let server = Arc::new(Server::http((bind, port)).map_err(|e| std::io::Error::other(e.to_string()))?);
    let url = format!("http://{}", server.server_addr().to_ip().map(|a| a.to_string()).unwrap_or_default());
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let url = req.url().to_string();
            let result = match route(&dir, &url) {
                Ok(resp) => {
                    let code = resp.status_code().0;
                    req.respond(resp).map(|_| code)
                }
                Err(file) => match std::fs::File::open(&file) {
                    Ok(f) => {
                        let ctype = match file.extension().and_then(|e| e.to_str()) {
                            Some("exe") => "application/vnd.microsoft.portable-executable",
                            Some("zip") => "application/zip",
                            _ => "text/plain; charset=utf-8",
                        };
                        req.respond(Response::from_file(f).with_header(header("Content-Type", ctype))).map(|_| 200)
                    }
                    Err(_) => req.respond(Response::from_string("not found").with_status_code(404)).map(|_| 404),
                },
            };
            if verbose {
                println!("releases: {url} -> {}", result.map(|c| c.to_string()).unwrap_or_else(|e| e.to_string()));
            }
        }
    });
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_like_the_website() {
        let d = std::env::temp_dir().join(format!("mockplex-releases-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("updates/stable")).unwrap();
        std::fs::write(d.join("updates/stable/x64.json"), "{\"version\":\"0.1.1\"}").unwrap();
        let status = |u: &str| route(&d, u).map(|r| r.status_code().0).unwrap_or(0);
        assert_eq!(status("/api/v1/version?channel=stable&arch=x64"), 200);
        assert_eq!(status("/api/v1/version?channel=stable&arch=x64-setup"), 404);
        assert_eq!(status("/api/v1/version?channel=../x&arch=x64"), 404);
        assert_eq!(status("/api/v1/releases"), 200);
        assert_eq!(status("/downloads/../secret"), 404);
        assert!(route(&d, "/downloads/0.1.1/a.zip").is_err(), "a file is served from disk");
        assert_eq!(status("/healthz"), 200);
    }
}
