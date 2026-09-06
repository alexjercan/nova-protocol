#!/usr/bin/env python3
"""Generate the Preface's Industrial Saturn SVG using the Python standard library.

The geometry is illustrative. It does not define station layouts or equipment.
"""

import argparse
from html import escape
from pathlib import Path
import random

WIDTH = 1600
HEIGHT = 1000
OUTPUT = Path(__file__).resolve().parents[1] / "src" / "images" / "industrial-saturn.svg"
GROUND = "#030d08"
INK = "#06170f"
DEEP = "#183426"
MID = "#267b4d"
GREEN = "#4ed184"
BRIGHT = "#c0ffd4"
DIM = "#6ea985"
GOLD = "#c2a23b"
LIME = "#b8da5d"


class Plate:
    """An SVG drawing with accessible CRT presentation."""

    def __init__(self, title: str, description: str):
        self.parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" '
            f'height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}" '
            'role="img" aria-labelledby="title description">',
            f"<title id=\"title\">{escape(title)}</title>",
            f"<desc id=\"description\">{escape(description)}</desc>",
            '<defs><pattern id="scan" width="4" height="4" '
            'patternUnits="userSpaceOnUse"><path d="M0 1H4" '
            'stroke="#001008" stroke-opacity=".24"/></pattern>'
            '<radialGradient id="tube"><stop offset="55%" '
            'stop-color="#030d08" stop-opacity="0"/>'
            '<stop offset="100%" stop-color="#030d08" stop-opacity=".48"/>'
            '</radialGradient></defs>',
        ]
        self.rect(0, 0, WIDTH, HEIGHT, GROUND)

    def element(self, name: str, **attrs):
        """Append an empty SVG element with escaped attributes."""
        rendered = " ".join(
            f'{key.replace("_", "-")}="{escape(str(value), quote=True)}"'
            for key, value in attrs.items()
        )
        self.parts.append(f"<{name} {rendered}/>")

    def rect(self, x, y, width, height, fill=INK, stroke=None, **attrs):
        """Draw a rectangle."""
        if stroke is not None:
            attrs["stroke"] = stroke
        self.element("rect", x=x, y=y, width=width, height=height, fill=fill, **attrs)

    def path(self, d, stroke=GREEN, width=2, fill="none", **attrs):
        """Draw a filled or stroked path."""
        self.element("path", d=d, stroke=stroke, stroke_width=width, fill=fill, **attrs)

    def polygon(self, points, fill=INK, stroke=MID, **attrs):
        """Draw a polygon from coordinate pairs."""
        self.element(
            "polygon", points=" ".join(f"{x},{y}" for x, y in points),
            fill=fill, stroke=stroke, **attrs,
        )

    def text(self, x, y, value, size=20, fill=DIM, **attrs):
        """Draw an escaped monospace label."""
        self.parts.append(
            f'<text x="{x}" y="{y}" font-family="DejaVu Sans Mono, monospace" '
            f'font-size="{size}" fill="{fill}" '
            + " ".join(f'{k.replace("_", "-")}="{escape(str(v))}"' for k, v in attrs.items())
            + f">{escape(value)}</text>"
        )

    def group(self, transform):
        """Begin a transformed drawing group."""
        self.parts.append(f'<g transform="{escape(transform, quote=True)}">')

    def end_group(self):
        """Close the current group."""
        self.parts.append("</g>")

    def finish(self, title, subtitle):
        """Finish the drawing with a frame, static scanlines, and status labels."""
        w, h = WIDTH, HEIGHT
        self.rect(24, 24, w - 48, h - 48, "none", GREEN, stroke_width=3)
        self.rect(44, 44, w - 88, h - 88, "none", DEEP)
        self.rect(58, 58, 8, 52, LIME)
        self.text(84, 83, title, 28, BRIGHT, letter_spacing=3)
        self.text(84, 110, subtitle, 16)
        self.rect(58, h - 94, w - 116, 38, GROUND)
        self.path(f"M58 {h - 94}H{w - 58}", MID)
        self.text(76, h - 67, "NOVA PROTOCOL / WORLD STUDIES", 16)
        self.text(w - 80, h - 67, "VISUAL PROPOSAL / NOT TO SCALE", 16, LIME, text_anchor="end")
        self.rect(45, 45, w - 90, h - 90, "url(#scan)")
        self.rect(26, 26, w - 52, h - 52, "url(#tube)")
        self.parts.append("</svg>")
        return "\n".join(self.parts) + "\n"


def stars(p: Plate, seed: int):
    """Draw a seeded star field without assigning astronomical positions."""
    rng = random.Random(seed)
    for _ in range(190):
        x, y = rng.randint(70, 1530), rng.randint(145, 875)
        size = rng.choice([1, 1, 2, 3])
        p.rect(x, y, size, size, rng.choice([MID, DIM, BRIGHT]))


def panel(p: Plate, x, y, width, height, seed):
    """Draw a worn machinery panel with vents, fasteners, and a service light."""
    rng = random.Random(seed)
    p.rect(x, y, width, height, INK, MID, stroke_width=2)
    p.path(f"M{x + 5} {y + height - 5}V{y + 5}H{x + width - 5}", DEEP)
    for bx in [x + 8, x + width - 8]:
        for by in [y + 8, y + height - 8]:
            p.rect(bx - 1, by - 1, 3, 3, DIM)
    for vy in range(int(y + 20), int(y + height - 14), 8):
        p.path(f"M{x + 16} {vy}H{x + width * .62}", DEEP, 3)
    p.rect(x + width - 19, y + 15, 5, 14, rng.choice([GREEN, GOLD, MID]))
    for _ in range(9):
        sx = rng.randint(int(x + 12), int(x + width - 12))
        sy = rng.randint(int(y + 12), int(y + height - 12))
        p.path(f"M{sx} {sy}h{rng.randint(2, 8)}", MID, 1, opacity=".5")


def truss(p: Plate, x, y, width, height, bays):
    """Draw an industrial lattice girder."""
    p.rect(x, y, width, height, INK, MID, stroke_width=3)
    step = width / bays
    for i in range(bays):
        a, b = x + i * step, x + (i + 1) * step
        p.path(f"M{a} {y}L{b} {y + height}V{y}L{a} {y + height}", MID, 2)
    p.path(f"M{x} {y}H{x + width}", GREEN, 2)


def industrial_saturn():
    """Draw a ring-side industrial silhouette with a layered Saturn backdrop."""
    p = Plate("Industrial Saturn", "Concept art of a large industrial station beneath Saturn and its rings. No specific station or orbital layout is established.")
    stars(p, 41)
    p.group("translate(942 372) rotate(-19)")
    for radius in range(334, 601, 7):
        p.element("ellipse", cx=0, cy=0, rx=radius, ry=radius * .235,
                  fill="none", stroke=MID if radius % 3 else DIM,
                  stroke_width=2, opacity=".65")
    p.end_group()
    p.parts.append('<defs><clipPath id="planet"><circle cx="942" cy="372" r="225"/></clipPath></defs>')
    p.element("circle", cx=942, cy=372, r=225, fill=DEEP, stroke=DIM, stroke_width=2)
    p.parts.append('<g clip-path="url(#planet)" transform="rotate(-19 942 372)">')
    for i in range(20):
        p.path(f"M690 {177 + i * 21}Q945 {214 + i * 21} 1190 {177 + i * 21}",
               [MID, DIM, INK, MID][i % 4], [5, 2, 11, 4][i % 4], opacity=".6")
    p.element("ellipse", cx=1060, cy=375, rx=155, ry=270, fill=GROUND, opacity=".62")
    p.end_group()
    p.group("translate(942 372) rotate(-19)")
    for radius in range(334, 601, 5):
        ry = radius * .235
        p.path(f"M{-radius} 0A{radius} {ry} 0 0 0 {radius} 0",
               DIM if radius % 4 == 0 else MID, 2, opacity=".9")
    p.end_group()

    truss(p, 110, 583, 1380, 69, 20)
    p.polygon([(212, 652), (1312, 652), (1462, 716), (1462, 850),
               (1388, 886), (274, 886), (156, 820), (156, 694)], INK, GREEN, stroke_width=3)
    p.polygon([(212, 652), (1312, 652), (1462, 716), (322, 716)], DEEP, MID)
    for i in range(12):
        panel(p, 336 + i * 84, 738, 74, 105, i)
        p.path(f"M{330 + i * 84} 715l-70 -59", MID)
    p.rect(280, 723, 30, 129, DEEP, MID)
    for y in range(735, 849, 12):
        p.path(f"M283 {y}h24", GOLD, 4)
    for x, y, w, h in [(335, 348, 206, 235), (572, 461, 157, 122),
                        (1080, 403, 188, 180), (1320, 506, 104, 77)]:
        p.polygon([(x, y), (x + 25, y - 23), (x + w + 25, y - 23),
                   (x + w, y)], DEEP, GREEN)
        p.polygon([(x + w, y), (x + w + 25, y - 23),
                   (x + w + 25, y + h - 23), (x + w, y + h)], GROUND, MID)
        p.rect(x, y, w, h, INK, GREEN, stroke_width=2)
        for row in range(int(h // 49)):
            panel(p, x + 12, y + 10 + row * 49, w - 24, 40, row + x)
    truss(p, 285, 307, 320, 32, 8)
    p.path("M372 305V207H469V280M418 207V166M401 180H435", GREEN, 3)
    p.path("M1163 379V280H1291V315M1174 285V365", MID, 4)
    p.rect(752, 527, 256, 54, DEEP, GREEN)
    p.text(774, 562, "CARGO / SERVICE", 21, BRIGHT)
    for x in range(118, 1490, 45):
        p.path(f"M{x} 582V564h24", DIM, 2)
    for i in range(4):
        p.path(f"M{164 + i * 18} 806V692H{292 + i * 18}V623", MID, 7)
    p.path("M103 849H197M139 849V878M129 878H149", DIM, 2)
    return p.finish("INDUSTRIAL SATURN", "Homes inside the machinery of extraction")


def main():
    """Write only the Preface's image to its fixed source path."""
    argparse.ArgumentParser(description=__doc__).parse_args()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_bytes(industrial_saturn().encode("utf-8"))
    print(f"Wrote {OUTPUT.name}")


if __name__ == "__main__":
    main()
