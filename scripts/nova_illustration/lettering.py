"""Shared comic lettering from authored lines, independent of world illustration."""

import math
from .colors import CREAM, INK, FOLIO
from .svg import group, path, text


def labelled_speech(x, y, w, speaker, lines, tip, side):
    """Letter a labelled balloon with an authored top, bottom, left, or right tail."""
    h = 42+27*len(lines)
    tx, ty = tip
    if side in ('top','bottom'):
        ax, ay = max(x+25,min(x+w-30,tx)), y if side=='top' else y+h
    elif side in ('left','right'):
        ax, ay = x if side=='left' else x+w, y+h*.65
    else:
        raise ValueError(f'Unknown balloon tail side: {side}')
    length = math.hypot(tx-ax,ty-ay)
    if length > 65:
        tx, ty = ax+(tx-ax)*65/length, ay+(ty-ay)*65/length
    shape = f'M{x+24} {y}'
    if side == 'top':
        shape += f'H{ax-5}L{tx} {ty}L{ax+18} {y}'
    shape += f'H{x+w-24}Q{x+w} {y} {x+w} {y+24}'
    if side == 'right':
        shape += f'V{ay-12}L{tx} {ty}L{x+w} {ay+10}'
    shape += f'V{y+h-24}Q{x+w} {y+h} {x+w-24} {y+h}'
    if side == 'bottom':
        shape += f'H{ax+18}L{tx} {ty}L{ax-5} {y+h}'
    shape += f'H{x+24}Q{x} {y+h} {x} {y+h-24}'
    if side == 'left':
        shape += f'V{ay+10}L{tx} {ty}L{x} {ay-12}'
    shape += f'V{y+24}Q{x} {y} {x+24} {y}Z'
    art = path(shape,CREAM,width=2.5)
    art += text(x+18,y+25,speaker.upper(),13,FOLIO['title'],font_weight='bold',letter_spacing=1.3)
    art += ''.join(text(x+18,y+52+i*27,line,22,INK) for i,line in enumerate(lines))
    return group(art,class_='dialogue',data_x=x,data_y=y,data_width=w,data_height=h)


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
