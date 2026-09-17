#!/usr/bin/env python3
"""Generate the green CRT speaker portraits.

The base game's faces write under `assets/base/portraits/`; their SVG sources
live in `art/portraits/`. A mod that ships faces of its own writes them under
its own asset tree.

Two casts live here. The training range's two voices - the player and Range
Control - are drawn for the game and nowhere else. The season one crew are
drawn FROM THE STORY: every skin, hair, coat and trim colour below is read out
of `nova_illustration.colors`, the same palette `gen-lore-portraits.py` renders
the encyclopedia portrait studies with, so a character who is repainted in the
book is repainted in the comms panel by rerunning this script. What changes
between the two is the medium, not the person: 32 hard pixels inside a green
phosphor frame instead of a 600x720 study.
"""

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from nova_illustration.colors import ELENA, JONAH, LEILA, NADIA, RINA, SAMIR, TOMAS
SOURCE_DIR = ROOT / "art" / "portraits"
BASE_DIR = ROOT / "assets" / "base" / "portraits"

BG = "#030d08"
PANEL = "#06170f"
GRID = "#164c31"
GREEN = "#55d68b"
BRIGHT = "#c0ffd4"
DARK = "#183426"
AMBER = "#c2a23b"
# The channel chip: Range Control's blue, reused for every voice that reaches
# the panel over the radio rather than from a seat in the same room.
RADIO = "#5fb7e0"


def rect(x: int, y: int, w: int, h: int, fill: str) -> str:
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}"/>'


def polygon(points: str, fill: str) -> str:
    return f'<polygon points="{points}" fill="{fill}"/>'


def frame() -> list[str]:
    shapes = [
        rect(0, 0, 32, 32, BG),
        rect(1, 1, 30, 30, GREEN),
        rect(2, 2, 28, 28, PANEL),
        rect(3, 3, 26, 26, BG),
    ]
    for y in range(4, 29, 4):
        shapes.append(rect(3, y, 26, 1, GRID))
    shapes.extend([rect(3, 3, 6, 1, AMBER), rect(23, 28, 6, 1, GREEN)])
    return shapes


def person(
    skin: str,
    shadow: str,
    hair: str,
    uniform: str,
    uniform_shadow: str,
    eyes: str = BRIGHT,
) -> list[str]:
    return [
        rect(6, 25, 20, 4, uniform_shadow),
        rect(8, 22, 16, 6, uniform),
        polygon("8,22 12,20 20,20 24,22 21,25 11,25", uniform),
        rect(13, 19, 6, 4, shadow),
        rect(11, 8, 10, 12, skin),
        rect(10, 10, 2, 7, shadow),
        rect(20, 10, 2, 7, shadow),
        rect(12, 11, 3, 2, DARK),
        rect(17, 11, 3, 2, DARK),
        rect(13, 11, 1, 1, eyes),
        rect(18, 11, 1, 1, eyes),
        rect(15, 13, 2, 4, shadow),
        rect(14, 17, 4, 1, DARK),
        rect(15, 18, 3, 1, skin),
        rect(11, 7, 10, 2, hair),
    ]


# --- the season one crew -----------------------------------------------------
#
# One builder each, all through `crew()`. The 32-pixel grid can hold about
# three facts about a face, so each portrait spends them on what the story
# already uses to tell these people apart at a glance: Rina's orange work coat,
# Samir's glasses, Leila's hair, Tomas's grey, Nadia's and Elena's channel.
# Everything else - skin, shadow, coat, trim - is the book's own colour.

def crew(
    palette,
    hair_shapes: list[str],
    extras: list[str] | None = None,
    radio: bool = False,
) -> list[str]:
    """One speaker: the CRT frame, the shared head, and this face's own marks.

    `hair_shapes` replaces the default cap so a silhouette reads before a
    feature does, and `extras` draws last so glasses and collars sit on top of
    the face rather than under it. `radio` marks a voice that is not in the
    room, the way Range Control's blue chip does.
    """
    shapes = frame() + person(
        palette["skin"],
        palette["shadow"],
        palette["hair"],
        palette["coat"],
        palette["coat_shadow"],
        eyes=palette["eye"],
    )
    shapes += hair_shapes
    shapes += [
        rect(13, 20, 6, 1, palette["shirt"]),
        rect(23, 3, 6, 1, RADIO if radio else GREEN),
    ]
    shapes += extras or []
    return shapes


def jonah() -> list[str]:
    """The captain: a short crop, three days of stubble, and a collar bar."""
    return crew(
        JONAH,
        [
            rect(11, 6, 10, 3, JONAH["hair"]),
            rect(10, 7, 1, 3, JONAH["hair"]),
            rect(21, 7, 1, 3, JONAH["hair"]),
        ],
        [
            rect(11, 16, 3, 3, JONAH["stubble"]),
            rect(18, 16, 3, 3, JONAH["stubble"]),
            rect(13, 19, 6, 1, JONAH["stubble"]),
            rect(14, 17, 4, 1, DARK),
            rect(15, 18, 3, 1, JONAH["skin"]),
            rect(10, 22, 4, 1, JONAH["coat_light"]),
            rect(18, 22, 4, 1, JONAH["coat_light"]),
        ],
    )


def leila() -> list[str]:
    """The chief engineer: the volume of hair the study gives her, and pockets."""
    return crew(
        LEILA,
        [
            rect(9, 4, 14, 5, LEILA["hair"]),
            rect(8, 7, 2, 9, LEILA["hair"]),
            rect(22, 7, 2, 9, LEILA["hair"]),
            rect(10, 6, 5, 1, LEILA["hair_line"]),
        ],
        [
            rect(9, 23, 4, 3, LEILA["coat_light"]),
            rect(19, 23, 4, 3, LEILA["coat_light"]),
            rect(10, 24, 2, 1, LEILA["coat_shadow"]),
            rect(20, 24, 2, 1, LEILA["coat_shadow"]),
        ],
    )


def tomas() -> list[str]:
    """The pilot: hair going grey at the temples, and a flight collar zip."""
    return crew(
        TOMAS,
        [
            rect(11, 6, 10, 3, TOMAS["hair"]),
            rect(10, 7, 1, 4, TOMAS["hair_line"]),
            rect(21, 7, 1, 4, TOMAS["hair_line"]),
            rect(11, 6, 4, 1, TOMAS["hair_line"]),
        ],
        [
            rect(15, 21, 2, 7, TOMAS["coat_light"]),
            rect(9, 24, 3, 1, TOMAS["coat_light"]),
            rect(20, 24, 3, 1, TOMAS["coat_light"]),
        ],
    )


def rina() -> list[str]:
    """Work operations: braided hair, and the one orange coat in the chapter."""
    return crew(
        RINA,
        [
            rect(10, 5, 12, 4, RINA["hair"]),
            rect(12, 5, 1, 4, RINA["hair_line"]),
            rect(15, 5, 1, 4, RINA["hair_line"]),
            rect(18, 5, 1, 4, RINA["hair_line"]),
            rect(9, 8, 1, 4, RINA["hair"]),
            rect(22, 8, 1, 4, RINA["hair"]),
        ],
        [
            rect(8, 23, 5, 4, RINA["shirt"]),
            rect(19, 23, 5, 4, RINA["shirt"]),
            rect(15, 21, 2, 7, RINA["coat_light"]),
            rect(9, 24, 3, 1, RINA["coat_shadow"]),
            rect(20, 24, 3, 1, RINA["coat_shadow"]),
        ],
    )


def samir() -> list[str]:
    """Systems and first aid: the glasses first, the armband second."""
    return crew(
        SAMIR,
        [
            rect(10, 5, 12, 4, SAMIR["hair"]),
            rect(9, 7, 1, 4, SAMIR["hair"]),
            rect(22, 7, 1, 4, SAMIR["hair"]),
            rect(11, 5, 2, 1, SAMIR["hair_line"]),
            rect(16, 5, 2, 1, SAMIR["hair_line"]),
        ],
        [
            rect(11, 10, 5, 4, SAMIR["glasses"]),
            rect(16, 11, 1, 1, SAMIR["glasses"]),
            rect(17, 10, 5, 4, SAMIR["glasses"]),
            rect(12, 11, 3, 2, DARK),
            rect(18, 11, 3, 2, DARK),
            rect(13, 11, 1, 1, SAMIR["eye"]),
            rect(19, 11, 1, 1, SAMIR["eye"]),
            rect(9, 23, 4, 1, BRIGHT),
            rect(10, 24, 2, 2, BRIGHT),
        ],
    )


def nadia() -> list[str]:
    """Gantry's mate, on the radio: shoulder-length hair and a gold trim."""
    return crew(
        NADIA,
        [
            rect(10, 5, 12, 4, NADIA["hair"]),
            rect(9, 8, 2, 8, NADIA["hair"]),
            rect(21, 8, 2, 8, NADIA["hair"]),
            rect(11, 5, 6, 1, NADIA["hair_line"]),
        ],
        [
            rect(8, 22, 16, 1, NADIA["trim"]),
            rect(15, 23, 2, 5, NADIA["trim"]),
            rect(22, 9, 2, 6, GREEN),
            rect(23, 15, 1, 3, RADIO),
        ],
        radio=True,
    )


def elena() -> list[str]:
    """Baikal's dispatcher, on the radio: grey in the hair, and a jade collar."""
    return crew(
        ELENA,
        [
            rect(10, 5, 12, 4, ELENA["hair"]),
            rect(9, 6, 1, 5, ELENA["hair"]),
            rect(22, 6, 1, 5, ELENA["hair"]),
            rect(11, 5, 4, 1, ELENA["hair_light"]),
            rect(17, 6, 4, 1, ELENA["hair_light"]),
        ],
        [
            rect(11, 21, 4, 3, ELENA["coat_light"]),
            rect(17, 21, 4, 3, ELENA["coat_light"]),
            rect(13, 23, 6, 2, ELENA["shirt"]),
            rect(8, 26, 16, 1, RADIO),
        ],
        radio=True,
    )


def player() -> list[str]:
    s = frame()
    s += [
        rect(7, 25, 18, 4, "#245c3d"),
        polygon("8,24 11,20 21,20 24,24 22,28 10,28", "#347951"),
        polygon("10,8 13,5 19,5 22,8 23,18 20,22 12,22 9,18", "#8fb69b"),
        polygon("11,9 13,7 19,7 21,9 21,16 19,19 13,19 11,16", "#0b2b1c"),
        rect(12, 10, 8, 5, "#123f2d"),
        polygon("12,10 20,10 18,12 13,12", GREEN),
        rect(13, 11, 4, 1, BRIGHT),
        rect(9, 11, 2, 6, GREEN),
        rect(21, 11, 2, 6, GREEN),
        rect(14, 22, 4, 5, DARK),
    ]
    return s


def range_control() -> list[str]:
    s = frame() + person("#8a7f55", "#5b5a3e", "#2b2f2a", "#2f6d8a", "#224d63")
    s += [
        rect(10, 7, 12, 2, "#2b2f2a"),
        rect(9, 8, 3, 5, "#2b2f2a"),
        rect(20, 8, 3, 5, "#2b2f2a"),
        rect(8, 9, 2, 8, GREEN),
        rect(22, 9, 2, 8, GREEN),
        rect(19, 16, 5, 1, GREEN),
        rect(23, 16, 1, 3, BRIGHT),
        rect(13, 23, 6, 1, BRIGHT),
        rect(10, 26, 12, 1, "#5fb7e0"),
        rect(4, 4, 3, 1, "#5fb7e0"),
    ]
    return s


PORTRAITS = {
    "player": (BASE_DIR, player),
    "range-control": (BASE_DIR, range_control),
    "jonah": (BASE_DIR, jonah),
    "leila": (BASE_DIR, leila),
    "tomas": (BASE_DIR, tomas),
    "rina": (BASE_DIR, rina),
    "samir": (BASE_DIR, samir),
    "nadia": (BASE_DIR, nadia),
    "elena": (BASE_DIR, elena),
}


def svg(shapes: list[str]) -> str:
    body = "\n  ".join(shapes)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" '
        'viewBox="0 0 32 32" shape-rendering="crispEdges">\n  '
        + body
        + "\n</svg>\n"
    )


def main() -> None:
    SOURCE_DIR.mkdir(parents=True, exist_ok=True)
    for name, (asset_dir, build) in PORTRAITS.items():
        asset_dir.mkdir(parents=True, exist_ok=True)
        source = SOURCE_DIR / f"{name}.svg"
        output = asset_dir / f"{name}.png"
        source.write_text(svg(build()), encoding="utf-8")
        subprocess.run(
            ["magick", "-background", "none", str(source), str(output)],
            check=True,
        )
        print(output.relative_to(ROOT))


if __name__ == "__main__":
    main()
