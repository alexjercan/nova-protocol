#!/usr/bin/env python3
"""Pixel-block comic panel candidates in the campaign portrait style.

Every panel is 16 px blocks inside the CRT frame used by
art/portrait-candidates/industrial-commander-crt-green.svg: dark ground, bezel,
shelf lines, scanlines, status bars. Run from the repository root:

    python3 tasks/20260905-094102/art/pixel_panels.py

It writes the-strike-*.svg next to itself.
"""

from __future__ import annotations

import pathlib
import random

PAL = {
    "K": "#8f8656",  # khaki hull
    "L": "#b0aa75",  # light khaki
    "O": "#5d603e",  # olive
    "o": "#6d6845",  # olive light
    "D": "#344832",  # dark hull, unlit
    "G": "#4ed184",  # phosphor
    "g": "#267b4d",  # phosphor mid
    "W": "#c0ffd4",  # phosphor bright
    "d": "#183426",  # deep green, dead window
    "E": "#245c3d",
    "e": "#347951",
    "A": "#c2a23b",  # gold
    "a": "#9d8128",  # dark gold
    "b": "#796424",
    "Y": "#b8da5d",  # lime
    "P": "#bdba7d",  # pale
    "N": "#04120b",  # near black
    "S": "#0b2b1c",  # shadow
    "R": "#164c31",  # rim
}

BLOCK = 16
INSET = 48

CARRIER = """
.......LLLLLLLLLLLL......
....KKKKKKKKKKKKKKKKKK...
..KKKdGdKKKdGdKKKdGdKKK..
.KKKKKKKKKKKKKKKKKKKKKKKK
..KKKdGdKKKdGdKKKdGdKKK..
....OOOOOOOOOOOOOOOOOO...
.......OOOAAAAAAOOO......
"""

CARRIER_MID = """
.....LLLLLLLL....
..KKKKKKKKKKKKKK.
.KKdGdKKdGdKKdGdK
KKKKKKKKKKKKKKKKK
..OOOOOOOOOOOOOO.
.....OOAAAAOO....
"""

CARRIER_SMALL = """
...LLLLLL...
.KKdGdKdGdK.
KKKKKKKKKKKK
..OOOAAAOO..
"""

WARSHIP = """
......DDDDDDDDDD
...EDDDDDDDDDDDa
EEaDDDDDDDDDDDDD
...EDDDDDDDDDDDa
......DDDDDDDDDD
"""

CUTTER = """
.KKG
OOO.
"""

CUTTER_DARK = """
.KKd
OOO.
"""

CUTTER_BIG_DARK = """
..KKKKKd
.KKKKKKK
OOOOOOO.
..OO.OO.
"""

SKIFF = "aY"
SKIFF_LEFT = "Ya"

HELMET = """
..LLLL..
.LLLLLL.
LLLLLLLL
LLLLLLLL
KKKKKKKK
.OOOOOO.
"""


def lines(sprite: str) -> list[str]:
    return [row for row in sprite.strip("\n").split("\n")]


class Canvas:
    """A block grid inside the frame. Coordinates are blocks, not pixels."""

    def __init__(self, cols: int, rows: int) -> None:
        self.cols, self.rows = cols, rows
        self.px: dict[tuple[int, int], str] = {}

    def put(self, x: int, y: int, ch: str) -> None:
        if 0 <= x < self.cols and 0 <= y < self.rows and ch != ".":
            self.px[(x, y)] = ch

    def blit(self, sprite: str, x: int, y: int) -> None:
        for dy, row in enumerate(lines(sprite)):
            for dx, ch in enumerate(row):
                self.put(x + dx, y + dy, ch)

    def fill(self, x0: int, y0: int, x1: int, y1: int, ch: str) -> None:
        for y in range(y0, y1):
            for x in range(x0, x1):
                self.put(x, y, ch)

    def disk(self, cx: int, cy: int, r: int, fill: str, rim: str | None, limb: str | None) -> None:
        for y in range(cy - r, cy + r + 1):
            for x in range(cx - r, cx + r + 1):
                d2 = (x - cx) ** 2 + (y - cy) ** 2
                if d2 <= r * r:
                    ch = fill
                    if rim and d2 >= (r - 1) ** 2 and x < cx:
                        ch = rim
                    if limb and d2 >= (r - 1) ** 2 and x > cx + r // 2:
                        ch = limb
                    self.put(x, y, ch)

    def line(self, x0: int, y0: int, x1: int, y1: int, ch: str, glow: str | None = None) -> None:
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
        err = dx + dy
        x, y = x0, y0
        i = 0
        while True:
            if glow and i % 2 == 0:
                self.put(x, y - 1, glow)
                self.put(x, y + 1, glow)
            self.put(x, y, ch)
            if x == x1 and y == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x += sx
            if e2 <= dx:
                err += dx
                y += sy
            i += 1

    def burst(self, cx: int, cy: int, r: int, spikes: int) -> None:
        for y in range(cy - r, cy + r + 1):
            for x in range(cx - r, cx + r + 1):
                m = abs(x - cx) + abs(y - cy)
                if m <= r:
                    self.put(x, y, "W" if m <= r // 3 else "G" if m <= (2 * r) // 3 else "g")
        for k in range(r + 1, r + spikes + 1):
            for x, y in ((cx + k, cy), (cx - k, cy), (cx, cy + k), (cx, cy - k)):
                self.put(x, y, "W" if k <= r + spikes // 2 else "G")

    def junk(self, seed: int, x0: int, y0: int, x1: int, y1: int, n: int, chars: str = "OodES") -> None:
        rng = random.Random(seed)
        shapes = [(1, 1), (2, 1), (1, 2), (3, 1), (2, 2), (1, 1), (2, 1)]
        for _ in range(n):
            w, h = rng.choice(shapes)
            x, y = rng.randint(x0, x1 - w), rng.randint(y0, y1 - h)
            if any(self.px.get((x + i, y + j)) not in (None, "N") for i in range(w) for j in range(h)):
                continue
            ch = rng.choice(chars)
            self.fill(x, y, x + w, y + h, ch)

    def rects(self, ox: int, oy: int) -> list[str]:
        out = []
        for y in range(self.rows):
            x = 0
            while x < self.cols:
                ch = self.px.get((x, y))
                if ch is None:
                    x += 1
                    continue
                run = 1
                while self.px.get((x + run, y)) == ch:
                    run += 1
                out.append(
                    f'  <rect x="{ox + x * BLOCK}" y="{oy + y * BLOCK}" '
                    f'width="{run * BLOCK}" height="{BLOCK}" fill="{PAL[ch]}"/>'
                )
                x += run
        return out


def frame(w: int, h: int, oy: int, canvas: Canvas, shelves: bool) -> list[str]:
    """The CRT frame at vertical offset oy, with the canvas drawn inside it."""
    y = oy
    out = [
        f'  <rect y="{y}" width="{w}" height="{h}" fill="#030d08"/>',
        f'  <rect x="16" y="{y + 16}" width="{w - 32}" height="{h - 32}" fill="#082117" stroke="#55d68b" stroke-width="8"/>',
        f'  <rect x="32" y="{y + 32}" width="{w - 64}" height="{h - 64}" fill="#0b2b1c"/>',
        f'  <rect x="{INSET}" y="{y + INSET}" width="{w - 2 * INSET}" height="{h - 2 * INSET}" fill="#06170f"/>',
    ]
    if shelves:
        d = "".join(f"M{INSET} {yy}H{w - INSET}" for yy in range(y + 112, y + h - INSET, 64))
        out.append(f'  <path d="{d}" stroke="#164c31" stroke-width="4" opacity=".8"/>')
    out += canvas.rects(INSET, y + INSET)
    d = "".join(f"M{INSET} {yy}H{w - INSET}" for yy in range(y + 56, y + h - INSET, 16))
    out.append(f'  <path d="{d}" stroke="#001008" stroke-width="2" opacity=".55"/>')
    out.append(f'  <rect x="{INSET}" y="{y + INSET}" width="96" height="8" fill="#b8da5d"/>')
    out.append(f'  <rect x="{w - INSET - 96}" y="{y + h - INSET - 8}" width="96" height="8" fill="#55d68b"/>')
    return out


def svg(w: int, h: int, body: list[str]) -> str:
    head = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
        f'viewBox="0 0 {w} {h}" shape-rendering="crispEdges">'
    )
    return "\n".join([head, *body, "</svg>", ""])


def grid(w: int, h: int) -> Canvas:
    return Canvas((w - 2 * INSET) // BLOCK, (h - 2 * INSET) // BLOCK)


# ---------------------------------------------------------------- variants

def strike_a_wide() -> str:
    """The shot. Severance out of the moonlet's shadow, the bolt, Meridian hit."""
    w, h = 768, 448
    c = grid(w, h)
    c.junk(7, 0, 15, 16, 22, 34)
    c.blit(CUTTER, 4, 18)
    c.disk(37, 7, 7, "S", "R", "g")
    c.blit(WARSHIP, 25, 5)
    c.put(25, 7, "A")
    c.blit(CARRIER_MID, 1, 6)
    c.line(24, 7, 18, 9, "W", "G")
    c.burst(17, 9, 3, 2)
    return svg(w, h, frame(w, h, 0, c, shelves=True))


def strike_b_cockpit() -> str:
    """From Cutter One. The flash through the canopy, the guard channel in pieces."""
    w, h = 768, 448
    c = grid(w, h)
    c.fill(0, 0, 42, 15, "N")
    c.fill(0, 0, 42, 1, "K")
    c.fill(0, 0, 1, 15, "K")
    c.fill(41, 0, 42, 15, "K")
    c.fill(20, 0, 22, 15, "O")
    c.junk(3, 2, 6, 14, 15, 14, "KOodD")
    c.junk(4, 22, 9, 41, 15, 8, "dSDO")
    c.blit(CARRIER_SMALL, 25, 4)
    c.burst(31, 5, 2, 2)
    c.put(40, 3, "a")
    c.fill(0, 15, 42, 22, "O")
    c.fill(0, 15, 42, 16, "K")
    c.fill(2, 17, 18, 21, "N")
    bars = [3, 1, 4, 0, 0, 2, 5, 0, 3, 4, 0, 0, 1, 4, 2, 0]
    for i, b in enumerate(bars):
        for k in range(b):
            c.put(3 + i, 20 - k, "W" if k == b - 1 else "G")
    for i, ch in enumerate("GGYGaGG"):
        c.put(24 + i * 2, 17, ch)
    c.fill(24, 19, 38, 20, "d")
    c.fill(24, 19, 30, 20, "g")
    c.blit(HELMET, 6, 11)
    c.blit(HELMET, 28, 11)
    return svg(w, h, frame(w, h, 0, c, shelves=False))


def strike_c_flash() -> str:
    """Silhouettes against the flash. Almost no colour."""
    w, h = 768, 448
    c = grid(w, h)
    for x in range(0, 42):
        m = abs(x - 24)
        c.put(x, 9, "W" if m <= 8 else "G" if m <= 14 else "g" if m <= 20 else "E")
    c.burst(24, 9, 8, 0)
    for k, ch in ((9, "g"), (10, "E"), (11, "d")):
        for x in range(24 - k, 24 + k + 1):
            m = abs(x - 24)
            if m == k:
                for y in range(9 - (k - m), 9 + (k - m) + 1):
                    c.put(x, y, ch)
    left = "\n".join(row[:12] for row in lines(CARRIER))
    right = "\n".join(row[12:] for row in lines(CARRIER))
    dark = str.maketrans("KLOAGd", "DDdbdd")
    c.blit(left.translate(dark), 12, 7)
    c.blit(right.translate(dark), 26, 8)
    c.blit(WARSHIP, 32, 1)
    c.junk(11, 0, 14, 12, 22, 16, "dSD")
    c.blit(CUTTER_DARK, 3, 18)
    c.put(6, 18, "Y")
    return svg(w, h, frame(w, h, 0, c, shelves=False))


def strike_d_aftermath() -> str:
    """After. Two halves, skiffs sweeping, the cutter dark in the junk."""
    w, h = 768, 448
    c = grid(w, h)
    c.disk(36, 5, 5, "S", "R", "g")
    left = "\n".join(row[:11] for row in lines(CARRIER))
    right = "\n".join(row[11:] for row in lines(CARRIER))
    dead = str.maketrans("G", "d")
    c.blit(left.translate(dead), 8, 5)
    c.blit(right.translate(dead), 21, 9)
    c.put(13, 7, "G")
    c.put(24, 13, "G")
    c.junk(21, 17, 4, 24, 15, 14, "KOodA")
    for x, y in ((30, 3), (33, 6), (34, 10), (31, 14), (27, 16)):
        c.blit(SKIFF_LEFT, x, y)
    c.junk(9, 0, 14, 13, 22, 26)
    c.blit(CUTTER_DARK, 3, 18)
    return svg(w, h, frame(w, h, 0, c, shelves=True))


def strike_e_page() -> str:
    """Three panels on one page: the shadow, the shot, the junk."""
    w, ph = 768, 320
    body: list[str] = []
    # panel 1: the shadow
    c = grid(w, ph)
    c.junk(5, 0, 9, 14, 14, 18)
    c.blit(CUTTER, 3, 11)
    c.blit(CARRIER_MID, 3, 3)
    c.disk(37, 6, 6, "S", "R", "g")
    c.put(31, 7, "a")
    body += frame(w, ph, 0, c, shelves=True)
    # panel 2: the shot
    c = grid(w, ph)
    c.disk(37, 6, 6, "S", "R", "g")
    c.blit(WARSHIP, 26, 4)
    c.put(26, 6, "A")
    c.blit(CARRIER_MID, 3, 3)
    c.line(25, 6, 20, 6, "W", "G")
    c.burst(19, 6, 3, 2)
    c.junk(5, 0, 9, 14, 14, 18)
    c.blit(CUTTER, 3, 11)
    body += frame(w, ph, ph, c, shelves=True)
    # panel 3: the junk
    c = grid(w, ph)
    left = "\n".join(row[:11] for row in lines(CARRIER_SMALL))
    right = "\n".join(row[11:] for row in lines(CARRIER_SMALL))
    dead = str.maketrans("G", "d")
    c.blit(left.translate(dead), 27, 2)
    c.blit(right.translate(dead), 36, 4)
    for x, y in ((24, 2), (31, 6), (37, 1)):
        c.blit(SKIFF_LEFT, x, y)
    c.junk(13, 0, 2, 22, 14, 60, "OodESD")
    c.fill(6, 6, 18, 13, "N")
    c.blit(CUTTER_BIG_DARK, 8, 8)
    c.put(15, 8, "g")
    body += frame(w, ph, 2 * ph, c, shelves=False)
    return svg(w, 3 * ph, body)


VARIANTS = {
    "the-strike-a-wide.svg": strike_a_wide,
    "the-strike-b-cockpit.svg": strike_b_cockpit,
    "the-strike-c-flash.svg": strike_c_flash,
    "the-strike-d-aftermath.svg": strike_d_aftermath,
    "the-strike-e-page.svg": strike_e_page,
}


def main() -> None:
    here = pathlib.Path(__file__).resolve().parent
    for name, make in VARIANTS.items():
        (here / name).write_text(make())
        print(name)


if __name__ == "__main__":
    main()
