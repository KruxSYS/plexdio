#!/usr/bin/env python3
"""Release helper for ./build.sh release (run it with `python3 -I`; standard library only).

    release_manifest.py version CARGO_TOML               print [workspace.package] version
    release_manifest.py notes CHANGELOG VERSION          print the version's CHANGELOG section (fails if missing)
    release_manifest.py date CHANGELOG VERSION           print the section's date (YYYY-MM-DD)
    release_manifest.py zip OUT.zip SRC_DIR TOP_NAME DATE   deterministic zip of SRC_DIR as TOP_NAME/...
    release_manifest.py stage RELEASES_DIR VERSION CHANNEL RELEASED NOTES.md SETUP.exe PORTABLE.zip
    release_manifest.py manifests RELEASES_DIR            rebuild updates/* from api/releases.json

The releases folder mirrors what the website serves from the plexdio-releases volume:

    downloads/<ver>/<setup.exe>, <portable.zip>, SHA256SUMS, NOTES.md
    api/releases.json                         every package, newest first (GET /api/v1/releases)
    updates/<channel>/<arch>.json             newest package per channel (GET /api/v1/version)

Manifest entries use Screvia's VersionInfo fields (version, channel, arch, released_at, notes_md,
download_url, sha256, size, min_supported); releases.json adds file_name and notes_html.
"""
import hashlib
import html
import json
import os
import re
import shutil
import sys
import zipfile

SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$")
CHANNELS = ("stable", "beta")
ARCHES = ("x64", "x64-setup")


def die(msg):
    print(f"release_manifest: {msg}", file=sys.stderr)
    sys.exit(1)


def semver_key(v):
    m = SEMVER.match(v)
    if not m:
        die(f"not a semantic version: {v}")
    major, minor, patch, pre = m.groups()
    # A release sorts after its pre-releases; numeric identifiers compare as numbers.
    pre_key = (1,) if pre is None else (0,) + tuple(
        (0, int(p), "") if p.isdigit() else (1, 0, p) for p in pre.split(".")
    )
    return (int(major), int(minor), int(patch), pre_key)


def cargo_version(path):
    section = None
    for line in open(path, encoding="utf-8"):
        s = line.strip()
        if s.startswith("["):
            section = s
        elif section == "[workspace.package]" and s.startswith("version"):
            m = re.match(r'version\s*=\s*"([^"]+)"', s)
            if m:
                return m.group(1)
    die(f"no [workspace.package] version in {path}")


HEADING = re.compile(r"^## \[([^\]]+)\](?: - (\d{4}-\d{2}-\d{2}))?\s*$")


def changelog_section(path, version):
    """(date, markdown body) of `## [version] - date`."""
    lines = open(path, encoding="utf-8").read().splitlines()
    body, date, inside = [], None, False
    for line in lines:
        m = HEADING.match(line)
        if m:
            if inside:
                break
            if m.group(1) == version:
                inside, date = True, m.group(2)
            continue
        if inside:
            body.append(line)
    if not inside:
        die(f"CHANGELOG has no '## [{version}] - YYYY-MM-DD' section: add one before releasing")
    if not date:
        die(f"the CHANGELOG section of {version} has no date ('## [{version}] - YYYY-MM-DD')")
    text = "\n".join(body).strip()
    if not text:
        die(f"the CHANGELOG section of {version} is empty")
    return date, text


# --- Markdown subset -> HTML (release notes on the changelog page) -------------------------------
# Headings, paragraphs, bullet lists (one level, continuation lines), **bold**, *italic*, `code` and
# [links](https://...). Everything is HTML-escaped first; only http(s) and site-relative links survive.

LINK = re.compile(r"\[([^\]]+)\]\(([^)\s]+)\)")


def inline(text):
    out = html.escape(text, quote=True)
    codes = []

    def keep_code(m):
        codes.append(m.group(1))
        return f"\x00{len(codes) - 1}\x00"

    out = re.sub(r"`([^`]+)`", keep_code, out)

    def link(m):
        label, url = m.group(1), html.unescape(m.group(2))
        if re.match(r"^https?://", url) or url.startswith("/") or url.startswith("#"):
            return f'<a href="{html.escape(url, quote=True)}">{label}</a>'
        return label

    out = LINK.sub(link, out)
    out = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", out)
    out = re.sub(r"(?<![*\w])\*([^*]+)\*(?![*\w])", r"<em>\1</em>", out)
    out = re.sub(r"\x00(\d+)\x00", lambda m: f"<code>{codes[int(m.group(1))]}</code>", out)
    return out


def markdown_html(md):
    html_parts, para, items = [], [], []

    def flush_para():
        if para:
            html_parts.append(f"<p>{inline(' '.join(para))}</p>")
            para.clear()

    def flush_list():
        if items:
            html_parts.append("<ul>" + "".join(f"<li>{inline(' '.join(i))}</li>" for i in items) + "</ul>")
            items.clear()

    for raw in md.splitlines():
        line = raw.rstrip()
        stripped = line.strip()
        if not stripped:
            flush_para()
            flush_list()
            continue
        h = re.match(r"^(#{1,6})\s+(.*)$", stripped)
        if h:
            flush_para()
            flush_list()
            level = min(6, len(h.group(1)) + 1)  # the page uses h2 per release
            level = max(level, 3)
            html_parts.append(f"<h{level}>{inline(h.group(2))}</h{level}>")
            continue
        li = re.match(r"^[-*+]\s+(.*)$", stripped)
        if li:
            flush_para()
            items.append([li.group(1)])
            continue
        if items and raw[:1] in (" ", "\t"):
            items[-1].append(stripped)
            continue
        flush_list()
        para.append(stripped)
    flush_para()
    flush_list()
    return "\n".join(html_parts)


# --- Packages and manifests -----------------------------------------------------------------------

def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def make_zip(out, src_dir, top, date):
    y, m, d = (int(x) for x in date.split("-"))
    stamp = (y, m, d, 0, 0, 0)
    tmp = out + ".tmp"
    with zipfile.ZipFile(tmp, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        d_info = zipfile.ZipInfo(top + "/", stamp)
        d_info.external_attr = 0o40755 << 16
        z.writestr(d_info, b"")
        for name in sorted(os.listdir(src_dir)):
            path = os.path.join(src_dir, name)
            if not os.path.isfile(path):
                die(f"unexpected folder in the portable package: {path}")
            info = zipfile.ZipInfo(f"{top}/{name}", stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            with open(path, "rb") as f:
                z.writestr(info, f.read(), compresslevel=9)
    os.replace(tmp, out)


def write_json(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, path)


def load_releases(root):
    path = os.path.join(root, "api", "releases.json")
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, list):
        die(f"{path} is not a list")
    return data


VERSION_INFO_FIELDS = ("version", "channel", "arch", "released_at", "notes_md", "download_url", "sha256", "size", "min_supported")


def rebuild_manifests(root):
    """updates/<channel>/<arch>.json: stable = newest stable package; beta = newest of beta and stable."""
    releases = load_releases(root)
    for channel in CHANNELS:
        allowed = ("stable",) if channel == "stable" else ("stable", "beta")
        for arch in ARCHES:
            cands = [r for r in releases if r["arch"] == arch and r["channel"] in allowed]
            path = os.path.join(root, "updates", channel, f"{arch}.json")
            if not cands:
                if os.path.exists(path):
                    os.remove(path)
                continue
            best = max(cands, key=lambda r: semver_key(r["version"]))
            write_json(path, {k: best.get(k) for k in VERSION_INFO_FIELDS})


def stage(root, version, channel, released, notes_path, setup, portable):
    semver_key(version)
    if channel not in CHANNELS:
        die(f"channel must be one of {CHANNELS}")
    if not re.match(r"^\d{4}-\d{2}-\d{2}$", released):
        die("the release date must be YYYY-MM-DD")
    dest = os.path.join(root, "downloads", version)
    if os.path.exists(dest):
        die(f"{version} is already released ({dest}); bump the version in Cargo.toml")
    releases = load_releases(root)
    if any(r["version"] == version for r in releases):
        die(f"{version} is already in api/releases.json")
    notes_md = open(notes_path, encoding="utf-8").read().strip()
    notes_html = markdown_html(notes_md)
    os.makedirs(dest + ".tmp", exist_ok=True)
    entries = []
    for arch, src in (("x64-setup", setup), ("x64", portable)):
        name = os.path.basename(src)
        shutil.copyfile(src, os.path.join(dest + ".tmp", name))
        entries.append({
            "version": version,
            "channel": channel,
            "arch": arch,
            "released_at": f"{released}T00:00:00Z",
            "notes_md": notes_md,
            "download_url": f"/downloads/{version}/{name}",
            "sha256": sha256(src),
            "size": os.path.getsize(src),
            "min_supported": None,
            "file_name": name,
            "notes_html": notes_html,
        })
    with open(os.path.join(dest + ".tmp", "SHA256SUMS"), "w", encoding="utf-8") as f:
        for e in entries:
            f.write(f"{e['sha256']}  {e['file_name']}\n")
    with open(os.path.join(dest + ".tmp", "NOTES.md"), "w", encoding="utf-8") as f:
        f.write(f"# Plexdio {version} ({released})\n\n{notes_md}\n")
    os.replace(dest + ".tmp", dest)
    releases = entries + releases
    releases.sort(key=lambda r: (semver_key(r["version"]), r["arch"] == "x64-setup"), reverse=True)
    write_json(os.path.join(root, "api", "releases.json"), releases)
    rebuild_manifests(root)
    for e in entries:
        print(f"{e['arch']:10} {e['file_name']}  {e['size']} bytes  sha256 {e['sha256']}")


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd, args = argv[1], argv[2:]
    if cmd == "version" and len(args) == 1:
        v = cargo_version(args[0])
        semver_key(v)
        print(v)
    elif cmd == "notes" and len(args) == 2:
        print(changelog_section(args[0], args[1])[1])
    elif cmd == "date" and len(args) == 2:
        print(changelog_section(args[0], args[1])[0])
    elif cmd == "zip" and len(args) == 4:
        make_zip(*args)
    elif cmd == "stage" and len(args) == 7:
        stage(*args)
    elif cmd == "manifests" and len(args) == 1:
        rebuild_manifests(args[0])
    elif cmd == "render" and len(args) == 1:
        print(markdown_html(open(args[0], encoding="utf-8").read()))
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
