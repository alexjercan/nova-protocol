"""Shared comic lettering from authored lines, independent of world illustration."""

import math
from .colors import CREAM, INK
from .svg import group, path, text


def speech(x, y, w, lines, tip, side):
    """Place source-lettered balloons with bounded tails and inspectable text bounds."""
    h = 32 + 34*len(lines)
    tx, ty = tip
    if side == "right":
        ax, ay = x+w, y+h*.7
    elif side == "bottom":
        ax, ay = max(x+40, min(x+w-45, tx)), y+h
    else:
        raise ValueError(f"Unknown balloon tail side: {side}")
    distance = math.hypot(tx-ax, ty-ay)
    if distance > 90:
        tx, ty = ax+(tx-ax)*90/distance, ay+(ty-ay)*90/distance
    if side == "right":
        shape = f"M{x+30} {y}H{x+w-30}Q{x+w} {y} {x+w} {y+30}V{ay-14}L{tx} {ty}L{x+w} {ay+10}V{y+h-30}Q{x+w} {y+h} {x+w-30} {y+h}H{x+30}Q{x} {y+h} {x} {y+h-30}V{y+30}Q{x} {y} {x+30} {y}Z"
    else:
        shape = f"M{x+30} {y}H{x+w-30}Q{x+w} {y} {x+w} {y+30}V{y+h-30}Q{x+w} {y+h} {x+w-30} {y+h}H{ax+16}L{tx} {ty}L{ax-9} {y+h}H{x+30}Q{x} {y+h} {x} {y+h-30}V{y+30}Q{x} {y} {x+30} {y}Z"
    art = path(shape, CREAM, width=2.5)
    art += "".join(text(x+24, y+43+i*34, line, 27, INK) for i, line in enumerate(lines))
    return group(art, class_="speech", data_x=x, data_y=y, data_width=w, data_height=h)
