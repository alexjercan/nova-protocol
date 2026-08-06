#!/usr/bin/env python3
"""Generate a distinct placeholder thumbnail for every picker-visible scenario.

The Scenarios picker shows a thumbnail next to the selected scenario's details.
Real per-scenario art is OWNER work (a drawn or composed look, not a gameplay
still - owner call 2026-08-04), so until it exists this writes a GOOD
PLACEHOLDER per scenario the same way `scripts/gen-placeholder-sounds.py` fills
the audio gap: one deterministic 320x180 PNG per scenario, rendered from its
own title in the NOVA OS phosphor look (glitched title text, chromatic offset,
scanlines, dark field). Every scenario therefore looks DIFFERENT and the picker
stops looking broken.

Overwrite any generated file with real art at the same path and no code change
is needed - the same contract the placeholder sounds have. The advisory
coverage report (`scripts/gen-web-screenshots.py --report`) tells the two apart
by re-rendering: a file that still matches this generator's output is a
placeholder awaiting real art; anything else is authored art.

Run from anywhere (paths are resolved from this file):

    python3 scripts/gen-scenario-thumbnails.py            # write every PNG
    python3 scripts/gen-scenario-thumbnails.py --check    # verify, write nothing

`--check` re-renders every thumbnail in memory and compares it byte for byte
with the committed file, so a stale commit or a non-deterministic edit to this
script fails instead of silently drifting. It exits non-zero on any mismatch.

Stdlib only (no Pillow), like its sibling generators: the PNG encoder is
`encode_png` imported from `scripts/gen-web-screenshots.py` rather than a second
copy, and the type is drawn with the 5x7 bitmap font below.
"""

import argparse
import hashlib
import importlib.util
import os
import random
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The PNG encoder lives in the screenshot packager; import it rather than
# writing a second encoder. The module name has dashes, so it cannot be a plain
# `import` - load it by path. It runs argparse only under `main()`.
_SPEC = importlib.util.spec_from_file_location(
    "gen_web_screenshots", os.path.join(REPO_ROOT, "scripts", "gen-web-screenshots.py"))
_SCREENSHOTS = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_SCREENSHOTS)
encode_png = _SCREENSHOTS.encode_png

WIDTH = 320
HEIGHT = 180

# Every picker-visible scenario: the flat `!hidden` set plus the `hidden`
# campaign members the picker lists indented under their campaign header (base:
# broadside_gunship + final_tally under "Nova Protocol"; The Ledger: chapters 2
# to 5 under "The Ledger"). Each entry is
# (scenario id, picker title, output path relative to the repo root).
#
# The path is inside the OWNING mod's asset tree, because a thumbnail is that
# mod's own art: base writes under `assets/base/` and is referenced as
# `self://thumbnails/<id>.png`; a portal mod writes under its own
# `webmods/<mod>/` folder and is referenced the same way (and listed in that
# bundle's `resources`). No scenario points at another mod's art any more.
SCENARIOS = [
    # --- base (the built-in scenarios; builders in nova_assets) ---
    ("asteroid_field", "Asteroid Field", "assets/base/thumbnails/asteroid_field.png"),
    ("shakedown_run", "Shakedown Run", "assets/base/thumbnails/shakedown_run.png"),
    ("broadside", "Broadside", "assets/base/thumbnails/broadside.png"),
    ("broadside_gunship", "Broadside: Rust Tally",
     "assets/base/thumbnails/broadside_gunship.png"),
    ("lifeline", "Lifeline", "assets/base/thumbnails/lifeline.png"),
    ("final_tally", "Final Tally", "assets/base/thumbnails/final_tally.png"),
    # --- webmods/gauntlet ---
    ("gauntlet_run", "Gauntlet Run", "webmods/gauntlet/thumbnails/gauntlet_run.png"),
    # --- webmods/the-ledger (chapter 1 visible, 2-5 under the campaign header) ---
    ("ledger_ch1_dead_weight", "Dead Weight",
     "webmods/the-ledger/thumbnails/ledger_ch1_dead_weight.png"),
    ("ledger_ch2_claim_jumpers", "Claim Jumpers",
     "webmods/the-ledger/thumbnails/ledger_ch2_claim_jumpers.png"),
    ("ledger_ch2b_the_heavies", "The Heavies",
     "webmods/the-ledger/thumbnails/ledger_ch2b_the_heavies.png"),
    ("ledger_ch3_quiet_channel", "The Quiet Channel",
     "webmods/the-ledger/thumbnails/ledger_ch3_quiet_channel.png"),
    ("ledger_ch4_the_buyer", "The Buyer",
     "webmods/the-ledger/thumbnails/ledger_ch4_the_buyer.png"),
    ("ledger_ch5_the_raid", "The Raid",
     "webmods/the-ledger/thumbnails/ledger_ch5_the_raid.png"),
]

# The NOVA OS screen: the dark phosphor field these thumbnails sit on
# (`nova_ui::theme::SCREEN_0`/`SCREEN_1`), top to bottom.
FIELD_TOP = (0x00, 0x13, 0x04)
FIELD_BOTTOM = (0x00, 0x2b, 0x0f)

# Title inks, from `nova_ui::theme`. One is picked per scenario by a stable hash
# of its id, so the set reads as one system while neighbouring rows in the
# picker are unlikely to share an ink.
INKS = [
    (0x36, 0xFF, 0x79),  # PHOSPHOR
    (0x7D, 0xFF, 0xAB),  # PHOSPHOR_HI
    (0xFF, 0xB8, 0x4A),  # AMBER_NOVA
    (0x36, 0xA3, 0xFF),  # BLUE
    (0xFF, 0x7B, 0x2D),  # ORANGE
    (0xB9, 0xFF, 0xC9),  # SCREEN_TEXT
]

# A 5x7 uppercase bitmap font - the whole typeface these titles need. An
# unsupported character is a hard error, not a blank: a new scenario title with
# a glyph this font cannot draw must be noticed here, not in the picker.
GLYPH_W = 5
GLYPH_H = 7

FONT = {
    "A": (".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"),
    "B": ("####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."),
    "C": (".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."),
    "D": ("####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."),
    "E": ("#####", "#....", "#....", "####.", "#....", "#....", "#####"),
    "F": ("#####", "#....", "#....", "####.", "#....", "#....", "#...."),
    "G": (".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###."),
    "H": ("#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"),
    "I": (".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."),
    "J": ("..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."),
    "K": ("#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"),
    "L": ("#....", "#....", "#....", "#....", "#....", "#....", "#####"),
    "M": ("#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"),
    "N": ("#...#", "##..#", "#.#.#", "#.#.#", "#..##", "#...#", "#...#"),
    "O": (".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."),
    "P": ("####.", "#...#", "#...#", "####.", "#....", "#....", "#...."),
    "Q": (".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"),
    "R": ("####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"),
    "S": (".####", "#....", "#....", ".###.", "....#", "....#", "####."),
    "T": ("#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."),
    "U": ("#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."),
    "V": ("#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."),
    "W": ("#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#"),
    "X": ("#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"),
    "Y": ("#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."),
    "Z": ("#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"),
    "0": (".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."),
    "1": ("..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."),
    "2": (".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"),
    "3": ("#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."),
    "4": ("...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."),
    "5": ("#####", "#....", "####.", "....#", "....#", "#...#", ".###."),
    "6": ("..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."),
    "7": ("#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."),
    "8": (".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."),
    "9": (".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."),
    " ": (".....", ".....", ".....", ".....", ".....", ".....", "....."),
    ":": (".....", "..#..", "..#..", ".....", "..#..", "..#..", "....."),
    "-": (".....", ".....", ".....", ".###.", ".....", ".....", "....."),
    ".": (".....", ".....", ".....", ".....", ".....", "..#..", "..#.."),
    "'": ("..#..", "..#..", ".....", ".....", ".....", ".....", "....."),
    ",": (".....", ".....", ".....", ".....", "..#..", "..#..", ".#..."),
}

# A malformed glyph would silently draw a clipped letter; catch it at import.
for _char, _rows in FONT.items():
    assert len(_rows) == GLYPH_H and all(len(row) == GLYPH_W for row in _rows), _char




class Frame:
    """A 320x180 RGBA raster with the few primitives these thumbnails need."""

    def __init__(self, width, height):
        self.w = width
        self.h = height
        self.px = bytearray(b"\x00\x00\x00\xff" * (width * height))

    def set(self, x, y, color):
        if 0 <= x < self.w and 0 <= y < self.h:
            i = (y * self.w + x) * 4
            self.px[i] = color[0]
            self.px[i + 1] = color[1]
            self.px[i + 2] = color[2]

    def add(self, x, y, color, amount):
        """Additive light: brighten a pixel toward `color` by `amount` (0..1).

        Additive rather than alpha-blended because everything drawn here is
        emitted light on a dark screen - overlapping glow should build up."""
        if not (0 <= x < self.w and 0 <= y < self.h):
            return
        i = (y * self.w + x) * 4
        for c in range(3):
            self.px[i + c] = min(255, self.px[i + c] + int(color[c] * amount))

    def scale_row(self, y, factor):
        if not 0 <= y < self.h:
            return
        base = y * self.w * 4
        for x in range(self.w):
            i = base + x * 4
            for c in range(3):
                self.px[i + c] = int(self.px[i + c] * factor)

    def shift_row(self, y, dx):
        """Slide one scanline horizontally, leaving the vacated edge black - the
        torn-scanline half of the glitch look."""
        if not 0 <= y < self.h or dx == 0:
            return
        base = y * self.w * 4
        row = bytes(self.px[base:base + self.w * 4])
        blank = b"\x00\x00\x00\xff"
        if dx > 0:
            shifted = blank * dx + row[:(self.w - dx) * 4]
        else:
            shifted = row[-dx * 4:] + blank * -dx
        self.px[base:base + self.w * 4] = shifted

    def bytes(self):
        return bytes(self.px)


def text_width(text, scale):
    """Rendered width of `text` at `scale`, without the trailing letter gap."""
    if not text:
        return 0
    return len(text) * (GLYPH_W + 1) * scale - scale


def draw_glyph(frame, char, x, y, scale, ink, amount=1.0):
    """Stamp one glyph with its top-left at (x, y), `scale` pixels per font dot."""
    rows = FONT.get(char)
    if rows is None:
        raise ValueError(f"no glyph for {char!r} - add it to FONT")
    for row_index, row in enumerate(rows):
        for col, bit in enumerate(row):
            if bit != "#":
                continue
            for dy in range(scale):
                for dx in range(scale):
                    frame.add(x + col * scale + dx, y + row_index * scale + dy, ink, amount)


def glyph_mask(text, scale):
    """The set of (x, y) dots `text` covers, relative to its own top-left."""
    dots = set()
    pen = 0
    for char in text:
        rows = FONT.get(char)
        if rows is None:
            raise ValueError(f"no glyph for {char!r} - add it to FONT")
        for row_index, row in enumerate(rows):
            for col, bit in enumerate(row):
                if bit != "#":
                    continue
                for dy in range(scale):
                    for dx in range(scale):
                        dots.add((pen + col * scale + dx, row_index * scale + dy))
        pen += (GLYPH_W + 1) * scale
    return dots


def wrap_title(title, scale, max_width):
    """Greedy word wrap of an already-uppercased title at `scale`.

    Returns None when a single word cannot fit, so the caller can try a smaller
    scale rather than emit a clipped line."""
    lines = []
    current = ""
    for word in title.split():
        if text_width(word, scale) > max_width:
            return None
        candidate = word if not current else f"{current} {word}"
        if text_width(candidate, scale) <= max_width:
            current = candidate
        else:
            lines.append(current)
            current = word
    if current:
        lines.append(current)
    return lines


def layout_title(title, max_width, max_lines):
    """(scale, lines) for the largest scale whose wrap fits `max_lines`."""
    for scale in (5, 4, 3, 2):
        lines = wrap_title(title, scale, max_width)
        if lines is not None and len(lines) <= max_lines:
            return scale, lines
    raise ValueError(f"title {title!r} does not fit even at the smallest scale")


def render(scenario_id, title):
    """The RGBA pixels of one scenario's placeholder thumbnail.

    Deterministic: every random choice comes from a generator seeded with the
    scenario id, so the same id always renders the same bytes."""
    rng = random.Random(hashlib.sha256(scenario_id.encode("utf-8")).hexdigest())
    ink = INKS[int(hashlib.sha256(scenario_id.encode("utf-8")).hexdigest(), 16) % len(INKS)]
    frame = Frame(WIDTH, HEIGHT)

    # 1. The screen: a vertical SCREEN_0 -> SCREEN_1 gradient.
    for y in range(HEIGHT):
        t = y / (HEIGHT - 1)
        color = tuple(int(FIELD_TOP[c] + (FIELD_BOTTOM[c] - FIELD_TOP[c]) * t) for c in range(3))
        for x in range(WIDTH):
            frame.set(x, y, color)

    # 2. A sparse starfield, so the field reads as space and not as a swatch.
    for _ in range(70):
        frame.add(rng.randrange(WIDTH), rng.randrange(HEIGHT), (255, 255, 255),
                  rng.uniform(0.05, 0.22))

    # 3. The title: centred, wrapped, with a soft glow and a chromatic offset
    #    (red pulled left, blue pushed right) for the CRT-misconvergence look.
    upper = title.upper()
    scale, lines = layout_title(upper, WIDTH - 48, 3)
    line_height = (GLYPH_H + 2) * scale
    block_h = line_height * len(lines) - 2 * scale
    top = (HEIGHT - block_h) // 2
    for index, line in enumerate(lines):
        x = (WIDTH - text_width(line, scale)) // 2
        y = top + index * line_height
        dots = glyph_mask(line, scale)
        for dx, dy in dots:
            # Glow first, so the solid stroke lands on top of its own halo.
            for ox, oy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                frame.add(x + dx + ox, y + dy + oy, ink, 0.18)
        for dx, dy in dots:
            frame.add(x + dx - 2, y + dy, (ink[0], 0, 0), 0.55)
            frame.add(x + dx + 2, y + dy, (0, 0, ink[2]), 0.55)
        for dx, dy in dots:
            frame.set(x + dx, y + dy, ink)

    # 4. The scenario id, small and dim along the bottom - a label on the plate.
    label = scenario_id.upper().replace("_", " ")
    draw_glyph_line(frame, label, 12, HEIGHT - 18, 1, ink, 0.45)

    # 5. Torn scanlines: a few short bands slid sideways, biased to the title
    #    band so the glitch reads as the type breaking up.
    for _ in range(4):
        band_top = rng.randrange(top - 6, top + block_h + 6)
        for y in range(band_top, band_top + rng.randrange(1, 4)):
            frame.shift_row(y, rng.choice((-3, -2, 2, 3)))

    # 6. CRT scanlines over everything, then a dim frame around the plate.
    for y in range(0, HEIGHT, 2):
        frame.scale_row(y, 0.62)
    for x in range(WIDTH):
        frame.add(x, 0, ink, 0.30)
        frame.add(x, HEIGHT - 1, ink, 0.30)
    for y in range(HEIGHT):
        frame.add(0, y, ink, 0.30)
        frame.add(WIDTH - 1, y, ink, 0.30)

    return frame.bytes()


def draw_glyph_line(frame, text, x, y, scale, ink, amount):
    """Draw a whole line of glyphs at one intensity (the dim id label)."""
    pen = x
    for char in text:
        draw_glyph(frame, char, pen, y, scale, ink, amount)
        pen += (GLYPH_W + 1) * scale


def encoded(scenario_id, title):
    """The exact PNG file bytes for one scenario, without touching the disk."""
    return encode_png(WIDTH, HEIGHT, render(scenario_id, title))


def is_generated_placeholder(scenario_id, title, path):
    """True when the file at `path` is still exactly this generator's output.

    How the advisory coverage report tells a placeholder from real art without
    any marker file: real art overwrites the same path and stops matching."""
    try:
        with open(path, "rb") as handle:
            return handle.read() == encoded(scenario_id, title)
    except OSError:
        return False


def check():
    """Verify every committed thumbnail matches a fresh render. Writes nothing."""
    stale, missing = [], []
    for scenario_id, title, rel in SCENARIOS:
        path = os.path.join(REPO_ROOT, rel)
        if not os.path.exists(path):
            missing.append(rel)
            continue
        with open(path, "rb") as handle:
            if handle.read() != encoded(scenario_id, title):
                stale.append(rel)
    for rel in missing:
        print(f"  MISSING  {rel}")
    for rel in stale:
        print(f"  STALE    {rel} (differs from a fresh render)")
    if missing or stale:
        print(f"\n{len(missing) + len(stale)} of {len(SCENARIOS)} thumbnail(s) out of date - "
              "run scripts/gen-scenario-thumbnails.py", file=sys.stderr)
        return 1
    print(f"{len(SCENARIOS)} scenario thumbnail(s) match a fresh render (byte for byte).")
    return 0


def generate():
    for scenario_id, title, rel in SCENARIOS:
        path = os.path.join(REPO_ROOT, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as handle:
            handle.write(encoded(scenario_id, title))
        print(f"  wrote  {rel}  ({title})")
    print(f"\n{len(SCENARIOS)} scenario thumbnail(s) written at {WIDTH}x{HEIGHT}.")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Generate the placeholder scenario thumbnails the picker shows.")
    parser.add_argument("--check", action="store_true",
                        help="verify the committed PNGs match a fresh render, write nothing")
    args = parser.parse_args()
    sys.exit(check() if args.check else generate())


if __name__ == "__main__":
    main()
