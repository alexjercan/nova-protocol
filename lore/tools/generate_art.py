#!/usr/bin/env python3
"""Generate deterministic CRT concept plates using only the Python standard library.

The geometry is illustrative. It does not define station layouts or equipment.
"""

import argparse
from html import escape
from pathlib import Path
import random

WIDTH = 1600
HEIGHT = 1000
OUTPUT = Path(__file__).resolve().parents[1] / "src" / "images"
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
    """An SVG drawing with shared, accessible CRT presentation."""

    def __init__(self, title: str, description: str, width=WIDTH, height=HEIGHT):
        self.width, self.height = width, height
        self.parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" '
            f'height="{height}" viewBox="0 0 {width} {height}" '
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
        self.rect(0, 0, width, height, GROUND)

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

    def finish(self, title, subtitle, status="VISUAL PROPOSAL / NOT TO SCALE"):
        """Finish the drawing with a frame, static scanlines, and status labels."""
        w, h = self.width, self.height
        self.rect(24, 24, w - 48, h - 48, "none", GREEN, stroke_width=3)
        self.rect(44, 44, w - 88, h - 88, "none", DEEP)
        self.rect(58, 58, 8, 52, LIME)
        self.text(84, 83, title, 28, BRIGHT, letter_spacing=3)
        self.text(84, 110, subtitle, 16)
        self.rect(58, h - 94, w - 116, 38, GROUND)
        self.path(f"M58 {h - 94}H{w - 58}", MID)
        self.text(76, h - 67, "NOVA PROTOCOL / WORLD STUDIES", 16)
        self.text(w - 80, h - 67, status, 16, LIME, text_anchor="end")
        self.rect(45, 45, w - 90, h - 90, "url(#scan)")
        self.rect(26, 26, w - 52, h - 52, "url(#tube)")
        self.parts.append("</svg>")
        return "\n".join(self.parts) + "\n"


def stars(p: Plate, seed: int, bounds=(70, 145, 1530, 875)):
    """Draw a seeded star field without assigning astronomical positions."""
    rng = random.Random(seed)
    left, top, right, bottom = bounds
    for _ in range(190):
        x, y = rng.randint(left, right), rng.randint(top, bottom)
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


def worker(p: Plate, x, y, scale=1, coat=MID):
    """Draw an anonymous, unranked worker silhouette, not a character portrait."""
    p.group(f"translate({x} {y}) scale({scale})")
    p.polygon([(8, 76), (29, 53), (68, 53), (89, 75), (103, 131),
               (85, 139), (72, 104), (72, 164), (26, 164), (26, 106),
               (16, 139), (0, 132)], coat, DIM, stroke_width=2)
    p.polygon([(29, 16), (39, 7), (63, 7), (74, 20), (74, 43),
               (62, 55), (38, 51), (27, 40)], DEEP, GREEN, stroke_width=2)
    p.rect(31, 23, 39, 15, INK, MID)
    p.path("M34 25H64", BRIGHT, 2)
    p.rect(23, 29, 7, 18, GOLD)
    p.rect(34, 65, 29, 6, LIME)
    p.path("M48 77V150M27 112H72", INK, 5)
    p.polygon([(26, 159), (48, 159), (43, 218), (17, 218)], DEEP, MID)
    p.polygon([(51, 159), (73, 159), (84, 218), (56, 218)], DEEP, MID)
    p.rect(14, 211, 29, 10, INK, MID)
    p.rect(56, 211, 31, 10, INK, MID)
    p.end_group()


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


def company_habitat():
    """Draw company sleeping compartments compressed around an industrial passage."""
    p = Plate("Company housing", "Concept art of stacked sleeping compartments, intrusive industrial services, and an anonymous worker. Not a station floor plan.")
    p.polygon([(72, 150), (1528, 150), (1000, 360), (600, 360)], DEEP)
    p.polygon([(72, 150), (600, 360), (600, 650), (72, 888)], INK)
    p.polygon([(1528, 150), (1000, 360), (1000, 650), (1528, 888)], INK)
    p.rect(600, 360, 400, 290, DEEP, MID)
    p.polygon([(72, 888), (600, 650), (1000, 650), (1528, 888)], DEEP, MID)
    for x in range(90, 1540, 105):
        p.path(f"M{x} 888L800 548", MID, 1)
    for y in [675, 710, 759, 822, 881]:
        p.path(f"M75 {y}H1525", MID, 1)

    def wall_point(t, v):
        x = 78 + 522 * t
        top, bottom = 156 + 204 * t, 886 - 236 * t
        return round(x, 1), round(top + (bottom - top) * v, 1)

    for i, (left, right) in enumerate([(0, .32), (.34, .61), (.63, .83), (.85, .98)]):
        for row, (top, bottom) in enumerate([(.08, .45), (.51, .9)]):
            p.polygon([wall_point(left, top), wall_point(right, top),
                       wall_point(right, bottom), wall_point(left, bottom)], DEEP, GREEN, stroke_width=2)
            p.polygon([wall_point(left + .02, top + .055), wall_point(right - .02, top + .055),
                       wall_point(right - .02, top + .12), wall_point(left + .02, top + .12)], INK, MID)
            a = wall_point(left + .035, bottom - .055)
            b = wall_point(right - .035, bottom - .055)
            p.path(f"M{a[0]} {a[1]}L{b[0]} {b[1]}", GOLD if row == 0 else MID, 4)
            a = wall_point(right - .04, top + .19)
            p.rect(a[0] - 5, a[1], 5, 14, LIME if i % 2 else MID)
    for i in range(5):
        y = 198 + i * 44
        end = 389 + i * 23
        route = f"M1520 {y}H1390L1022 {end}H974"
        p.path(route, GROUND, 22)
        p.path(route, MID if i % 2 else DIM, 13)
        p.path(route, DEEP, 5)
        p.path(f"M1370 {y - 17}v34", GOLD, 4)
    for i in range(4):
        x, y = 1072 + i * 113, 533 + i * 25
        panel(p, x, y, 91, 94 + i * 18, i + 31)
    for x in [290, 535, 1065, 1310]:
        p.path(f"M{x} 151L800 425", MID, 4)
    p.path("M550 173H1050L961 222H639Z", BRIGHT, 2, fill=MID)
    p.path("M682 290H918", BRIGHT, 8)
    p.rect(639, 374, 322, 45, GROUND, MID)
    p.text(800, 403, "WE ARE EXPANDING", 24, LIME, text_anchor="middle")
    p.rect(696, 444, 207, 206, INK, GREEN, stroke_width=3)
    p.path("M800 448V640M710 466H888", MID, 2)
    p.rect(720, 479, 56, 33, DEEP, MID)
    p.rect(816, 479, 56, 33, DEEP, MID)
    p.path("M672 644H926", GOLD, 6)
    p.path("M908 518H953V601H987", MID, 6)
    worker(p, 750, 557, .8)
    p.element("ellipse", cx=790, cy=741, rx=58, ry=12, fill=GROUND, opacity=".7")
    p.path("M1128 804Q1066 836 1138 844T1370 817", GROUND, 10)
    p.path("M1138 804Q1076 835 1148 842T1380 817", MID, 2)
    p.polygon([(180, 865), (480, 865), (518, 846), (218, 846)], GROUND, MID)
    for x in range(210, 473, 18):
        p.path(f"M{x} 859l29 -9", MID, 3)
    return p.finish("COMPANY HOUSING", "A place to sleep is part of the employment relationship")


def salvage_habitat():
    """Draw an improvised shared repair berth without specifying the Roost."""
    p = Plate("Independent repair berth", "Concept art of mismatched industrial compartments, workers, spare equipment, and a shared table. This is not an established layout of the Roost.")
    p.rect(72, 150, 1456, 738, INK, MID)
    stars(p, 83, (930, 203, 1428, 443))
    p.rect(917, 191, 526, 267, "none", DEEP, stroke_width=21)
    p.path("M919 190V456H1441M1180 191V454", GREEN, 3)
    p.path("M961 207L1130 438M989 207L1158 438", MID, 2, opacity=".3")
    p.polygon([(72, 888), (338, 660), (1312, 660), (1528, 888)], DEEP, MID)
    for x in range(90, 1540, 123):
        p.path(f"M{x} 888L825 473", MID, 1)
    for y in [698, 751, 817, 877]:
        p.path(f"M80 {y}H1520", MID, 1)
    truss(p, 74, 154, 1450, 49, 18)
    p.path("M785 205V296Q789 312 805 312H1064", MID, 7)
    p.path("M870 205V319M857 319H883V350H857Z", GREEN, 3)
    p.path("M805 350V425M935 350V425", DIM, 2)
    p.polygon([(734, 450), (802, 395), (1163, 395), (1280, 475),
               (1280, 626), (1218, 685), (753, 685), (701, 625)], DEEP, GREEN, stroke_width=3)
    p.polygon([(802, 395), (1163, 395), (1280, 475), (844, 475)], INK, MID)
    for i in range(4):
        panel(p, 853 + i * 94, 492, 83, 127, i + 50)
    p.rect(735, 488, 91, 119, GROUND, GREEN)
    for x in [748, 770, 792, 811]:
        p.path(f"M{x} 503V585", GOLD if x == 770 else MID, 4)
    p.path("M827 629H1237M886 636V673M1198 636V673", MID, 7)
    p.path("M803 609Q750 701 913 729T1035 805", GROUND, 10)
    p.path("M804 609Q753 699 915 726T1038 803", MID, 3)

    for x, y, w, h, colour in [(111, 253, 202, 369, MID),
                              (330, 236, 230, 386, GOLD),
                              (578, 279, 106, 343, MID)]:
        p.rect(x, y, w, h, DEEP, colour, stroke_width=2)
        p.rect(x + 15, y + 34, w - 30, 48, INK, MID)
        p.path(f"M{x + 19} {y + 45}h{w - 38}", GREEN, 3)
        p.rect(x + 14, y + 106, w - 28, h - 123, INK, MID)
        p.rect(x + w - 33, y + 159, 7, 28, colour)
    p.path("M108 227Q365 177 671 243M211 226V249M443 214V234", MID, 3)
    p.path("M168 651H672", GOLD, 4)
    panel(p, 124, 700, 119, 109, 97)
    panel(p, 256, 739, 114, 92, 98)
    p.polygon([(1269, 730), (1456, 730), (1485, 760), (1485, 840),
               (1269, 840)], INK, MID, stroke_width=2)
    for x in range(1290, 1460, 27):
        p.path(f"M{x} 746V825", MID, 3)
    p.path("M1283 751H1467", GOLD, 4)
    worker(p, 469, 593, .79, DIM)
    worker(p, 637, 607, .72, MID)
    worker(p, 1098, 590, .83, DEEP)
    p.polygon([(419, 723), (646, 723), (717, 768), (457, 768)], MID, GREEN, stroke_width=2)
    p.path("M465 770V855M690 770V855M480 844H676", MID, 8)
    for x, y in [(481, 736), (558, 748), (619, 735)]:
        p.rect(x, y - 15, 15, 20, DEEP, GOLD, stroke_width=2)
        p.path(f"M{x + 15} {y - 12}h7v12h-7", GOLD, 2)
    p.path("M563 206V566", MID, 2)
    p.polygon([(528, 566), (594, 566), (615, 586), (507, 586)], DEEP, GOLD)
    p.path("M514 589H608", LIME, 4)
    return p.finish("INDEPENDENT REPAIR BERTH", "Scarce supplies, shared work, and room outside company control")


def character_portrait(identity):
    """Draw one authored character study; every appearance remains provisional."""
    profiles = {
        "pell": ("DORIAN PELL", [(410, 420), (430, 350), (605, 320), (748, 360), (805, 448), (797, 703), (752, 830), (660, 900), (502, 873), (417, 762), (385, 584)]),
        "halloran": ("HALLORAN", [(430, 435), (486, 338), (650, 344), (754, 430), (771, 575), (738, 764), (624, 885), (506, 831), (422, 690)]),
        "okono": ("OKONO", [(399, 458), (444, 350), (602, 319), (753, 373), (816, 500), (802, 712), (752, 829), (650, 885), (489, 851), (405, 740), (375, 590)]),
        "calloway": ("CALLOWAY", [(428, 440), (485, 340), (675, 344), (764, 447), (768, 659), (720, 802), (622, 879), (509, 835), (448, 690)]),
    }
    title, face = profiles[identity]
    p = Plate(title, f"Proposed CRT portrait of {title.title()}. Appearance and clothing are not established biography, ancestry, rank, or uniform specifications.", 1200, 1400)
    for x in range(110, 1150, 80):
        p.path(f"M{x} 180V1270", DEEP)
    for y in range(180, 1280, 80):
        p.path(f"M90 {y}H1110", DEEP)
    p.path("M180 430V200H410M790 200H1020V430M180 1010V1260H360M840 1260H1020V1010", MID, 3)
    for y in range(480, 951, 34):
        p.path(f"M143 {y}h{22 if y % 2 else 12}M1045 {y}h12", DIM, 2)

    shoulders = [(120, 1288), (154, 1126), (271, 1010), (463, 938), (512, 887), (698, 887), (755, 942), (960, 1021), (1055, 1136), (1080, 1288)]
    p.polygon(shoulders, DEEP, GREEN, stroke_width=3)
    p.polygon([(154, 1126), (302, 1044), (394, 1110), (380, 1288), (120, 1288)], INK, MID)
    p.polygon([(840, 1041), (960, 1021), (1055, 1136), (1080, 1288), (881, 1288)], INK, MID)
    p.polygon([(512, 829), (702, 829), (730, 978), (626, 1062), (480, 960)], "#527d60", MID)
    p.polygon([(512, 835), (565, 864), (601, 1010), (481, 960)], "#347951", MID)
    ear_left, ear_right = {
        "pell": (400, 798),
        "halloran": (430, 765),
        "okono": (392, 806),
        "calloway": (439, 767),
    }[identity]
    for x, y, flip in [(ear_left, 566, 1), (ear_right, 566, -1)]:
        p.group(f"translate({x} {y}) scale({flip} 1)")
        p.polygon([(0, 0), (-31, -10), (-44, 25), (-34, 107), (0, 132), (22, 88)], "#527d60", MID, stroke_width=2)
        p.path("M-12 20L-27 38L-18 84L0 98", DEEP, 5)
        p.end_group()
    p.polygon(face, "#73977f", GREEN, stroke_width=3)
    points = " ".join(f"{x},{y}" for x, y in face)
    p.parts.append(f'<defs><clipPath id="face"><polygon points="{points}"/></clipPath></defs><g clip-path="url(#face)">')
    p.polygon([(350, 320), (561, 320), (498, 494), (467, 675), (513, 790), (602, 900), (390, 913)], "#347951", "none")
    p.polygon([(541, 380), (714, 377), (763, 485), (636, 534), (514, 487)], "#8fb69b", "none")
    p.polygon([(660, 621), (782, 598), (745, 740), (664, 778), (624, 720)], "#8fb69b", "none")
    p.polygon([(470, 641), (536, 652), (563, 740), (512, 792), (446, 736)], "#527d60", "none")
    p.polygon([(605, 823), (711, 800), (686, 856), (635, 879), (542, 840)], "#8fb69b", "none")
    p.path("M465 608L541 623M663 624L746 600", "#527d60", 6)
    p.polygon([(596, 546), (622, 548), (649, 716), (622, 743), (570, 719)], "#527d60", "none")
    p.path("M614 571L600 690L620 710", "#b7dfc3", 6)
    p.path("M574 715L596 727M626 729L646 715", DEEP, 6)
    p.path("M601 751V772", DEEP, 4)
    if identity == "pell":
        p.polygon([(370, 453), (425, 333), (583, 309), (748, 347), (820, 461), (770, 450), (732, 401), (510, 390), (430, 464)], DEEP, MID)
        p.path("M431 439L479 395M735 411L777 470", DIM, 16)
        p.path("M468 542L535 537M666 534L738 542", INK, 17)
        rng = random.Random(81)
        for _ in range(160):
            x, y = rng.randint(446, 764), rng.randint(753, 886)
            p.path(f"M{x} {y}l3 5", DEEP, 2)
        p.path("M546 790Q612 802 694 784", INK, 7)
        p.path("M555 813L664 820", "#b7dfc3", 4)
        p.path("M538 465H666M548 481H648", "#527d60", 3)
    elif identity == "halloran":
        p.polygon([(407, 532), (419, 395), (475, 303), (653, 294), (787, 397), (753, 494), (674, 386), (553, 460), (486, 431)], INK, MID)
        p.path("M460 389L540 330L666 346M500 400L603 355M675 351L746 424", MID, 9)
        p.path("M463 528L536 511M658 539L727 547", DEEP, 13)
        p.path("M540 794L599 806L674 790", INK, 6)
        p.path("M548 813L625 827", "#b7dfc3", 4)
        p.path("M520 657L487 717M722 649L743 680", DIM, 4)
    elif identity == "okono":
        p.polygon([(368, 518), (403, 386), (457, 321), (610, 293), (758, 342), (825, 467), (797, 538), (746, 442), (456, 439), (405, 530)], DEEP, MID)
        rng = random.Random(94)
        for _ in range(100):
            x, y = rng.randint(408, 794), rng.randint(317, 441)
            p.path(f"M{x} {y}l8 5", DIM if x % 4 == 0 else MID, 3)
        p.path("M467 529Q508 515 550 535M658 535Q710 518 749 538", DEEP, 12)
        p.path("M541 790Q608 813 686 791", INK, 6)
        p.path("M557 817Q615 828 663 816", DIM, 4)
        p.path("M467 642L499 659M708 659L748 637", "#b7dfc3", 3)
    else:
        p.polygon([(407, 515), (426, 362), (492, 295), (675, 302), (771, 383), (792, 512), (746, 462), (715, 390), (526, 398), (462, 489)], INK, MID)
        p.path("M447 403Q542 337 663 339M457 423Q548 358 671 357M685 337L739 409", DIM, 6)
        p.path("M452 456L471 405M730 407L755 463", DIM, 12)
        p.path("M469 537Q511 521 552 540M656 541Q701 526 738 541", DEEP, 10)
        p.path("M550 785Q620 804 689 783", INK, 5)
        p.path("M563 811L668 812", "#b7dfc3", 4)
        p.path("M523 676L535 759M704 678L694 754", "#527d60", 4)
    for x in [508, 696]:
        p.path(f"M{x - 35} 574Q{x} 555 {x + 32} 575Q{x} 586 {x - 35} 574", DEEP, 3, fill="#b7dfc3")
        p.element("circle", cx=x + (5 if identity == "halloran" else 0), cy=574, r=10, fill=DEEP)
        p.rect(x + 1, 568, 4, 4, BRIGHT)
    p.end_group()
    if identity == "okono":
        for x in [508, 696]:
            p.element("ellipse", cx=x, cy=578, rx=70, ry=45, fill="none", stroke=DIM, stroke_width=6)
        p.path("M578 569Q602 557 626 569M438 562L397 547M766 562L796 547", DIM, 5)
    if identity == "calloway":
        p.polygon([(463, 937), (538, 977), (626, 1014), (700, 979), (755, 941), (768, 1048), (650, 1120), (480, 1048)], INK, DIM, stroke_width=3)
        p.path("M625 1022V1283M490 1024L584 1064M670 1071L747 1025", MID, 5)
        p.rect(611, 1083, 28, 9, GOLD)
        p.path("M302 1087V1279M897 1084V1279", MID, 4)
    else:
        p.polygon([(459, 930), (520, 970), (599, 1051), (512, 1115), (401, 975)], INK, MID, stroke_width=3)
        p.polygon([(701, 935), (765, 955), (815, 994), (716, 1090), (630, 1054)], INK, MID, stroke_width=3)
        p.path("M606 1067V1289M462 1132L402 1277M770 1120L818 1279", MID, 5)
        p.rect(257, 1150, 134, 88, DEEP, MID, stroke_width=3)
        p.path("M269 1164H377M271 1223H373", DIM, 3)
        if identity == "halloran":
            p.polygon([(835, 1150), (944, 1141), (956, 1220), (844, 1230)], INK, DIM, stroke_width=3)
            for y in range(1157, 1215, 12):
                p.path(f"M833 {y}h12M940 {y - 5}h12", GOLD, 2)
        elif identity == "okono":
            p.rect(833, 1128, 90, 132, INK, MID)
            for x in [850, 874, 899]:
                p.path(f"M{x} 1142V1204", DIM, 6)
        else:
            p.path("M844 1143H946V1234H841M857 1156H932", DIM, 3)
    return p.finish(title, "CHARACTER STUDY / APPEARANCE PROPOSAL", "PORTRAIT PROPOSAL / NOT A RECORD")


def timeline_diagram():
    """Draw labelled fixed anchors, undated developments, and proposed years."""
    p = Plate("Timeline before Y0", "Chronology diagram. Filled circles mark fixed Y-4 and Y0 anchors. A boxed interval contains undated aftermath developments. Open diamonds and dashed lines mark proposed years Y-40, Y-24, Y-16, Y-10, Y-3, Y-2, and Y-1. Spacing does not represent elapsed time.", 1600, 1600)
    p.text(100, 181, "ESTABLISHED RECORD", 24, BRIGHT)
    p.text(858, 181, "PROPOSED WORKING DATES", 24, LIME)
    p.path("M797 150V1458", MID)
    p.path("M157 237V1374", GREEN, 4)
    p.path("M904 252V1327", GOLD, 3, stroke_dasharray="12 10")
    p.rect(120, 234, 74, 104, INK, GREEN, stroke_width=3)
    p.text(222, 258, "BEFORE Y-4 / DATES OPEN", 22, BRIGHT)
    for i, text in enumerate(["EWI expands; Kestrel competes.", "The Shelter is a working base.", "The Roost belongs to Kestrel."]):
        p.text(222, 291 + i * 28, text, 19)
    p.element("circle", cx=157, cy=451, r=16, fill=GREEN)
    p.text(222, 440, "Y-4 / FIXED", 24, BRIGHT)
    for i, text in enumerate(["The Shelter is destroyed.", "Pell disobeys to rescue people.", "He does not know it is deliberate."]):
        p.text(222, 478 + i * 29, text, 19)
    p.rect(111, 630, 619, 474, INK, GREEN, stroke_width=2)
    p.text(140, 671, "AFTERMATH / DATES AND ORDER OPEN", 22, BRIGHT)
    for i, text in enumerate(["EWI blames Kestrel's reactor.", "Forced sale and worker contracts.", "Flight and Kite community formation.", "Calloway is promoted; Datum rebuilt.", "The Roost becomes the Kites' base."]):
        p.text(140, 727 + i * 54, text, 21)
    p.text(140, 1042, "These developments can overlap.", 18)
    p.text(140, 1071, "The box is not a scene sequence.", 18)
    p.element("circle", cx=157, cy=1270, r=16, fill=GREEN)
    p.text(222, 1253, "START OF Y0 / FIXED", 24, BRIGHT)
    for i, text in enumerate(["Four years after the destruction.", "Datum is the largest ring station", "and a direct Earth relay.", "The Kites are the largest", "independent community at Saturn."]):
        p.text(222, 1291 + i * 29, text, 19)
    milestones = [
        ("Y-40", ["EWI begins large-scale", "space mining."]),
        ("Y-24", ["Major EWI mining presence", "at Saturn."]),
        ("Y-16", ["Substantial Kestrel operations", "at Saturn."]),
        ("Y-10", ["The Shelter becomes", "Kestrel's main base."]),
        ("Y-3", ["Formal transfer of Kestrel", "to EWI completed."]),
        ("Y-2", ["First rebuilt sections", "of Datum occupied."]),
        ("Y-1", ["Datum's direct Earth relay", "enters service."]),
    ]
    for i, (year, lines) in enumerate(milestones):
        y = 271 + i * 170
        p.polygon([(904, y - 13), (917, y), (904, y + 13), (891, y)], GROUND, GOLD, stroke_width=3)
        p.text(947, y + 5, year + " / PROPOSED", 23, LIME)
        for j, line in enumerate(lines):
            p.text(947, y + 43 + j * 28, line, 21)
    p.text(100, 1463, "Filled circle: fixed date    Box: undated interval    Open diamond: proposed date", 18)
    return p.finish("BEFORE Y0 / TIMELINE", "World history and the fall of Kestrel / order only, not elapsed-time spacing", "REFERENCE DIAGRAM / DATES LABELLED")


def render_all():
    """Return every generated filename and its deterministic SVG content."""
    return {
        "industrial-saturn.svg": industrial_saturn(),
        "company-habitat.svg": company_habitat(),
        "salvage-habitat.svg": salvage_habitat(),
        **{f"portrait-{name}.svg": character_portrait(name) for name in ["pell", "halloran", "okono", "calloway"]},
        "timeline.svg": timeline_diagram(),
    }


def main():
    """Write the plates, or fail if the checked-in plates need regeneration."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check files without writing")
    args = parser.parse_args()
    stale = []
    for name, content in render_all().items():
        path = OUTPUT / name
        if args.check:
            if not path.is_file() or path.read_bytes() != content.encode("utf-8"):
                stale.append(str(path.relative_to(OUTPUT.parent)))
        else:
            OUTPUT.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content.encode("utf-8"))
            print(f"Wrote {path.relative_to(OUTPUT.parent)}")
    if stale:
        parser.exit(1, "Regenerate lore artwork: " + ", ".join(stale) + "\n")
    if args.check:
        print("All CRT illustrations are current.")


if __name__ == "__main__":
    main()
