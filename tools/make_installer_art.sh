#!/usr/bin/env bash
# Renders the Inno Setup wizard images (KruxSYS look, as Screvia's) from SVG. Needs inkscape and ImageMagick.
#   installer/wizard.svg       -> installer/wizard-100.bmp, wizard-200.bmp   (side image, 164x314 at 100 %)
#   assets/brand/plexdio.svg   -> installer/wizard-small-100.bmp, -200.bmp   (top-right mark, 55x58 at 100 %)
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
inst="$root/installer"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
render() { inkscape "$1" --export-type=png --export-filename="$2" -w "$3" -h "$4" >/dev/null 2>&1; }

render "$inst/wizard.svg" "$tmp/wiz100.png" 164 314
render "$inst/wizard.svg" "$tmp/wiz200.png" 328 628
convert "$tmp/wiz100.png" -background '#07090c' -flatten BMP3:"$inst/wizard-100.bmp"
convert "$tmp/wiz200.png" -background '#07090c' -flatten BMP3:"$inst/wizard-200.bmp"
render "$root/assets/brand/plexdio.svg" "$tmp/sm100.png" 55 55
render "$root/assets/brand/plexdio.svg" "$tmp/sm200.png" 110 110
convert "$tmp/sm100.png" -background white -gravity center -extent 55x58 -flatten BMP3:"$inst/wizard-small-100.bmp"
convert "$tmp/sm200.png" -background white -gravity center -extent 110x116 -flatten BMP3:"$inst/wizard-small-200.bmp"
echo "installer art regenerated"
