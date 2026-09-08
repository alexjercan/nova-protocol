"""Shared SVG primitives in drawing coordinates, never game-space quantities."""

from html import escape

from .colors import INK


def tag(name, body="", **attrs):
    """Build SVG with escaped, explicitly authored attributes."""
    attributes = " ".join(f'{k.rstrip("_").replace("_", "-")}="{escape(str(v), quote=True)}"' for k, v in attrs.items())
    return f"<{name} {attributes}>{body}</{name}>"


def path(d, fill="none", stroke=INK, width=3, **attrs):
    """Draw linework in illustration coordinates, not meters or engine units."""
    return tag("path", d=d, fill=fill, stroke=stroke, stroke_width=width, stroke_linecap="round", stroke_linejoin="round", **attrs)


def rect(x, y, w, h, fill, stroke="none", width=2, radius=0, **attrs):
    """Draw an authored surface or framing rectangle."""
    return tag("rect", x=x, y=y, width=w, height=h, fill=fill, stroke=stroke, stroke_width=width, rx=radius, **attrs)


def ellipse(x, y, rx, ry, fill, stroke="none", width=2, **attrs):
    """Draw a curved component or light."""
    return tag("ellipse", cx=x, cy=y, rx=rx, ry=ry, fill=fill, stroke=stroke, stroke_width=width, **attrs)


def text(x, y, words, size, fill, **attrs):
    """Letter the study with a local font and no external assets."""
    return tag("text", escape(words), x=x, y=y, font_family="DejaVu Sans, sans-serif", font_size=size, fill=fill, **attrs)


def group(art, transform="", **attrs):
    """Place an authored drawing without deforming its proportions."""
    return tag("g", art, transform=transform, **attrs)
