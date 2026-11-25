//! When automatic update checks run: on startup (after a short delay) and then at most once a
//! day, while "Automatically check for updates" is on. The time of the last check is kept in the
//! config, so restarting Plexdio does not check again.

/// Time between automatic checks.
pub const INTERVAL_SECS: u64 = 24 * 60 * 60;
/// Delay after startup before the first automatic check, so it never competes with sign-in and
/// the library load.
pub const STARTUP_DELAY_SECS: u64 = 10;
/// How often the background thread wakes to see whether a check is due (Plexdio can sit in the
/// tray for days).
pub const POLL_SECS: u64 = 60 * 60;

/// Is an automatic check due at `now` (unix seconds)? A last check in the future (the clock was
/// set back) counts as due, so a wrong clock can't stop checks for good.
pub fn is_due(enabled: bool, last_check: Option<u64>, now: u64) -> bool {
    if !enabled {
        return false;
    }
    match last_check {
        None => true,
        Some(last) if last > now => true,
        Some(last) => now - last >= INTERVAL_SECS,
    }
}

/// Seconds until the next automatic check is due (0 when overdue).
pub fn secs_until_due(last_check: Option<u64>, now: u64) -> u64 {
    match last_check {
        Some(last) if last <= now => INTERVAL_SECS.saturating_sub(now - last),
        _ => 0,
    }
}

/// Unix time now, in seconds.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: u64 = 1_764_000_000;

    #[test]
    fn first_run_is_due() {
        assert!(is_due(true, None, T));
    }

    #[test]
    fn at_most_once_a_day() {
        assert!(!is_due(true, Some(T), T));
        assert!(!is_due(true, Some(T), T + 60));
        assert!(!is_due(true, Some(T), T + INTERVAL_SECS - 1));
        assert!(is_due(true, Some(T), T + INTERVAL_SECS));
        assert!(is_due(true, Some(T), T + 3 * INTERVAL_SECS));
    }

    #[test]
    fn switched_off_never_checks() {
        assert!(!is_due(false, None, T));
        assert!(!is_due(false, Some(T - 10 * INTERVAL_SECS), T));
    }

    #[test]
    fn a_clock_set_back_does_not_block_checks() {
        assert!(is_due(true, Some(T + 1000), T));
        assert_eq!(secs_until_due(Some(T + 1000), T), 0);
    }

    #[test]
    fn time_until_the_next_check() {
        assert_eq!(secs_until_due(None, T), 0);
        assert_eq!(secs_until_due(Some(T), T), INTERVAL_SECS);
        assert_eq!(secs_until_due(Some(T), T + 3600), INTERVAL_SECS - 3600);
        assert_eq!(secs_until_due(Some(T), T + 2 * INTERVAL_SECS), 0);
    }
}
