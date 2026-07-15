#!/usr/bin/env python3
"""Package the game's captured screenshots (and generate the section icons) into
the web site's asset folder.

The web site (`web/src/assets/`) references a set of screenshots that the capture
examples produce and a handful of tiny section icons. This script is the "move
the right files, at the right size, to the right place" step at the end of the
capture pipeline:

  1. It VALIDATES and COPIES each captured screenshot from a staging dir into
     `web/src/assets/`. Figures/heroes are 16:9 and copied as-is (the site sizes
     them with `aspect-ratio: 16/9; object-fit: cover`); thumbnails are 16:9 too.
     A file that is present but the wrong shape is a hard error - a silent bad
     asset is worse than a missing one. A shot not captured yet is reported and
     skipped, so this is safe to run before every shot exists.
  2. It GENERATES the five 44x44 section icons directly (simple flat diagram
     glyphs, matching the editor's per-section colours) - these are authored, not
     captured.

Capturing the screenshots (needs a display + a GPU; use Xvfb + lavapipe headless)
into the staging dir, then packaging them:

    NOVA_SHOT_DIR=target/reel BCS_REEL=1 \\
        cargo run --example 13_screenshot_reel --features debug
    NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \\
        cargo run --example 14_screenshot_ui --features debug
    NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \\
        cargo run --example 15_screenshot_combat --features debug
    python3 scripts/gen-web-screenshots.py            # stage -> web/src/assets

Run from the repo root. Uses only the Python standard library (no Pillow), like
`scripts/gen-placeholder-sounds.py`, so it needs no third-party package: PNG
dimensions are read from the IHDR header and the icons are written with a tiny
built-in PNG encoder.
"""

import argparse
import os
import shutil
import struct
import sys
import zlib

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_STAGE = os.path.join(REPO_ROOT, "target", "reel")
WEB_ASSETS = os.path.join(REPO_ROOT, "web", "src", "assets")

# Every screenshot the site references, grouped by kind. `stage` is the filename
# the capture examples write; `web` is the destination name (identical here).
# 16:9 figures and thumbnails are copied as-is (the site sizes them). The example
# that produces each is noted so it is obvious what to run.
#
# Shots not yet produced by an example are listed with example=None: the script
# reports them as pending and skips them, so it stays useful as coverage grows.
FIGURES = [
    # name                         example
    ("feature-gravity.png",        "13_screenshot_reel"),
    ("wiki-gravity.png",           "13_screenshot_reel"),
    ("wiki-sections.png",          "13_screenshot_reel"),
    ("tutorial-menu.png",          "14_screenshot_ui"),
    ("feature-editor.png",         "14_screenshot_ui"),
    ("feature-combat.png",         "15_screenshot_combat"),
    ("tutorial-combat-lock.png",   "15_screenshot_combat"),
    ("feature-autopilot.png",      "15_screenshot_combat"),
    ("tutorial-radar-lock.png",    "15_screenshot_combat"),
    ("devlog5-target-viewfinder.png", "15_screenshot_combat"),
    ("feature-hud.png",            "15_screenshot_combat"),
    # wiki-radar/-combat/-hud/-flight are filled by ALIASES (reuse), below.
    ("feature-juice.png",          "17_screenshot_juice"),
    ("tutorial-orbit.png",         "18_screenshot_orbit"),
    ("wiki-section-hull.png",      "16_screenshot_sections"),
    ("wiki-section-controller.png", "16_screenshot_sections"),
    ("wiki-section-thruster.png",  "16_screenshot_sections"),
    ("wiki-section-turret.png",    "16_screenshot_sections"),
    ("wiki-section-torpedo-bay.png", "16_screenshot_sections"),
    ("devlog5-radar-stance-slots.png", None),
]

# Thumbnails are 16:9 too (the post cards size them at 300px wide).
THUMBNAILS = [
    ("thumb-devlog-3.png", None),
    ("thumb-devlog-4.png", None),
    ("thumb-devlog-5.png", None),
]

# Wiki mechanic pages that reuse a captured shot of the same subject (no separate
# capture): {web name -> source web name}. The source must be copied first. Swap
# any of these for a distinct capture later by dropping the file in the stage dir
# (it takes precedence - see process_group).
ALIASES = {
    "wiki-radar.png": "tutorial-radar-lock.png",
    "wiki-combat.png": "feature-combat.png",
    "wiki-hud.png": "feature-hud.png",
    "wiki-flight.png": "feature-autopilot.png",
}

# 44x44 authored section icons: (web name, section letter, RGB accent). Colours
# echo the editor's component cards.
ICONS = [
    ("icon-hull.png",        "hull",        (0x5B, 0x8F, 0xB9)),
    ("icon-controller.png",  "controller",  (0x4F, 0xB7, 0xB3)),
    ("icon-thruster.png",    "thruster",    (0xD1, 0x8A, 0x3E)),
    ("icon-turret.png",      "turret",      (0xC8, 0x55, 0x55)),
    ("icon-torpedo-bay.png", "torpedo-bay", (0x8E, 0x6F, 0xC0)),
]

ICON_SIZE = 44
FIGURE_ASPECT = 16.0 / 9.0
ASPECT_TOLERANCE = 0.02


def png_dimensions(path):
    """Return (width, height) of a PNG by reading its IHDR, stdlib only."""
    with open(path, "rb") as handle:
        header = handle.read(24)
    if header[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a PNG")
    # A truncated file that still starts with the signature would make the
    # unpack below raise struct.error; surface it as a ValueError so the caller
    # can report it as a normal validation failure alongside the mis-sized ones.
    if len(header) < 24 or header[12:16] != b"IHDR":
        raise ValueError(f"{path} is a truncated or malformed PNG (no IHDR)")
    width, height = struct.unpack(">II", header[16:24])
    return width, height


def write_png(path, width, height, pixels):
    """Write an 8-bit RGBA PNG from a flat bytes buffer (row-major RGBA)."""

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    # Prepend the per-row filter byte (0 = none).
    stride = width * 4
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw.extend(pixels[y * stride:(y + 1) * stride])

    signature = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    with open(path, "wb") as handle:
        handle.write(signature)
        handle.write(chunk(b"IHDR", ihdr))
        handle.write(chunk(b"IDAT", idat))
        handle.write(chunk(b"IEND", b""))


class Canvas:
    """A tiny RGBA raster with the few fill primitives the icons need."""

    def __init__(self, size, background=(0, 0, 0, 0)):
        self.size = size
        self.px = bytearray(background * (size * size))

    def _set(self, x, y, color):
        if 0 <= x < self.size and 0 <= y < self.size:
            i = (y * self.size + x) * 4
            r, g, b = color
            self.px[i:i + 4] = bytes((r, g, b, 255))

    def rect(self, x0, y0, x1, y1, color):
        for y in range(y0, y1):
            for x in range(x0, x1):
                self._set(x, y, color)

    def rounded_rect(self, x0, y0, x1, y1, radius, color):
        for y in range(y0, y1):
            for x in range(x0, x1):
                cx = min(max(x, x0 + radius), x1 - 1 - radius)
                cy = min(max(y, y0 + radius), y1 - 1 - radius)
                if (x - cx) ** 2 + (y - cy) ** 2 <= radius * radius:
                    self._set(x, y, color)

    def circle(self, cx, cy, radius, color):
        for y in range(cy - radius, cy + radius + 1):
            for x in range(cx - radius, cx + radius + 1):
                if (x - cx) ** 2 + (y - cy) ** 2 <= radius * radius:
                    self._set(x, y, color)

    def triangle_down(self, cx, top_y, half_w, height, color):
        for row in range(height):
            span = int(half_w * (1 - row / height))
            y = top_y + row
            for x in range(cx - span, cx + span + 1):
                self._set(x, y, color)

    def bytes(self):
        return bytes(self.px)


def draw_icon(section, accent):
    """A flat 44x44 diagram glyph per section: a rounded accent tile with a
    simple white mark that reads at icon size."""
    c = Canvas(ICON_SIZE)
    white = (0xF2, 0xF4, 0xF7)
    dark = (0x10, 0x14, 0x1C)
    c.rounded_rect(2, 2, 42, 42, 8, accent)

    if section == "hull":
        # A layered plate.
        c.rounded_rect(10, 12, 34, 32, 5, white)
        c.rounded_rect(14, 16, 30, 28, 3, accent)
    elif section == "controller":
        # A chip: square body with pins.
        c.rect(15, 15, 29, 29, white)
        c.rect(19, 19, 25, 25, dark)
        for x in (12, 30):
            c.rect(x, 18, x + 2, 26, white)
        for y in (12, 30):
            c.rect(18, y, 26, y + 2, white)
    elif section == "thruster":
        # A nozzle with a flame.
        c.rect(17, 10, 27, 24, white)
        c.triangle_down(22, 24, 8, 12, white)
    elif section == "turret":
        # A base with a raised barrel.
        c.circle(22, 30, 8, white)
        c.rect(19, 10, 25, 30, white)
    elif section == "torpedo-bay":
        # A tube with a torpedo.
        c.rounded_rect(9, 18, 35, 26, 4, white)
        c.circle(30, 22, 3, accent)
    return c.bytes()


def process_group(entries, kind, stage_dir, expect_aspect):
    """Validate + copy each present staged file; report pending/failed."""
    copied, pending, failed = [], [], []
    for name, example in entries:
        src = os.path.join(stage_dir, name)
        if not os.path.exists(src):
            pending.append((name, example))
            continue
        try:
            width, height = png_dimensions(src)
        except ValueError as error:
            failed.append((name, str(error)))
            continue
        if expect_aspect is not None:
            aspect = width / height
            if abs(aspect - FIGURE_ASPECT) > ASPECT_TOLERANCE:
                failed.append((name, f"{width}x{height} is not 16:9"))
                continue
        shutil.copyfile(src, os.path.join(WEB_ASSETS, name))
        copied.append((name, width, height))

    print(f"\n{kind}:")
    for name, w, h in copied:
        print(f"  copied  {name}  ({w}x{h})")
    for name, example in pending:
        hint = f" (run {example})" if example else " (no capture example yet)"
        print(f"  pending {name}{hint}")
    return copied, pending, failed


def main():
    parser = argparse.ArgumentParser(description="Package captured screenshots into web/src/assets.")
    parser.add_argument("--stage-dir", default=DEFAULT_STAGE,
                        help=f"where the capture examples wrote the PNGs (default {DEFAULT_STAGE})")
    parser.add_argument("--no-icons", action="store_true", help="skip generating the section icons")
    args = parser.parse_args()

    if not os.path.isdir(WEB_ASSETS):
        sys.exit(f"web assets dir not found: {WEB_ASSETS}")
    os.makedirs(args.stage_dir, exist_ok=True)

    all_failed = []
    fig = process_group(FIGURES, "Figures", args.stage_dir, FIGURE_ASPECT)
    all_failed += fig[2]
    thumb = process_group(THUMBNAILS, "Thumbnails", args.stage_dir, FIGURE_ASPECT)
    all_failed += thumb[2]

    # Wiki mechanic pages reuse a captured shot of the same subject, unless a
    # distinct capture for them was staged (then that already won, above).
    print("\nAliases (wiki pages reusing a captured shot):")
    aliased = 0
    for alias, source in ALIASES.items():
        if os.path.exists(os.path.join(args.stage_dir, alias)):
            continue  # a distinct capture exists; process_group handled it
        source_path = os.path.join(WEB_ASSETS, source)
        if os.path.exists(source_path):
            shutil.copyfile(source_path, os.path.join(WEB_ASSETS, alias))
            print(f"  alias   {alias}  <- {source}")
            aliased += 1
        else:
            print(f"  pending {alias} (source {source} not available)")

    if not args.no_icons:
        print("\nIcons (generated 44x44):")
        for name, section, accent in ICONS:
            pixels = draw_icon(section, accent)
            write_png(os.path.join(WEB_ASSETS, name), ICON_SIZE, ICON_SIZE, pixels)
            print(f"  wrote   {name}")

    copied_count = len(fig[0]) + len(thumb[0]) + aliased
    pending_count = len(fig[1]) + len(thumb[1])
    print(f"\nDone: {copied_count} screenshot(s) copied, "
          f"{0 if args.no_icons else len(ICONS)} icon(s) generated, "
          f"{pending_count} screenshot(s) still pending.")

    if all_failed:
        print("\nFAILED (present but wrong shape):", file=sys.stderr)
        for name, why in all_failed:
            print(f"  {name}: {why}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
