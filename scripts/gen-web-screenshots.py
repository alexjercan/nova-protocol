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
  2. It BUILDS the composite shots a single capture cannot make (the devlog-5
     stance comparison) by decoding two already-copied figures with a tiny
     stdlib PNG codec, contain-scaling each into half the frame, and writing the
     side-by-side result. A distinct capture staged for the composite name wins.
  3. It GENERATES the five 44x44 section icons directly (simple flat diagram
     glyphs, matching the editor's per-section colours) - these are authored, not
     captured.

`--self-test` round-trips synthetic images through the PNG codec (all five row
filters, RGB + RGBA) and exits, so the decode/resize/compose path is checkable
without any GPU-captured asset.

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
dimensions are read from the IHDR header, and a tiny built-in PNG codec
(zlib-backed) both decodes the figures the composites need and encodes the icons
and composites.
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
    # devlog5-radar-stance-slots is built by COMPOSITES (below) from two shots.
]

# Thumbnails are 16:9 too (the post cards size them at 300px wide).
THUMBNAILS = [
    ("thumb-devlog-3.png", None),
    ("thumb-devlog-4.png", None),
    ("thumb-devlog-5.png", None),
]

# Composites built in-script from two already-copied figures placed side by side
# (each scaled to half width) into one 16:9 frame - the shots a single capture
# cannot make. (web name, left source web name, right source web name). Like an
# alias, a distinct capture staged for the composite name wins over the build.
COMPOSITES = [
    # The devlog-5 stance comparison: white NAV lock (left) vs red combat lock
    # (right), matching the post's caption "white NAV lock lowered, red combat
    # lock raised".
    ("devlog5-radar-stance-slots.png", "tutorial-radar-lock.png", "feature-combat.png"),
]

# Composite output frame (16:9, the figure resolution the capture reel uses).
COMPOSITE_SIZE = (1920, 1080)

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


def decode_png(path):
    """Decode a non-interlaced 8-bit PNG to (width, height, channels, pixels).

    Stdlib only: walk the chunks for IHDR + IDAT, zlib-inflate the image data,
    and reverse the per-scanline filters. Handles colour type 2 (RGB, 3ch) and 6
    (RGBA, 4ch) at bit depth 8 - the shapes Bevy's `save_to_disk` writes and the
    icon encoder above produces. Anything else raises ValueError (surfaced as a
    normal validation failure, not a crash)."""
    with open(path, "rb") as handle:
        data = handle.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a PNG")

    width = height = channels = None
    idat = bytearray()
    pos = 8
    while pos + 8 <= len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        tag = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length  # length(4) + tag(4) + body + crc(4)
        if tag == b"IHDR":
            if len(body) < 13:
                raise ValueError(f"{path}: truncated IHDR chunk")
            w, h, bit_depth, color_type, _comp, _filt, interlace = struct.unpack(">IIBBBBB", body[:13])
            if bit_depth != 8 or color_type not in (2, 6) or interlace != 0:
                raise ValueError(
                    f"{path}: unsupported PNG (bit_depth={bit_depth}, "
                    f"color_type={color_type}, interlace={interlace}); "
                    "expected 8-bit RGB/RGBA non-interlaced")
            width, height, channels = w, h, (3 if color_type == 2 else 4)
        elif tag == b"IDAT":
            idat += body
        elif tag == b"IEND":
            break

    if width is None:
        raise ValueError(f"{path}: no IHDR chunk")

    try:
        raw = zlib.decompress(bytes(idat))
    except zlib.error as error:
        # Keep the "report, don't crash" contract of png_dimensions: a corrupt
        # IDAT stream is a validation failure, not an uncaught crash.
        raise ValueError(f"{path}: corrupt PNG image data ({error})")
    stride = width * channels
    if len(raw) != (stride + 1) * height:
        raise ValueError(f"{path}: inflated size {len(raw)} != expected {(stride + 1) * height}")

    # Reverse the per-scanline filters (PNG spec 9.2). Each output byte is
    # reconstructed from the raw byte plus predictors: a = byte `channels` to the
    # left (0 at the row start), b = byte above, c = byte above-left.
    out = bytearray(stride * height)
    prev = bytearray(stride)
    src = 0
    for y in range(height):
        ftype = raw[src]
        src += 1
        line = raw[src:src + stride]
        src += stride
        row = out[y * stride:(y + 1) * stride]
        if ftype == 0:
            row[:] = line
        elif ftype == 1:  # Sub
            for i in range(stride):
                a = row[i - channels] if i >= channels else 0
                row[i] = (line[i] + a) & 0xFF
        elif ftype == 2:  # Up
            for i in range(stride):
                row[i] = (line[i] + prev[i]) & 0xFF
        elif ftype == 3:  # Average
            for i in range(stride):
                a = row[i - channels] if i >= channels else 0
                row[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif ftype == 4:  # Paeth
            for i in range(stride):
                a = row[i - channels] if i >= channels else 0
                b = prev[i]
                c = prev[i - channels] if i >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                row[i] = (line[i] + pr) & 0xFF
        else:
            raise ValueError(f"{path}: unknown filter type {ftype} on row {y}")
        out[y * stride:(y + 1) * stride] = row
        prev = row
    return width, height, channels, out


def resize_box(pixels, sw, sh, channels, dw, dh):
    """Area-average (box) resample to (dw, dh). Each destination pixel averages
    the source pixels its cell covers - clean for the integer downscales here
    (1920->960 is exactly 2 source columns per destination column)."""
    out = bytearray(dw * dh * channels)
    # Source column span for each destination column, precomputed once.
    x_spans = [(dx * sw // dw, max((dx + 1) * sw // dw, dx * sw // dw + 1)) for dx in range(dw)]
    for dy in range(dh):
        y0 = dy * sh // dh
        y1 = max((dy + 1) * sh // dh, y0 + 1)
        for dx in range(dw):
            x0, x1 = x_spans[dx]
            acc = [0] * channels
            count = 0
            for sy in range(y0, y1):
                base = sy * sw * channels
                for sx in range(x0, x1):
                    off = base + sx * channels
                    for c in range(channels):
                        acc[c] += pixels[off + c]
                    count += 1
            dst = (dy * dw + dx) * channels
            for c in range(channels):
                out[dst + c] = acc[c] // count
    return out


def compose_side_by_side(left_path, right_path, out_w, out_h):
    """Build an opaque-black RGBA `out_w`x`out_h` buffer with `left_path` in the
    left half and `right_path` in the right half. Each source is scaled to
    *contain* its half (aspect preserved) and centered, so nothing is distorted
    or cropped; the leftover margin is black - which blends into these
    space-black frames rather than reading as a letterbox. Sources may be RGB or
    RGBA; the result is opaque RGBA for `write_png`."""
    half_w = out_w // 2
    out = bytearray(b"\x00\x00\x00\xff" * (out_w * out_h))  # opaque black canvas
    for tile_index, path in enumerate((left_path, right_path)):
        sw, sh, channels, pixels = decode_png(path)
        scale = min(half_w / sw, out_h / sh)
        draw_w = max(1, round(sw * scale))
        draw_h = max(1, round(sh * scale))
        tile = resize_box(pixels, sw, sh, channels, draw_w, draw_h)
        x_offset = tile_index * half_w + (half_w - draw_w) // 2
        y_offset = (out_h - draw_h) // 2
        for y in range(draw_h):
            src_row = y * draw_w * channels
            dst_row = ((y_offset + y) * out_w + x_offset) * 4
            for x in range(draw_w):
                s = src_row + x * channels
                d = dst_row + x * 4
                out[d] = tile[s]
                out[d + 1] = tile[s + 1]
                out[d + 2] = tile[s + 2]
                out[d + 3] = 255
    return out


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


def build_composites(stage_dir):
    """Generate each COMPOSITES entry side by side from two already-copied
    figures, unless a distinct capture for the composite name was staged (then it
    wins, like an alias). Returns (built, pending, failed)."""
    print("\nComposites (built from two figures):")
    built, pending, failed = [], [], []
    out_w, out_h = COMPOSITE_SIZE
    for name, left, right in COMPOSITES:
        staged = os.path.join(stage_dir, name)
        if os.path.exists(staged):
            # A distinct capture was staged; validate + copy it, it wins.
            try:
                width, height = png_dimensions(staged)
            except ValueError as error:
                failed.append((name, str(error)))
                continue
            if abs(width / height - FIGURE_ASPECT) > ASPECT_TOLERANCE:
                failed.append((name, f"{width}x{height} is not 16:9"))
                continue
            shutil.copyfile(staged, os.path.join(WEB_ASSETS, name))
            print(f"  captured {name}  ({width}x{height}, staged capture wins)")
            built.append(name)
            continue

        sources = [(left, os.path.join(WEB_ASSETS, left)), (right, os.path.join(WEB_ASSETS, right))]
        missing = [src for src, path in sources if not os.path.exists(path)]
        if missing:
            print(f"  pending {name} (sources not available: {', '.join(missing)})")
            pending.append((name, None))
            continue
        try:
            pixels = compose_side_by_side(sources[0][1], sources[1][1], out_w, out_h)
        except ValueError as error:
            failed.append((name, str(error)))
            continue
        write_png(os.path.join(WEB_ASSETS, name), out_w, out_h, pixels)
        print(f"  built   {name}  ({out_w}x{out_h}) <- {left} | {right}")
        built.append(name)
    return built, pending, failed


def self_test():
    """Round-trip synthetic images through decode/resize/compose so the codec is
    checkable without any GPU-captured asset. Exercises all five PNG row filters
    (the risky reconstruction logic) for both RGB and RGBA."""
    import tempfile

    def make_pixels(w, h, ch):
        px = bytearray(w * h * ch)
        for y in range(h):
            for x in range(w):
                i = (y * w + x) * ch
                px[i] = (x * 7 + y * 3) & 0xFF
                px[i + 1] = (x * 13 + y * 5) & 0xFF
                px[i + 2] = (x * 3 + y * 11) & 0xFF
                if ch == 4:
                    px[i + 3] = (x + y) & 0xFF
        return bytes(px)

    def encode_filtered(w, h, ch, px, ftype):
        """Emit a PNG whose every row uses filter `ftype` (forward transform), so
        decode_png's reverse of that filter is what gets tested."""
        stride = w * ch
        raw = bytearray()
        prev = bytes(stride)
        for y in range(h):
            line = px[y * stride:(y + 1) * stride]
            filt = bytearray(stride)
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                b = prev[i]
                c = prev[i - ch] if i >= ch else 0
                if ftype == 0:
                    pred = 0
                elif ftype == 1:
                    pred = a
                elif ftype == 2:
                    pred = b
                elif ftype == 3:
                    pred = (a + b) >> 1
                else:
                    p = a + b - c
                    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                    pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                filt[i] = (line[i] - pred) & 0xFF
            raw.append(ftype)
            raw.extend(filt)
            prev = line
        color_type = 2 if ch == 3 else 6

        def chunk(tag, data):
            body = tag + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

        ihdr = struct.pack(">IIBBBBB", w, h, 8, color_type, 0, 0, 0)
        return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
                + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b""))

    tmp = tempfile.mkdtemp()
    for ch in (3, 4):
        px = make_pixels(9, 7, ch)
        for ftype in range(5):
            path = os.path.join(tmp, f"t_{ch}_{ftype}.png")
            with open(path, "wb") as handle:
                handle.write(encode_filtered(9, 7, ch, px, ftype))
            w, h, dch, dec = decode_png(path)
            assert (w, h, dch) == (9, 7, ch), f"decode shape wrong for filter {ftype} ch {ch}"
            assert bytes(dec) == px, f"decode mismatch for filter {ftype} ch {ch}"

    # Box resample: a solid 4x2 halves to a 2x2 of the same colour (averaging
    # equal values is a no-op), and a 2x1 of [0,0,0]/[200,200,200] averages to 100.
    solid = bytes([10, 20, 30] * (4 * 2))
    assert bytes(resize_box(solid, 4, 2, 3, 2, 2)) == bytes([10, 20, 30] * (2 * 2)), "resize solid"
    two = bytes([0, 0, 0, 200, 200, 200])
    assert bytes(resize_box(two, 2, 1, 3, 1, 1)) == bytes([100, 100, 100]), "resize average"

    # Compose (no letterbox): two 2x2 tiles into 4x2 fill their halves exactly
    # (each contains its 2x2 half at scale 1) -> left half red, right half blue.
    red = os.path.join(tmp, "red.png")
    blue = os.path.join(tmp, "blue.png")
    write_png(red, 2, 2, bytes([220, 0, 0, 255] * 4))
    write_png(blue, 2, 2, bytes([0, 0, 220, 255] * 4))
    comp = compose_side_by_side(red, blue, 4, 2)
    assert comp[0:4] == bytes([220, 0, 0, 255]) and comp[8:12] == bytes([0, 0, 220, 255]), "compose halves"

    # Compose (letterbox): the same 2x2 tiles into a 4x4 canvas contain to 2x2 and
    # centre vertically (y_offset=1), so row 0 is the black bar and row 1 is the
    # image. Proves the aspect-preserving pad path.
    comp2 = compose_side_by_side(red, blue, 4, 4)
    assert comp2[0:4] == bytes([0, 0, 0, 255]), "compose top row is black bar"
    row1_left = comp2[(1 * 4 + 0) * 4:(1 * 4 + 0) * 4 + 4]
    row1_right = comp2[(1 * 4 + 2) * 4:(1 * 4 + 2) * 4 + 4]
    assert row1_left == bytes([220, 0, 0, 255]) and row1_right == bytes([0, 0, 220, 255]), "compose letterbox row"

    shutil.rmtree(tmp, ignore_errors=True)
    print("self-test OK: decode filters 0-4 (RGB+RGBA), resize_box, compose_side_by_side")


def main():
    parser = argparse.ArgumentParser(description="Package captured screenshots into web/src/assets.")
    parser.add_argument("--stage-dir", default=DEFAULT_STAGE,
                        help=f"where the capture examples wrote the PNGs (default {DEFAULT_STAGE})")
    parser.add_argument("--no-icons", action="store_true", help="skip generating the section icons")
    parser.add_argument("--self-test", action="store_true",
                        help="round-trip synthetic images through the PNG codec and exit")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return

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

    # Composites are built from figures copied above (their sources must already
    # be in web/src/assets), so this runs after the figure pass.
    comp = build_composites(args.stage_dir)
    all_failed += comp[2]

    if not args.no_icons:
        print("\nIcons (generated 44x44):")
        for name, section, accent in ICONS:
            pixels = draw_icon(section, accent)
            write_png(os.path.join(WEB_ASSETS, name), ICON_SIZE, ICON_SIZE, pixels)
            print(f"  wrote   {name}")

    copied_count = len(fig[0]) + len(thumb[0]) + aliased + len(comp[0])
    pending_count = len(fig[1]) + len(thumb[1]) + len(comp[1])
    print(f"\nDone: {copied_count} screenshot(s) copied/built, "
          f"{0 if args.no_icons else len(ICONS)} icon(s) generated, "
          f"{pending_count} screenshot(s) still pending.")

    if all_failed:
        print("\nFAILED (present but wrong shape):", file=sys.stderr)
        for name, why in all_failed:
            print(f"  {name}: {why}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
