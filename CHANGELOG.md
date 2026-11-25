# Changelog

All notable changes to Plexdio are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/). The version itself lives in one place: `[workspace.package]
version` in `Cargo.toml`.

`./build.sh release` copies the section of the version it builds into the release notes. It refuses to
build a version that has no `## [x.y.z] - YYYY-MM-DD` section here.

## [Unreleased]

## [0.1.0] - 2026-11-25

The first release published on [plexdio.kruxsys.com](https://plexdio.kruxsys.com). Plexdio is in development:
it has been tested against its own mock Plex server, never a real Plex server.

### Added

- **Installer and portable zip.** Plexdio now ships as an installer (per-user, no admin rights, Start menu
  shortcut, uninstaller, silent updates) and as a portable zip that keeps its settings and caches in a `Data`
  folder next to `plexdio.exe`.
- **Update checks.** Plexdio checks plexdio.kruxsys.com once a day for a new version (Settings → About, on by
  default) and can update itself: the installed copy runs the new installer silently, the portable copy swaps its
  exe. Every download is checked against its SHA-256 before anything is installed.
- Sign-in with the Plex PIN flow; the token is kept in the Windows Credential Manager.
- Offline-first library cache with full and delta sync, Home with Made for you mixes, instant search.
- Gapless playback, crossfade, ReplayGain/loudness normalisation and a 10-band equaliser.
- Lyrics, radio and autoplay, playlist editing, multi-select, downloads for offline listening and a mini-player.
- A now-playing visualiser with nine retro looks, and Ride, a game on the song that is playing.
- Cast to other Plex players (experimental).
