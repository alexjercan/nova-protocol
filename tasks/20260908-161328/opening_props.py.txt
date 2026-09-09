"""Incidental props shared by this opening's page draft and comparison study."""

from nova_illustration.colors import INK, JADE, JONAH, MATERIALS
from nova_illustration.svg import ellipse, group, path


def mug_hand(x, y, scale):
    """Draw the borrowed mug and gripping hand; this is not a series-wide asset."""
    art = path("M-245 119L-120 38L-60 35L-8 91L-129 186H-245Z", JONAH['coat'], width=3)
    art += path("M-237 157L-108 73L-61 91L-129 186H-245Z", JONAH['coat_shadow'], "none")
    art += path("M-113 38L-78 26L-35 74L-62 97Z", JONAH['shirt'], width=2)
    art += path("M-71 33L-48 11Q-41 5-25 14L7 43L8 65L-14 91L-44 83L-68 67Z", JONAH['skin'], width=2.2)
    art += path("M-7 11Q-67-6-71 41Q-72 70-22 77", stroke=INK, width=17)
    art += path("M-7 11Q-67-6-71 41Q-72 70-22 77", stroke=MATERIALS['ceramic_edge'], width=11)
    art += path("M-25-17L98-17L91 107Q41 128-16 107Z", MATERIALS['ceramic'], width=2.6)
    art += path("M64-16L98-17L91 107Q75 115 57 116Z", MATERIALS['ceramic_shadow'], "none")
    art += path("M-22 33Q38 50 95 32L94 46Q35 66-20 48Z", JADE, "none")
    art += ellipse(36, -17, 61, 16, MATERIALS['ceramic_highlight'], INK, 2.5)
    art += ellipse(36, -15, 49, 9, MATERIALS['dark_brown'])
    art += path("M-57 29Q-46 20-29 34L-12 53Q-10 64-18 65L-38 47L-48 47Q-30 53-19 70Q-17 77-24 80L-46 65L-52 67L-32 82Q-30 91-40 92L-63 77L-71 54Z", JONAH['light'], width=2)
    art += path("M-51 36L-38 46M-54 55L-46 63", stroke=JONAH['lines'], width=1.3)
    art += path("M-19 85L-14 104", stroke=MATERIALS['ceramic_highlight'], width=2)
    return group(art, f"translate({x} {y}) scale({scale})")
