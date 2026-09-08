"""Color-only presentation shared by comic art, lore portraits, and design sheets."""

from .colors import LORE_CHANNELS
from .svg import group, tag

SCHEMES = ('comic', 'lore')


def present(art, scheme, instance):
    """Apply a scheme to artwork, not labels or geometry.

The caller supplies a unique SVG-safe instance key for inline composition.
Comic returns the original colors. Lore maps luminance into the encyclopedia's
sage/olive range. The filter belongs on a child group for Firefox compatibility.
"""
    if scheme not in SCHEMES:
        raise ValueError(f'Unknown illustration scheme: {scheme}')
    if not instance or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_' for c in instance):
        raise ValueError('An SVG-safe instance key is required')
    if scheme == 'comic':
        return group(art, data_scheme=scheme, data_instance=instance)
    return lore_filter(instance)+group(art,filter=f'url(#{instance}-lore-color)',data_scheme=scheme,data_instance=instance)


def lore_filter(instance):
    """Define the shared color transform for exports or an interactive preview."""
    if not instance or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_' for c in instance):
        raise ValueError('An SVG-safe instance key is required')
    filter_id = f'{instance}-lore-color'
    channels = tag('feComponentTransfer',''.join(tag(name,**attrs) for name,attrs in LORE_CHANNELS))
    shader = tag('filter',tag('feColorMatrix',type='saturate',values='0')+channels,id=filter_id,x='-10%',y='-10%',width='120%',height='120%',color_interpolation_filters='sRGB')
    return tag('defs',shader)
