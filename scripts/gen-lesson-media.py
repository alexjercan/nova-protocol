#!/usr/bin/env python3
"""Generate a placeholder demonstration for every base training lesson.

The Lessons screen draws one demonstration per lesson: a still frame, or a
looping sprite sheet cut into cells by the grid the lesson authors. Real
recorded footage is OWNER work, so until it exists this writes a GOOD
PLACEHOLDER per lesson the same way `scripts/gen-scenario-thumbnails.py` fills
the picker's art gap - a deterministic PNG per lesson in the NOVA OS phosphor
look, rendered from the lesson's own title and id so every lesson looks
DIFFERENT and the screen stops looking broken.

A still is one 480x270 frame. A loop is a 4x5 sheet of twenty 240x135 cells -
the grid `crates/nova_authoring/src/base_content/lessons.rs` authors - carrying
two seconds of periodic motion, so the frame the screen cuts out genuinely
animates instead of flickering between two stills.

That is a QUARTER of the size captured footage ships at, and deliberately so:
this art is drawn a pixel at a time in Python, and the screen it imitates is a
low-resolution CRT. The grid is what the game cuts on, not the cell size, so a
placeholder and a capture sit at the same path on the same lesson.

Real art lands at the same path with no code change, the same contract the
scenario thumbnails have - and REAL ART WINS: a lesson named in
`scripts/capture-lesson-media.sh`'s `PRODUCERS` table is never drawn here, by
either mode, so running the two in either order is safe. The table is read
rather than inferred from the file, because inferring gets a changed grid
exactly backwards.

The grid must keep matching the authored `columns`/`rows`/`frames`, and
`--check` is what proves that for the lessons still on placeholders: it
re-renders every sheet in memory and compares pixels, so a stale commit
or a non-deterministic edit fails instead of silently drifting.

Run from anywhere (paths are resolved from this file):

    python3 scripts/gen-lesson-media.py            # write every demonstration
    python3 scripts/gen-lesson-media.py --check    # verify, write nothing

The raster, the 5x7 bitmap font and the drawing primitives are imported from
`scripts/gen-scenario-thumbnails.py` rather than copied, so the two sets of
placeholder art cannot drift into two different looks. The one thing this does
not do in Python is the file format: a lesson demonstration is WEBP (see
`lessons.rs`), so the finished raster goes through ffmpeg as LOSSLESS WebP -
which is also what lets `--check` compare pixels instead of file bytes, and so
survive a different libwebp.
"""

import argparse
import hashlib
import importlib.util
import math
import os
import random
import subprocess
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The raster, the font and the PNG encoder live in the thumbnail generator;
# import it by path (the module name has dashes). It runs argparse only under
# `main()`.
_SPEC = importlib.util.spec_from_file_location(
    "gen_scenario_thumbnails",
    os.path.join(REPO_ROOT, "scripts", "gen-scenario-thumbnails.py"))
_THUMBS = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_THUMBS)

Frame = _THUMBS.Frame
draw_glyph_line = _THUMBS.draw_glyph_line
glyph_mask = _THUMBS.glyph_mask
layout_title = _THUMBS.layout_title
text_width = _THUMBS.text_width
FIELD_TOP = _THUMBS.FIELD_TOP
FIELD_BOTTOM = _THUMBS.FIELD_BOTTOM
INKS = _THUMBS.INKS
GLYPH_H = _THUMBS.GLYPH_H

# A still is one frame this size; a loop cell is exactly half of it, so the two
# shapes share a composition and the screen's media frame never reflows.
STILL_W, STILL_H = 480, 270
CELL_W, CELL_H = 240, 135

# The sheet grid the base lessons author. Changing it here means changing
# `looping()` in `lessons.rs` in the same commit - the game cuts the cells by
# the AUTHORED grid, not by anything in the file. The motion below is written
# against `t = nth / FRAMES`, so a longer sheet is a slower sweep, not a
# truncated one.
COLUMNS, ROWS, FRAMES = 4, 5, 20

# Every base lesson: (lesson id, screen title, "still" or "loop"). The output
# path is `assets/base/training/<id>.webp` for all of them, listed in
# `assets/base/base.bundle.ron` and referenced as `self://training/<id>.webp`.
LESSONS = [
    ("start_welcome", "How training works", "still"),
    ("start_hud", "Reading the HUD", "still"),
    ("start_verbs", "The keybind dock", "still"),
    ("start_camera", "Looking around", "loop"),
    ("start_cinematic", "Two HUD levels", "still"),
    ("flight_aim", "Turn, then thrust", "loop"),
    ("flight_momentum", "You keep your speed", "loop"),
    ("flight_speedcap", "The speed cap", "loop"),
    ("flight_stop", "The STOP order", "loop"),
    ("flight_cancel", "Taking the ship back", "loop"),
    ("flight_rcs", "Using the RCS thrusters", "loop"),
    ("flight_gravity", "Gravity wells", "still"),
    ("flight_goto", "GOTO a mark", "loop"),
    ("flight_arrival", "The arrival envelope", "still"),
    ("flight_orbit", "ORBIT a mark", "loop"),
    ("combat_radar", "Using the radar", "loop"),
    ("combat_allegiance", "Who shoots whom", "still"),
    ("combat_stance", "Raising weapons", "loop"),
    ("combat_cover", "Cover and the firing line", "still"),
    ("combat_components", "Lock a component", "loop"),
    ("combat_damage_types", "Kinetic and Pierce", "still"),
    ("combat_turrets", "Turret arcs", "still"),
    ("combat_magazines", "A magazine is a rate limit", "still"),
    ("combat_torpedoes", "Torpedoes", "loop"),
    ("combat_point_defense", "Your battery defends itself", "still"),
    ("combat_railgun", "The hull is the aim", "still"),
    ("combat_collapse", "When a ship comes apart", "still"),
    ("build_sections", "Ships are built from sections", "still"),
    ("build_mass", "Mass and thrust", "still"),
    ("build_balance", "Thruster placement", "still"),
    ("build_turning", "What decides your turn rate", "still"),
    ("build_flight_test", "Fly what you built", "loop"),
    ("build_weapon_mounts", "Where a weapon can point", "still"),
    ("build_docking_port", "Bolting on a docking port", "still"),
    ("build_dock_envelope", "Making a dock", "still"),
    ("build_generate", "Generate a hull", "still"),
    ("novaos_open", "Opening NOVA OS", "loop"),
    ("novaos_terminal", "The prompt", "loop"),
    ("novaos_view", "Turning the model", "loop"),
    ("novaos_commands", "What you can type", "loop"),
    ("novaos_contacts", "Reading contacts", "still"),
    ("novaos_service", "Repair and reload a section", "still"),
    ("novaos_rebind_section", "Rebinding a section", "still"),
    ("advanced_scenarios", "Scenarios and campaigns", "still"),
    ("advanced_mods", "Mods", "still"),
    ("advanced_bindings", "Rebinding controls", "still"),
    ("advanced_mouse", "Mouse sensitivity", "still"),
]


def seed_of(lesson_id):
    return int(hashlib.sha256(lesson_id.encode("utf-8")).hexdigest(), 16)


def ink_of(lesson_id):
    return INKS[seed_of(lesson_id) % len(INKS)]


def field(frame, x0, y0, w, h, stars):
    """The phosphor screen: a SCREEN_0 -> SCREEN_1 gradient and a starfield.

    `stars` is a fixed list of (x, y, amount) in cell space, so every cell of a
    sheet shares one sky and only the moving parts move."""
    for y in range(h):
        t = y / (h - 1)
        color = tuple(int(FIELD_TOP[c] + (FIELD_BOTTOM[c] - FIELD_TOP[c]) * t) for c in range(3))
        for x in range(w):
            frame.set(x0 + x, y0 + y, color)
    for sx, sy, amount in stars:
        frame.add(x0 + sx, y0 + sy, (255, 255, 255), amount)


def starfield(rng, w, h, count):
    return [(rng.randrange(w), rng.randrange(h), rng.uniform(0.05, 0.22))
            for _ in range(count)]


def chevron(frame, x0, y0, cx, cy, heading, ink, scale=1.0):
    """A ship: a small arrowhead pointing along `heading` radians."""
    nose = (cx + math.cos(heading) * 7 * scale, cy + math.sin(heading) * 7 * scale)
    left = (cx + math.cos(heading + 2.5) * 6 * scale, cy + math.sin(heading + 2.5) * 6 * scale)
    right = (cx + math.cos(heading - 2.5) * 6 * scale, cy + math.sin(heading - 2.5) * 6 * scale)
    for a, b in ((nose, left), (nose, right), (left, right)):
        line(frame, x0, y0, a, b, ink, 1.0)


def line(frame, x0, y0, a, b, ink, amount):
    steps = int(max(abs(b[0] - a[0]), abs(b[1] - a[1]))) + 1
    for step in range(steps + 1):
        t = step / steps
        frame.add(x0 + int(a[0] + (b[0] - a[0]) * t),
                  y0 + int(a[1] + (b[1] - a[1]) * t), ink, amount)


def ring(frame, x0, y0, cx, cy, radius, ink, amount):
    steps = max(16, int(radius * 8))
    for step in range(steps):
        angle = step / steps * math.tau
        frame.add(x0 + int(cx + math.cos(angle) * radius),
                  y0 + int(cy + math.sin(angle) * radius), ink, amount)


def scanlines(frame, x0, y0, w, h, ink):
    for y in range(0, h, 2):
        for x in range(w):
            i = ((y0 + y) * frame.w + (x0 + x)) * 4
            for c in range(3):
                frame.px[i + c] = int(frame.px[i + c] * 0.62)
    for x in range(w):
        frame.add(x0 + x, y0, ink, 0.30)
        frame.add(x0 + x, y0 + h - 1, ink, 0.30)
    for y in range(h):
        frame.add(x0, y0 + y, ink, 0.30)
        frame.add(x0 + w - 1, y0 + y, ink, 0.30)


def caption(frame, x0, y0, w, text, ink, scale, amount):
    """One dim centred line of small type."""
    upper = text.upper()
    x = x0 + (w - text_width(upper, scale)) // 2
    draw_glyph_line(frame, upper, x, y0, scale, ink, amount)


def title_block(frame, x0, y0, w, h, title, ink):
    """The lesson title, wrapped and centred, with the CRT misconvergence."""
    scale, lines = layout_title(title.upper(), w - 64, 3)
    line_height = (GLYPH_H + 2) * scale
    block_h = line_height * len(lines) - 2 * scale
    top = y0 + (h - block_h) // 2
    for index, text in enumerate(lines):
        x = x0 + (w - text_width(text, scale)) // 2
        y = top + index * line_height
        dots = glyph_mask(text, scale)
        for dx, dy in dots:
            for ox, oy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                frame.add(x + dx + ox, y + dy + oy, ink, 0.18)
        for dx, dy in dots:
            frame.add(x + dx - 2, y + dy, (ink[0], 0, 0), 0.55)
            frame.add(x + dx + 2, y + dy, (0, 0, ink[2]), 0.55)
        for dx, dy in dots:
            frame.set(x + dx, y + dy, ink)


def render_still(lesson_id, title):
    """One 480x270 still: the title over a schematic drawn from the id."""
    rng = random.Random(seed_of(lesson_id))
    ink = ink_of(lesson_id)
    frame = Frame(STILL_W, STILL_H)
    field(frame, 0, 0, STILL_W, STILL_H, starfield(rng, STILL_W, STILL_H, 110))

    # A schematic behind the type, so the eleven stills are not eleven title
    # cards: concentric rings, a grid, and a hull outline, chosen by the id.
    cx, cy = STILL_W / 2, STILL_H / 2
    shape = seed_of(lesson_id) % 3
    if shape == 0:
        for radius in (40, 70, 100):
            ring(frame, 0, 0, cx, cy, radius, ink, 0.22)
    elif shape == 1:
        for step in range(-4, 5):
            line(frame, 0, 0, (cx + step * 36, 24), (cx + step * 36, STILL_H - 24), ink, 0.12)
            line(frame, 0, 0, (40, cy + step * 26), (STILL_W - 40, cy + step * 26), ink, 0.12)
    else:
        hull = [(cx - 90, cy), (cx - 40, cy - 34), (cx + 70, cy - 22),
                (cx + 96, cy), (cx + 70, cy + 22), (cx - 40, cy + 34)]
        for index in range(len(hull)):
            line(frame, 0, 0, hull[index], hull[(index + 1) % len(hull)], ink, 0.26)

    title_block(frame, 0, 0, STILL_W, STILL_H, title, ink)
    caption(frame, 0, STILL_H - 22, STILL_W, f"{lesson_id.replace('_', ' ')} - placeholder",
            ink, 1, 0.45)
    scanlines(frame, 0, 0, STILL_W, STILL_H, ink)
    return frame.bytes()


def render_loop(lesson_id, title):
    """One 4x5 sheet of twenty 240x135 cells carrying two seconds of motion."""
    rng = random.Random(seed_of(lesson_id))
    ink = ink_of(lesson_id)
    stars = starfield(rng, CELL_W, CELL_H, 45)
    motion = seed_of(lesson_id) % 3
    sheet = Frame(CELL_W * COLUMNS, CELL_H * ROWS)

    for nth in range(FRAMES):
        x0 = (nth % COLUMNS) * CELL_W
        y0 = (nth // COLUMNS) * CELL_H
        t = nth / FRAMES
        field(sheet, x0, y0, CELL_W, CELL_H, stars)
        cx, cy = CELL_W / 2, CELL_H / 2

        if motion == 0:
            # A run across the cell, wrapping: the ship holds its heading and
            # the marks it passes fall behind it.
            ship_x = (t * (CELL_W + 60)) - 30
            for mark in (0.25, 0.55, 0.85):
                ring(sheet, x0, y0, CELL_W * mark, cy + 26, 4, ink, 0.30)
            line(sheet, x0, y0, (0, cy), (CELL_W, cy), ink, 0.10)
            for trail in range(1, 6):
                line(sheet, x0, y0, (ship_x - trail * 6, cy), (ship_x - trail * 6 - 3, cy),
                     ink, 0.30 - trail * 0.05)
            chevron(sheet, x0, y0, ship_x, cy, 0.0, ink)
        elif motion == 1:
            # A circle held around a body: the orbit the flight computer flies.
            radius = 42
            ring(sheet, x0, y0, cx, cy, radius, ink, 0.18)
            ring(sheet, x0, y0, cx, cy, 11, ink, 0.45)
            angle = t * math.tau
            ship_x = cx + math.cos(angle) * radius
            ship_y = cy + math.sin(angle) * radius
            chevron(sheet, x0, y0, ship_x, ship_y, angle + math.pi / 2, ink)
        else:
            # A sweep: the radar arm going round, contacts lighting as it
            # passes them.
            angle = t * math.tau
            ring(sheet, x0, y0, cx, cy, 52, ink, 0.16)
            ring(sheet, x0, y0, cx, cy, 26, ink, 0.10)
            line(sheet, x0, y0, (cx, cy),
                 (cx + math.cos(angle) * 52, cy + math.sin(angle) * 52), ink, 0.55)
            for contact in range(4):
                spot = contact / 4 * math.tau + 0.4
                lit = 0.8 if abs(((angle - spot + math.pi) % math.tau) - math.pi) < 0.5 else 0.22
                ring(sheet, x0, y0, cx + math.cos(spot) * 38, cy + math.sin(spot) * 38,
                     3, ink, lit)

        caption(sheet, x0, y0 + CELL_H - 16, CELL_W, title, ink, 1, 0.40)
        scanlines(sheet, x0, y0, CELL_W, CELL_H, ink)

    return sheet.bytes()


def raster(lesson_id, title, kind):
    """One lesson's finished art as (width, height, RGBA bytes)."""
    if kind == "still":
        return STILL_W, STILL_H, render_still(lesson_id, title)
    return CELL_W * COLUMNS, CELL_H * ROWS, render_loop(lesson_id, title)


def ffmpeg(args, stdin=None):
    """Run ffmpeg over a pipe, or say which tool is missing."""
    try:
        done = subprocess.run(["ffmpeg", "-v", "error", *args],
                              input=stdin, stdout=subprocess.PIPE, check=True)
    except FileNotFoundError:
        print("!! ffmpeg is not on PATH - run inside `nix develop`", file=sys.stderr)
        raise SystemExit(1)
    return done.stdout


def encoded(lesson_id, title, kind):
    """The exact WebP file bytes for one lesson, without touching the disk.

    LOSSLESS: this art is flat phosphor drawing, where lossy chroma would eat
    the one-pixel scanlines and the type. Lossy WebP is for the captured
    footage (`scripts/capture-lesson-media.sh`), which is photographic."""
    width, height, pixels = raster(lesson_id, title, kind)
    return ffmpeg(["-f", "rawvideo", "-pix_fmt", "rgba",
                   "-s", f"{width}x{height}", "-i", "-",
                   "-c:v", "libwebp", "-lossless", "1", "-pix_fmt", "bgra",
                   "-f", "webp", "-"], stdin=pixels)


def decoded(path):
    """The committed file as raw RGBA, or `None` if it will not decode."""
    try:
        return ffmpeg(["-i", path, "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
    except subprocess.CalledProcessError:
        return None


def path_of(lesson_id):
    return os.path.join(REPO_ROOT, "assets", "base", "training", f"{lesson_id}.webp")


def captured_lessons():
    """The lesson ids a capture producer owns, read from the capture script.

    The one place that knows which lessons have real footage is
    `scripts/capture-lesson-media.sh`'s own `PRODUCERS` table, so this reads
    THAT rather than guessing from the pixels. Guessing is what a comparison
    against a fresh render would do, and it gets the one case that matters
    backwards: change the authored grid and every stale placeholder stops
    matching, which would read as "real footage, leave it alone" and quietly
    ship 21 sheets cut on the wrong grid."""
    script = os.path.join(REPO_ROOT, "scripts", "capture-lesson-media.sh")
    with open(script, encoding="utf-8") as handle:
        source = handle.read()
    table = source.split("\nPRODUCERS=(", 1)[1].split("\n)", 1)[0]
    owned = set()
    for line in table.splitlines():
        line = line.strip().strip('"')
        if "|" not in line:
            continue
        # `example|lesson:kind[,lesson:kind...]` - one row may own several.
        for pair in line.split("|", 1)[1].split(","):
            lesson = pair.split(":", 1)[0].strip()
            if lesson:
                owned.add(lesson)
    return owned


def matches_a_fresh_render(lesson_id, title, kind):
    """True when the committed file still draws exactly this generator's art.

    PIXELS, not file bytes. The comparison has to survive a libwebp that packs
    the same image differently, and a lossless format is what makes comparing
    the decode exact."""
    path = path_of(lesson_id)
    if not os.path.exists(path):
        return False
    _, _, pixels = raster(lesson_id, title, kind)
    return decoded(path) == pixels


def check():
    """Every demonstration is on disk, and every placeholder is up to date.

    Three outcomes per lesson. CAPTURED: a producer owns it, so this only
    checks it is there. STALE: no producer, and the committed file is not what
    this generator draws today - the authored grid moved, or the title did, and
    the file is cut on the old one. MISSING: nothing at the path at all. The
    last two fail."""
    owned = captured_lessons()
    missing, captured, stale = [], [], []
    for lesson_id, title, kind in LESSONS:
        if not os.path.exists(path_of(lesson_id)):
            missing.append(lesson_id)
        elif lesson_id in owned:
            captured.append(lesson_id)
        elif not matches_a_fresh_render(lesson_id, title, kind):
            stale.append(lesson_id)
    for lesson_id in missing:
        print(f"  MISSING  assets/base/training/{lesson_id}.webp")
    for lesson_id in stale:
        print(f"  STALE    assets/base/training/{lesson_id}.webp")
    for lesson_id in captured:
        print(f"  CAPTURED assets/base/training/{lesson_id}.webp (a producer owns it)")
    if missing or stale:
        print(f"\n{len(missing)} missing and {len(stale)} stale of {len(LESSONS)} lesson "
              "demonstration(s) - run scripts/gen-lesson-media.py", file=sys.stderr)
        return 1
    drawn = len(LESSONS) - len(captured)
    print(f"{drawn} placeholder(s) match a fresh render (pixel for pixel); "
          f"{len(captured)} captured.")
    return 0


def generate():
    """Draw a placeholder for every lesson no capture producer owns.

    Captured footage is NEVER overwritten - not because the file looks
    unfamiliar, but because `scripts/capture-lesson-media.sh` declares that
    lesson as its own. To go back to a placeholder, take the producer out of
    that table."""
    owned = captured_lessons()
    os.makedirs(os.path.join(REPO_ROOT, "assets", "base", "training"), exist_ok=True)
    drawn, kept = 0, []
    for lesson_id, title, kind in LESSONS:
        if lesson_id in owned:
            kept.append(lesson_id)
            continue
        with open(path_of(lesson_id), "wb") as handle:
            handle.write(encoded(lesson_id, title, kind))
        drawn += 1
    print(f"wrote {drawn} placeholder demonstration(s) to assets/base/training/")
    for lesson_id in kept:
        print(f"  kept     assets/base/training/{lesson_id}.webp (captured footage)")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true",
                        help="verify the committed files match a fresh render; write nothing")
    args = parser.parse_args()
    return check() if args.check else generate()


if __name__ == "__main__":
    sys.exit(main())
