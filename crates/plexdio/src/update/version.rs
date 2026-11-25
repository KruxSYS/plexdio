//! Semantic versions (semver.org). Pre-releases sort below their release (`0.2.0-beta.1 <
//! 0.2.0`); build metadata (`+abc`) is ignored when comparing.

pub use semver::Version;
use std::cmp::Ordering;

/// Parses `1.2.3`, `v1.2.3` or `1.2.3-beta.1`, with surrounding spaces allowed.
pub fn parse(s: &str) -> Option<Version> {
    let s = s.trim();
    Version::parse(s.strip_prefix('v').unwrap_or(s)).ok()
}

/// The version of this build (the workspace `Cargo.toml`).
pub fn current() -> Version {
    parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|| Version::new(0, 0, 0))
}

/// semver precedence: like `Ord`, but build metadata does not count.
pub fn compare(a: &Version, b: &Version) -> Ordering {
    a.cmp_precedence(b)
}

/// Is `latest` newer than `current`?
pub fn is_newer(latest: &Version, current: &Version) -> bool {
    compare(latest, current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        parse(s).unwrap()
    }

    #[test]
    fn numeric_parts_compare_as_numbers() {
        assert!(is_newer(&v("0.1.1"), &v("0.1.0")));
        assert!(is_newer(&v("0.10.0"), &v("0.9.9")));
        assert!(is_newer(&v("1.0.0"), &v("0.99.99")));
        assert!(!is_newer(&v("0.1.0"), &v("0.1.0")));
        assert!(!is_newer(&v("0.1.0"), &v("0.1.1")));
    }

    #[test]
    fn pre_releases_sort_below_the_release() {
        assert!(is_newer(&v("0.2.0"), &v("0.2.0-beta.2")));
        assert!(is_newer(&v("0.2.0-beta.2"), &v("0.2.0-beta.1")));
        assert!(is_newer(&v("0.2.0-beta.10"), &v("0.2.0-beta.9")));
        assert!(is_newer(&v("0.2.0-beta.1"), &v("0.1.9")));
        assert!(is_newer(&v("0.2.0-rc.1"), &v("0.2.0-beta.5")));
    }

    #[test]
    fn build_metadata_is_ignored() {
        assert_eq!(compare(&v("0.1.0+abc"), &v("0.1.0+def")), Ordering::Equal);
        assert!(!is_newer(&v("0.1.0+zzz"), &v("0.1.0")));
    }

    #[test]
    fn parse_accepts_a_v_prefix_and_rejects_junk() {
        assert_eq!(parse(" v1.2.3 "), Some(Version::new(1, 2, 3)));
        assert_eq!(parse("1.2"), None);
        assert_eq!(parse("latest"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("01.2.3"), None);
    }

    #[test]
    fn this_build_has_a_valid_version() {
        assert_eq!(current().to_string(), env!("CARGO_PKG_VERSION"));
    }
}
