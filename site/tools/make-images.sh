#!/usr/bin/env bash
# Regenerates the site's images: screenshots from docs/*.png as WebP, the logos and favicons from
# assets/brand/*.svg and assets/icon.ico, and the social card. Needs ImageMagick (WebP) and inkscape.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
www="$root/site/static"
img="$www/assets/img"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
mkdir -p "$img"

# Screenshots (1280x900 app shots; see README "Screenshots"). WebP at q82, plus a 640 px wide copy for phones.
for name in home album artist search radio now-playing ride settings about login visualiser-looks ride-moments; do
  convert "$root/docs/$name.png" -strip -quality 82 -define webp:method=6 "$img/$name.webp"
  convert "$root/docs/$name.png" -strip -resize 640x -quality 80 -define webp:method=6 "$img/$name-640.webp"
done
convert "$root/docs/mini.png" -strip -quality 85 -define webp:method=6 "$img/mini.webp"

# Marks and favicons.
cp "$root/assets/brand/plexdio.svg" "$img/logo.svg"
cp "$root/assets/brand/kruxsys.svg" "$img/kruxsys.svg"
cp "$root/assets/brand/plexdio.svg" "$www/favicon.svg"
cp "$root/assets/icon.ico" "$www/favicon.ico"
inkscape "$root/assets/brand/plexdio.svg" --export-type=png --export-filename="$www/apple-touch-icon.png" -w 180 -h 180 >/dev/null 2>&1
inkscape "$root/site/tools/og-image.svg" --export-type=png --export-filename="$tmp/og.png" -w 1200 -h 630 >/dev/null 2>&1
convert "$tmp/og.png" -strip "$img/og-image.png"
command -v optipng >/dev/null && optipng -quiet -o2 "$www/apple-touch-icon.png" "$img/og-image.png" || true
echo "site images regenerated"; du -ch "$img"/*.webp | tail -1
