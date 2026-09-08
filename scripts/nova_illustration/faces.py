"""Forward-facing head drawings retained from the original portrait treatment."""

from dataclasses import dataclass

from .colors import ELENA, JONAH
from .svg import ellipse, group, path


@dataclass(frozen=True)
class FrontalFace:
    """Authored face contour, hair, and features in the original portrait coordinates."""
    head: str
    back_hair: str
    front_hair: str
    features: str


JONAH_FACE = FrontalFace(
    head="M221 184Q229 137 295 139Q354 140 371 186L365 291L350 337L317 365L276 363L241 334L225 290Z",
    back_hair='<path d="M213 236L207 177Q218 117 280 120Q337 110 375 153L382 229L363 255H226Z"/>',
    front_hair='<path d="M212 205L215 169Q234 125 279 133L290 120L313 129L333 125L355 143L369 144L377 196L357 220L351 173L312 163L271 176L238 168L231 216L218 248Z"/><path d="M239 316L258 337L285 344L310 343L336 335L354 312L350 337L317 365L276 363L241 334Z" opacity="0.48"/>',
    features=f'''<path d="M241 227L263 222L277 227M314 227L334 220L350 224" fill="none" stroke="{JONAH['brow']}" stroke-width="6"/>
<path d="M241 242Q258 232 277 242Q259 251 241 242M315 241Q333 231 350 240Q334 250 315 241" fill="{JONAH['eye']}"/>
<g fill="{JONAH['iris']}"><circle cx="261" cy="241" r="5"/><circle cx="330" cy="240" r="5"/></g>
<path d="M299 238L288 282L302 288L313 280M269 317Q296 312 326 314" fill="none" stroke="{JONAH['lines']}" stroke-width="3" stroke-linecap="round"/>
<path d="M280 326L310 325M237 251L250 255M340 253L354 248" stroke="{JONAH['detail']}" stroke-width="2"/>''',
)

ELENA_FACE = FrontalFace(
    head="M224 192Q235 145 300 146Q354 149 370 197L364 288Q350 340 306 364L277 357Q242 337 229 292Z",
    back_hair='<path d="M208 290L204 183Q210 119 287 117Q365 107 389 179L389 310L345 339L240 327Z"/>',
    front_hair=f'<path d="M210 239L214 179Q229 122 294 128Q353 115 379 179L378 241L358 219L350 169Q309 209 236 188L232 240Z"/><path d="M235 164Q271 134 315 142M246 177Q307 170 342 145" fill="none" stroke="{ELENA["hair_light"]}" stroke-width="9"/>',
    features=f'<path d="M243 227L277 224M315 225L348 230" fill="none" stroke="{ELENA["brow"]}" stroke-width="5"/><path d="M244 242Q261 235 277 244M315 244Q333 235 351 243" fill="none" stroke="{ELENA["ink"]}" stroke-width="5"/><path d="M299 244L288 281L307 286M269 315Q298 331 329 310M242 265L258 270M335 270L351 262" fill="none" stroke="{ELENA["lines"]}" stroke-width="3"/>',
)


def frontal_head(name):
    """Draw the original forward-facing head with light shadow planes and fine ink."""
    face, colors = {'elena': (ELENA_FACE, ELENA), 'jonah': (JONAH_FACE, JONAH)}[name]
    art = group(face.back_hair, fill=colors['hair'])
    for x in (221, 368):
        art += ellipse(x, 253, 14, 25, colors['skin'], colors['lines'], 1.6)
        art += path(f'M{x-3} 246Q{x+8} 240 {x+5} 256L{x} 263', stroke=colors['lines'], width=1.4)
    art += path(face.head, colors['skin'], colors['lines'], 2)
    art += path('M343 197L361 218L357 282L345 322L317 346L303 354L320 331L339 288Z', colors['shadow'], 'none', opacity=.42)
    art += path('M237 212L252 215L249 249L256 285L246 292L233 263Z', colors['light'], 'none', opacity=.45)
    art += path('M264 283L277 283M324 282L338 278M279 344L300 349', stroke=colors['light'], width=1.3)
    art += group(face.front_hair, fill=colors['hair']) + face.features
    if name == 'elena':
        art += path('M244 242Q261 232 277 244Q262 253 244 242ZM315 244Q333 233 351 243Q333 254 315 244Z', colors['eye'], colors['ink'], 1.5)
        art += ellipse(261, 243, 4.5, 5, colors['iris']) + ellipse(333, 243, 4.5, 5, colors['iris'])
        art += path('M246 253L259 256M337 256L350 251M284 332Q299 335 314 329', stroke=colors['lines'], width=1.2)
    else:
        art += path('M230 160Q249 140 274 141M307 140L329 146M347 160L359 177', stroke=colors['hair_line'], width=1.6)
        art += path('M251 319L256 325M262 331L267 335M278 339L283 341M307 340L312 337M333 324L338 318', stroke=colors['stubble'], width=1)
    return group(art, data_face=name, data_gaze='forward')
