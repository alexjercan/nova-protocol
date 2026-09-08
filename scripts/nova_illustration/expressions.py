"""Authored facial features, selected per character without changing head geometry."""

from types import MappingProxyType

from .colors import JONAH, RINA


def _jonah_features(brows, eyes, pupils, mouth, details):
    """Keep Jonah's nose and feature materials while varying the acting lines."""
    return f'''<path d="{brows}" fill="none" stroke="{JONAH['brow']}" stroke-width="6"/>
<path d="{eyes}" fill="{JONAH['eye']}"/>
<g fill="{JONAH['iris']}">{pupils}</g>
<path d="M299 238L288 282L302 288L313 280{mouth}" fill="none" stroke="{JONAH['lines']}" stroke-width="3" stroke-linecap="round"/>
<path d="{details}" stroke="{JONAH['detail']}" stroke-width="2"/>'''


def _rina_features(brows, eyes, pupils, lips, mouth, cheeks):
    """Keep Rina's nose and lip treatment while varying the acting lines."""
    return f'''<path d="{brows}" fill="none" stroke="{RINA['brow']}" stroke-width="6"/>
<path d="{eyes}" fill="{RINA['eye']}"/>
<g fill="{RINA['iris']}">{pupils}</g>
<path d="M297 244L284 280Q297 295 317 281" fill="none" stroke="{RINA['lines']}" stroke-width="4"/>
<path d="{lips}" fill="{RINA['lip']}"/>
<path d="{mouth}" fill="none" stroke="{RINA['ink']}" stroke-width="3"/>
<path d="{cheeks}" stroke="{RINA['detail']}" stroke-width="3"/>'''


EXPRESSIONS = MappingProxyType({
    'jonah': MappingProxyType({
        'original': _jonah_features(
            brows='M241 227L263 222L277 227M314 227L334 220L350 224',
            eyes='M241 242Q258 232 277 242Q259 251 241 242M315 241Q333 231 350 240Q334 250 315 241',
            pupils='<circle cx="261" cy="241" r="5"/><circle cx="330" cy="240" r="5"/>',
            mouth='M269 317Q296 312 326 314',
            details='M280 326L310 325M237 251L250 255M340 253L354 248',
        ),
        'wry': _jonah_features(
            brows='M241 221L263 214L277 220M314 228L334 223L350 226',
            eyes='M241 241Q258 232 277 241Q259 249 241 241M315 242Q333 235 350 241Q334 248 315 242',
            pupils='<ellipse cx="261" cy="240" rx="5" ry="4"/><ellipse cx="330" cy="241" rx="5" ry="3"/>',
            mouth='M269 317Q293 321 312 314Q321 311 326 306',
            details='M282 328L315 322M237 253L249 256M341 251L354 246',
        ),
    }),
    'rina': MappingProxyType({
        'original': _rina_features(
            brows='M241 223Q259 216 278 226M316 225Q335 216 355 226',
            eyes='M241 242Q261 231 281 243Q261 255 241 242M314 243Q335 231 356 242Q335 253 314 243',
            pupils='<circle cx="263" cy="242" r="6"/><circle cx="333" cy="242" r="6"/>',
            lips='M269 315Q283 306 299 311Q313 306 333 315Q300 340 269 315Z',
            mouth='M271 315Q300 321 330 315',
            cheeks='M240 267L257 274M337 274L356 266',
        ),
        'amused': _rina_features(
            brows='M241 220Q259 211 278 220M316 222Q335 212 355 222',
            eyes='M241 242Q261 231 281 243Q262 252 241 242M314 243Q335 231 356 242Q335 252 314 243',
            pupils='<ellipse cx="263" cy="241.5" rx="6" ry="4.5"/><ellipse cx="333" cy="242" rx="6" ry="4.5"/>',
            lips='M269 309Q284 312 299 314Q314 312 333 307Q301 340 269 309Z',
            mouth='M270 310Q299 330 332 308',
            cheeks='M240 263L255 270M339 270L356 261',
        ),
    }),
})


def facial_features(name, expression):
    """Return a complete authored feature layer; unknown character/expression pairs fail.

These replace the original features rather than covering them with extra ink.
The registry contains only characters with expression variants authored so far.
"""
    try:
        return EXPRESSIONS[name][expression]
    except KeyError as error:
        raise KeyError(f'Unknown facial expression: {name}/{expression}') from error
