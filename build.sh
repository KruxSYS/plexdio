#!/usr/bin/env bash
# Builds, tests and releases Plexdio. The version comes from one place: [workspace.package] version in Cargo.toml.
#
#   ./build.sh                 release exe -> dist/plexdio.exe (no installer, nothing published)
#   ./build.sh test            run the test suite (host)
#   ./build.sh release [opts]  build and stage a release, then publish it (see below)
#   ./build.sh publish [opts]  publish an already staged release into the website's releases volume
#
# release: refuses a version that is already staged; needs a "## [x.y.z] - YYYY-MM-DD" section in CHANGELOG.md.
#   1. cross-builds the release exe and checks its imports (system DLLs only, tools/allowed-imports.txt)
#   2. Plexdio-<ver>-win-x64-portable.zip  (Plexdio/plexdio.exe + portable marker + README.txt + LICENSE.txt)
#   3. Plexdio-<ver>-win-x64-setup.exe     (installer/plexdio.iss, Inno Setup 7 under Wine: $ISCC)
#   4. SHA-256, sizes, NOTES.md and the update manifests in releases/ (a mirror of the volume; git-ignored)
#   5. copies the two packages and SHA256SUMS to dist/
#   6. publishes into the volume (as `publish`), unless --no-publish
#   options: --channel stable|beta (stable)  --released YYYY-MM-DD (the CHANGELOG date)
#            --volume NAME  --no-publish  --releases-dir DIR (releases/)
#
# publish: copies releases/ into a Docker volume through a throwaway alpine container without network (the files
#   travel as a tar stream, so a remote Docker daemon works too). Binaries first, then api/releases.json, then the
#   update manifests, each via a temporary file and mv. Refuses a version the volume already has.
#   options: --volume NAME (live-plexmusicplayer_plexdio-releases)  --version X.Y.Z (Cargo.toml)  --releases-dir DIR
#   Volumes: live-plexmusicplayer_plexdio-releases (live site), stage-plexmusicplayer_plexdio-releases (staging),
#            dev-plexmusicplayer_plexdio-releases (a local `docker compose up`).
set -euo pipefail
cd "$(dirname "$0")"
TARGET=x86_64-pc-windows-gnu
ALPINE="alpine:3.22@sha256:2e0b81d60597f449baa56dc25d355367869d2dca4cd57e866f1e2b52af3eb55f"
DEFAULT_VOLUME=live-plexmusicplayer_plexdio-releases
PY="python3 -I tools/release_manifest.py"

build_exe() {
  cargo build --release --target "$TARGET" -p plexdio
  EXE="target/$TARGET/release/plexdio.exe"
  file "$EXE"
  ls -l "$EXE" | awk '{printf "size: %.2f MB\n", $5/1048576}'
  echo "imports:"
  local imports
  imports=$(x86_64-w64-mingw32-objdump -p "$EXE" | awk '/DLL Name/ {print $3}' | sort -fu)
  echo "$imports" | sed 's/^/  /'
  # Every import must be on the allow-list (system DLLs only; see the comments in the file).
  local bad
  bad=$(echo "$imports" | grep -v -i -x -F -f <(grep -v '^#' tools/allowed-imports.txt | sed '/^$/d') || true)
  if [ -n "$bad" ]; then
    echo "imports not on tools/allowed-imports.txt (system DLLs only):" >&2
    echo "$bad" | sed 's/^/  /' >&2
    exit 1
  fi
}

version() { $PY version Cargo.toml; }

volume_has() { # volume version -> 0 if downloads/<version> exists in the volume
  docker run --rm --network none -v "$1:/dest:ro" "$ALPINE" test -e "/dest/downloads/$2"
}

publish() {
  local volume=$DEFAULT_VOLUME ver="" rel=releases
  while [ $# -gt 0 ]; do
    case "$1" in
      --volume) volume=$2; shift 2 ;;
      --version) ver=$2; shift 2 ;;
      --releases-dir) rel=$2; shift 2 ;;
      *) echo "publish: unknown option $1" >&2; exit 2 ;;
    esac
  done
  [ -n "$ver" ] || ver=$(version)
  [[ $volume =~ ^[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] || { echo "bad volume name: $volume" >&2; exit 2; }
  [ -d "$rel/downloads/$ver" ] || { echo "$ver is not staged in $rel/ (run ./build.sh release first)" >&2; exit 1; }
  command -v docker >/dev/null || { echo "docker is required to publish" >&2; exit 1; }
  docker volume inspect "$volume" >/dev/null 2>&1 || {
    echo "volume $volume does not exist: start the site first (its compose stack creates it)" >&2; exit 1; }
  if volume_has "$volume" "$ver"; then
    echo "$ver is already published in $volume; a published version is never replaced (bump the version)" >&2
    exit 1
  fi
  # The tar holds this version's files plus the current api/ and updates/ (derived from all staged releases).
  tar -C "$rel" -cf - "downloads/$ver" api updates | docker run --rm -i --network none -v "$volume:/dest" "$ALPINE" sh -eu -c '
    v="$1"
    mkdir -p /in && tar -C /in -xf -
    [ ! -e "/dest/downloads/$v" ] || { echo "already published: $v" >&2; exit 1; }
    mkdir -p /dest/downloads /dest/api
    cp -R "/in/downloads/$v" "/dest/downloads/.$v.tmp"
    mv "/dest/downloads/.$v.tmp" "/dest/downloads/$v"
    cp /in/api/releases.json /dest/api/.releases.json.tmp
    mv /dest/api/.releases.json.tmp /dest/api/releases.json
    for f in $(cd /in && find updates -name "*.json"); do
      mkdir -p "/dest/$(dirname "$f")"
      cp "/in/$f" "/dest/$f.tmp"
      mv "/dest/$f.tmp" "/dest/$f"
    done
    chmod -R a+rX /dest' sh "$ver"
  echo "published Plexdio $ver to volume $volume"
}

release() {
  local channel=stable released="" volume=$DEFAULT_VOLUME publish_it=1 rel=releases
  while [ $# -gt 0 ]; do
    case "$1" in
      --channel) channel=$2; shift 2 ;;
      --released) released=$2; shift 2 ;;
      --volume) volume=$2; shift 2 ;;
      --no-publish) publish_it=0; shift ;;
      --releases-dir) rel=$2; shift 2 ;;
      *) echo "release: unknown option $1" >&2; exit 2 ;;
    esac
  done
  local ver; ver=$(version)
  if [ -e "$rel/downloads/$ver" ]; then
    echo "Plexdio $ver is already released ($rel/downloads/$ver). Bump [workspace.package] version in Cargo.toml" >&2
    echo "and add a CHANGELOG section for it; to publish the existing $ver elsewhere use ./build.sh publish." >&2
    exit 1
  fi
  $PY notes CHANGELOG.md "$ver" > /dev/null   # fails early without a CHANGELOG section
  [ -n "$released" ] || released=$($PY date CHANGELOG.md "$ver")
  local iscc="${ISCC:-$HOME/.wine/drive_c/is7/ISCC.exe}"
  [ -f "$iscc" ] || { echo "Inno Setup not found at $iscc (set ISCC=…/ISCC.exe; see README, Releasing)" >&2; exit 1; }
  echo "== Plexdio $ver ($channel, $released)"

  build_exe
  local stage; stage=$(mktemp -d "${TMPDIR:-/tmp}/plexdio-release.XXXXXX")
  trap 'rm -rf "$stage"' RETURN
  mkdir -p "$stage/pkg" "$stage/out"
  cp "$EXE" "$stage/pkg/plexdio.exe"
  cp LICENSE "$stage/pkg/LICENSE.txt"
  sed "s/@VERSION@/$ver/g" tools/portable-readme.txt | sed 's/$/\r/' > "$stage/pkg/README.txt"
  # Portable marker (as Screvia's zip): settings and caches go to Data\ next to plexdio.exe.
  : > "$stage/pkg/portable"
  local zip="$stage/out/Plexdio-$ver-win-x64-portable.zip"
  $PY zip "$zip" "$stage/pkg" Plexdio "$released"

  # The installer gets the same files without the marker (it writes its own "installed" marker).
  mkdir -p "$stage/setup-src"
  cp "$stage/pkg/plexdio.exe" "$stage/pkg/README.txt" "$stage/pkg/LICENSE.txt" "$stage/setup-src/"
  WINEDEBUG=-all wine "$iscc" /Qp "/DSourceDir=$(winepath -w "$stage/setup-src")" "/DAppVersion=$ver" \
    "/O$(winepath -w "$stage/out")" "$(winepath -w installer/plexdio.iss)"
  local setup="$stage/out/Plexdio-$ver-win-x64-setup.exe"
  [ -f "$setup" ] || { echo "the installer was not built" >&2; exit 1; }

  $PY notes CHANGELOG.md "$ver" > "$stage/notes.md"
  $PY stage "$rel" "$ver" "$channel" "$released" "$stage/notes.md" "$setup" "$zip"
  mkdir -p dist
  cp "$rel/downloads/$ver/"*.exe "$rel/downloads/$ver/"*.zip dist/
  cp "$rel/downloads/$ver/SHA256SUMS" "dist/SHA256SUMS"
  cp "$EXE" dist/plexdio.exe
  echo "staged in $rel/downloads/$ver and dist/"
  if [ "$publish_it" = 1 ]; then
    publish --volume "$volume" --version "$ver" --releases-dir "$rel"
  else
    echo "not published (--no-publish): ./build.sh publish --volume NAME"
  fi
}

case "${1:-dist}" in
  test) cargo test --workspace ;;
  dist)
    build_exe
    mkdir -p dist
    cp "$EXE" dist/plexdio.exe
    cp assets/icon.ico dist/plexdio.ico
    ;;
  release) shift; release "$@" ;;
  publish) shift; publish "$@" ;;
  *) echo "usage: $0 [dist|test|release [options]|publish [options]]" >&2; exit 2 ;;
esac
