#!/usr/bin/env python3
"""Regenerate every app icon from the Plexdio logo SVGs in assets/brand/.

    python3 tools/make_icons.py [assets-dir]

Needs Inkscape (SVG rendering) and Pillow. Writes, in the assets dir:

    icon.ico          16/24/32/48/64/256 frames (exe icon, Explorer, taskbar, tray)
    icon.png          256 px
    icon64.rgba       raw 64x64 RGBA, the window icon on Linux (no PNG decoder in the app)
    discord-icon.png  1024 px, for the Discord application's Rich Presence assets

Which SVG is used per size:

    16 px          brand/plexdio-16.svg   (pixel-aligned, solid glyph)
    24 px          brand/plexdio-24.svg   (pixel-aligned, solid glyph)
    32 px and up   brand/plexdio.svg      (the master)

The small variants exist because the master's outlined glyph and signal square turn to mush at 16 and
24 px. See docs/brand.md for the palette and geometry.
"""
import io
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ICO_SIZES = [16, 24, 32, 48, 64, 256]


def source_for(size, brand):
    if size <= 16:
        return brand / "plexdio-16.svg"
    if size <= 24:
        return brand / "plexdio-24.svg"
    return brand / "plexdio.svg"


def render(svg, size, tmp):
    out = Path(tmp) / f"{svg.stem}-{size}.png"
    subprocess.run(
        ["inkscape", str(svg), "--export-type=png", f"--export-filename={out}", "-w", str(size), "-h", str(size)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    img = Image.open(out).convert("RGBA")
    if img.size != (size, size):
        raise SystemExit(f"{svg} rendered at {img.size}, expected {size}x{size}")
    return img


def png_bytes(img):
    buf = io.BytesIO()
    img.save(buf, format="PNG", optimize=True)
    return buf.getvalue()


def ico_bytes(frames):
    """ICO with PNG-compressed frames (supported by Windows since Vista)."""
    header = struct.pack("<HHH", 0, 1, len(frames))
    offset = 6 + 16 * len(frames)
    entries = b""
    for size, data in frames:
        entries += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    return header + entries + b"".join(data for _, data in frames)


def main(assets):
    assets = Path(assets)
    brand = assets / "brand"
    if shutil.which("inkscape") is None:
        raise SystemExit("inkscape not found on PATH")
    with tempfile.TemporaryDirectory() as tmp:
        frames = [(s, png_bytes(render(source_for(s, brand), s, tmp))) for s in ICO_SIZES]
        (assets / "icon.ico").write_bytes(ico_bytes(frames))
        (assets / "icon.png").write_bytes(dict(frames)[256])
        rgba64 = render(source_for(64, brand), 64, tmp)
        (assets / "icon64.rgba").write_bytes(rgba64.tobytes())
        (assets / "discord-icon.png").write_bytes(png_bytes(render(brand / "plexdio.svg", 1024, tmp)))
    for name in ["icon.ico", "icon.png", "icon64.rgba", "discord-icon.png"]:
        print(f"wrote {assets / name} ({(assets / name).stat().st_size} bytes)")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "assets")
