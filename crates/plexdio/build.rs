use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    println!("cargo:rerun-if-changed=app.manifest");
    build_info();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico")
            .set("ProductName", "Plexdio")
            .set("FileDescription", "Plexdio - a small, native Windows music player for Plex")
            .set("CompanyName", "KruxSYS")
            .set("LegalCopyright", "\u{a9} 2026 KruxSYS")
            .set_manifest_file("app.manifest");
        if let Err(e) = res.compile() {
            panic!("failed to compile Windows resources: {e}");
        }
    }
}

/// `PLEXDIO_BUILD_HASH` (short git commit, or "unknown") and `PLEXDIO_BUILD_DATE` (UTC, YYYY-MM-DD)
/// for Settings > About. `SOURCE_DATE_EPOCH` pins the date for reproducible builds.
fn build_info() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    println!("cargo:rustc-env=PLEXDIO_BUILD_HASH={hash}");
    println!("cargo:rustc-env=PLEXDIO_BUILD_DATE={y:04}-{m:02}-{d:02}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    // A new commit changes HEAD (detached) or the branch ref it points to.
    for p in ["../../.git/HEAD", "../../.git/refs/heads", "../../.git/packed-refs"] {
        if std::path::Path::new(p).exists() {
            println!("cargo:rerun-if-changed={p}");
        }
    }
}

/// Days since 1970-01-01 to a (year, month, day) civil date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}
