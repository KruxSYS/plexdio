#!/usr/bin/env python3
"""Renders the site's pages: site/pages/<name>.html bodies wrapped in site/pages/_layout.html.

    python3 -I site/tools/render.py PAGES_DIR OUT_DIR VERSION

VERSION is the ?v= stamp on asset links; "auto" uses a hash of the static files already in OUT_DIR (so a
changed stylesheet or script gets a new link on every deploy).

Each page starts with comment lines that fill the layout:
    <!-- title: Plexdio — … -->
    <!-- description: … -->
    <!-- path: /download -->           (canonical path; default /<name>)
    <!-- scripts: download.js -->      (optional, space separated, from /assets/js/)
Run by the site image build (site/Dockerfile); standard library only.
"""
import hashlib
import html
import os
import re
import sys

META = re.compile(r"^\s*<!--\s*(title|description|path|scripts):\s*(.*?)\s*-->\s*$")


def content_hash(out):
    h = hashlib.sha256()
    for base, dirs, files in sorted(os.walk(out)):
        dirs.sort()
        for name in sorted(files):
            path = os.path.join(base, name)
            h.update(os.path.relpath(path, out).encode())
            with open(path, "rb") as f:
                h.update(f.read())
    return h.hexdigest()[:10]


def render(pages, out, version):
    if version == "auto":
        version = content_hash(out)
    layout = open(os.path.join(pages, "_layout.html"), encoding="utf-8").read()
    os.makedirs(out, exist_ok=True)
    count = 0
    for name in sorted(os.listdir(pages)):
        if not name.endswith(".html") or name.startswith("_"):
            continue
        meta, body = {}, []
        for line in open(os.path.join(pages, name), encoding="utf-8").read().splitlines():
            m = META.match(line)
            if m and not body:
                meta[m.group(1)] = m.group(2)
            else:
                body.append(line)
        for key in ("title", "description"):
            if key not in meta:
                sys.exit(f"render: {name} has no '<!-- {key}: … -->' line")
        stem = name[:-5]
        path = meta.get("path") or ("/" if stem == "index" else f"/{stem}")
        scripts = "".join(
            f'  <script src="/assets/js/{html.escape(s)}?v={version}" defer></script>\n'
            for s in meta.get("scripts", "").split()
        )
        page = (
            layout.replace("{{title}}", html.escape(meta["title"]))
            .replace("{{description}}", html.escape(meta["description"]))
            .replace("{{path}}", html.escape(path))
            .replace("{{scripts}}", scripts)
            .replace("{{version}}", html.escape(version))
            .replace("{{body}}", "\n".join(body).strip("\n"))
        )
        if "{{" in page:
            sys.exit(f"render: {name} left a {{{{placeholder}}}} unfilled")
        with open(os.path.join(out, name), "w", encoding="utf-8") as f:
            f.write(page)
        count += 1
    print(f"rendered {count} pages into {out}")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    render(*sys.argv[1:])
