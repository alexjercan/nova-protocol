"""Reusable provisional Baikal and Saturn scenery, not an orbital model."""

from .colors import STATION, SATURN, SKY, INK
from .svg import ellipse, group, path, rect, text


def stars(w, h):
    """Keep the sky sparse and deterministic; this is not gameplay randomness."""
    return "".join(ellipse((i*191+47) % w, (i*113+31) % h, 0.7+i%3*0.3, 0.7+i%3*0.3, SKY['star'], opacity=0.25+i%4*0.12) for i in range(65))


def saturn(x, y, scale):
    """Compose Saturn as scenery, not as a map of Baikal's orbit."""
    art = ellipse(0, 0, 610, 128, "none", SATURN['ring_back'], 47)
    art += ellipse(0, 0, 652, 138, "none", SATURN['ring_back'], 8)
    art += ellipse(0, 0, 310, 310, "url(#planet)", SATURN['limb'], 1.5)
    bands = ""
    for y0, thickness, color in [(-225, 13, SATURN['band_light']), (-169, 25, SATURN['band_dark']), (-110, 7, SATURN['band_light']), (-65, 32, SATURN['band_mid']), (12, 20, SATURN['band_light']), (85, 13, SATURN['band_dark']), (138, 27, SATURN['band_mid']), (225, 15, SATURN['band_dark'])]:
        bands += path(f"M-340 {y0}Q0 {y0+87} 340 {y0}", stroke=color, width=thickness, opacity=0.4)
    art += group(bands, clip_path="url(#globe)")
    art += path("M-610 0A610 128 0 0 0 610 0", stroke=SATURN['ring'], width=47)
    art += path("M-590 0A590 122 0 0 0 590 0", stroke=SATURN['ring_light'], width=5)
    art += path("M-619 0A619 130 0 0 0 619 0", stroke=SATURN['ring_dark'], width=3)
    art += path("M-652 0A652 138 0 0 0 652 0", stroke=SATURN['ring'], width=8)
    return group(art, f"translate({x} {y}) rotate(-18) scale({scale})")


def baikal(x, y, scale):
    """Retain the station study's ring, processing tanks, and solar surfaces."""
    art = path("M-311-5L315-5V33H-311Z", STATION['frame'], width=4)
    art += path("M-297 3H304", stroke=STATION['paint'], width=4)
    art += path("M-256 13L-220 28L-185 13L-150 28L-115 13L-80 28L-45 13L-10 28L25 13L60 28L95 13L130 28L165 13L200 28L235 13L270 28", stroke=STATION['shadow'], width=4)
    for tx in [-113, 10, 133]:
        art += rect(tx-43, -161, 86, 153, STATION['tank'], INK, 3)
        art += rect(tx+8, -158, 34, 147, STATION['tank_shadow'])
        art += ellipse(tx, -161, 43, 15, STATION['paint'], INK, 3)
        art += path(f"M{tx-43}-105H{tx+43}M{tx-43}-38H{tx+43}", stroke=STATION['green'], width=9)
        art += path(f"M{tx-22}-155V-20", stroke=STATION['paint'], width=3)
        art += path(f"M{tx}-178V-195H{tx-53}V-3", stroke=STATION['accent'], width=6)
        art += path(f"M{tx} 34V70", stroke=STATION['frame_light'], width=13)
        art += path(f"M{tx-43} 72L{tx-25} 57H{tx+53}L{tx+35} 72Z", STATION['paint'], width=2)
        art += rect(tx-43, 72, 78, 48, STATION['green'], INK, 2)
        art += rect(tx-30, 82, 45, 7, STATION['accent'])
    art += path("M-276-150V164M-328 6H-224M-312-98L-240 106M-312 108L-240-99", stroke=STATION['frame'], width=7)
    art += ellipse(-272, 8, 64, 175, "none", INK, 32)
    art += ellipse(-277, 2, 64, 175, "none", STATION['ring'], 24)
    art += ellipse(-277, 2, 64, 175, "none", STATION['ring_shadow'], 12)
    art += ellipse(-277, 2, 64, 175, "none", STATION['accent'], 3.5, stroke_dasharray="6 13")
    art += path("M234 1V-229H410M300 20V-119H472", stroke=STATION['frame'], width=8)
    for px, py, pw in [(260, -265, 163), (366, -153, 137)]:
        art += rect(px, py, pw, 78, STATION['solar'], STATION['frame_light'], 2.5)
        for j in range(12, pw, 18):
            art += path(f"M{px+j} {py}V{py+78}", stroke=STATION['solar_line'], width=1)
        art += path(f"M{px} {py+39}H{px+pw}", stroke=STATION['solar_line'], width=1.5)
    art += path("M310 32V164H246", stroke=STATION['frame_light'], width=9)
    art += rect(234, 152, 27, 18, STATION['shadow'], INK, 2)
    art += text(-58, 25, "BAIKAL", 12, STATION['paint'], font_weight="bold", letter_spacing=2)
    return group(art, f"translate({x} {y}) rotate(-10) scale({scale})")
