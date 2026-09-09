"""Reusable provisional station scenery, not orbital or interior engineering."""

from .colors import STATION, SATURN, SKY, INK, FREIGHT
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


def aquila(x, y, scale):
    """Adapt the existing two residential rings and broad freight-spine concept.

Berths and handling frames are drawing proposals. No ship is baked into this
location, and its proportions do not establish dimensions or an orbit.
"""
    art = path('M18-45H295V-128M19 29H351V-61',stroke=STATION['shadow'],width=12)
    for px,py in ((251,-175),(295,-67)):
        art += rect(px,py,154,80,STATION['solar'],STATION['frame_light'],3)
        for dx in range(10,155,18):
            art += path(f'M{px+dx} {py}V{py+80}',stroke=STATION['solar_line'],width=1)
        art += path(f'M{px} {py+40}H{px+154}',stroke=STATION['solar_line'],width=2)
    for cy,rx,ry,thickness in ((-116,230,79,40),(35,144,48,30)):
        art += ellipse(0,cy-17,rx,ry,'none',INK,thickness+6)
        art += ellipse(0,cy-17,rx,ry,'none',STATION['ring_shadow'],thickness)
        art += path(f'M{-rx} {cy}H{rx}M0 {cy-ry}V{cy+ry}M{-rx*.7} {cy-ry*.7}L{rx*.7} {cy+ry*.7}M{-rx*.7} {cy+ry*.7}L{rx*.7} {cy-ry*.7}',stroke=STATION['frame'],width=9)
        art += ellipse(0,cy,rx,ry,'none',INK,thickness+4)
        art += ellipse(0,cy,rx,ry,'none',STATION['ring'],thickness)
        art += ellipse(0,cy,rx,ry,'none',STATION['ring_shadow'],thickness*.4)
        art += ellipse(0,cy,rx,ry,'none',STATION['accent'],3,stroke_dasharray='5 12')
    art += path('M-29-181H28V143H-29Z',STATION['paint'],width=3)
    art += path('M28-181L42-169V133L28 143Z',STATION['shadow'],width=2)
    art += path('M-13-168V127M15-168V127',stroke=STATION['frame'],width=3)
    art += path('M-29-72H28M-29 11H28M-29 91H28',stroke=STATION['shadow'],width=10)
    art += path('M-25 77L-173 142M30 77L178 142',stroke=STATION['frame'],width=14)
    art += path('M-450 149L-425 124H425L452 149Z',STATION['paint'],width=3)
    art += rect(-450,149,902,43,STATION['frame'],INK,3)
    for px in range(-434,420,62):
        art += path(f'M{px} 157L{px+31} 183L{px+62} 157',stroke=STATION['shadow'],width=5)
    for px in (-398,-292,-188,91,201,311):
        art += path(f'M{px} 94L{px+16} 78H{px+100}L{px+84} 94Z',STATION['paint'],width=2)
        art += rect(px,94,84,48,STATION['ring_shadow'],INK,2)
        art += path(f'M{px+10} 101V134M{px+28} 101V134M{px+46} 101V134M{px+64} 101V134',stroke=STATION['frame_light'],width=2)
    for px in (-325,-108,207,389):
        art += path(f'M{px} 191V277M{px} 192L{px+44} 250',stroke=STATION['shadow'],width=13)
        art += path(f'M{px-2} 195V271',stroke=STATION['frame_light'],width=4)
        art += rect(px-18,264,36,19,STATION['paint'],INK,2)
    for px,side in ((-431,1),(425,-1)):
        art += path(f'M{px} 146V31H{px+side*128}V73',stroke=STATION['shadow'],width=12)
        art += path(f'M{px} 145V32H{px+side*128}',stroke=STATION['frame_light'],width=4)
    art += text(-71,177,'AQUILA',17,STATION['paint'],font_weight='bold',letter_spacing=3)
    return group(art,f'translate({x} {y}) rotate(-12) scale({scale})',data_place='aquila')


def transfer_hall(w, h):
    """Draw Aquila's pressure-side wall; windows, handholds, and lock belong to staging.

There is no floor plane or gravity cue. All placement remains an unscaled
local scene proposal, not a station layout or pressure-system schematic.
"""
    art = rect(0,0,w,h,FREIGHT['wall'])
    art += path(f'M0 0H{w}V{h*.2}H0Z',FREIGHT['wall_light'],'none')
    for px in range(40,int(w),260):
        art += path(f'M{px} 0V{h}',stroke=FREIGHT['panel'],width=4)
        art += rect(px+12,h*.77,194,50,FREIGHT['panel'],FREIGHT['frame'],2,5)
        art += path(f'M{px+27} {h*.77+13}H{px+184}',stroke=FREIGHT['rail'],width=2)
    art += path(f'M0 {h*.69}H{w}',stroke=FREIGHT['frame'],width=15)
    art += path(f'M0 {h*.69-3}H{w}',stroke=FREIGHT['rail'],width=5)
    return group(art,data_place='aquila-transfer-hall',data_gravity='freefall')


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
