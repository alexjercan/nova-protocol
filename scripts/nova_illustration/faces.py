"""Forward-facing head drawings retained from the original portrait treatment."""

from dataclasses import dataclass

from .colors import ELENA, JONAH, LEILA, RINA, TOMAS
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


LEILA_FACE = FrontalFace(
    head="M223 185Q237 143 296 145Q352 150 370 198L363 290Q351 343 303 366Q256 355 236 315L221 266Z",
    back_hair='<path d="M216 310Q181 307 195 260Q176 217 200 173Q193 141 230 126Q243 105 289 116Q342 100 372 142Q402 157 399 207Q429 235 403 270Q414 312 377 331L342 325L244 332Z"/><ellipse cx="392" cy="204" rx="40" ry="52"/>',
    front_hair=f'<path d="M213 252Q199 215 216 171Q238 130 282 136Q336 120 369 172L377 238L356 219L350 183Q315 185 297 163Q268 188 240 186L234 237L224 264Z"/><path d="M223 168Q247 140 281 148M311 142Q348 143 366 178" fill="none" stroke="{LEILA["hair_line"]}" stroke-width="4"/>',
    features=f'''<path d="M242 228Q261 217 278 227M315 228Q332 218 350 230" fill="none" stroke="{LEILA['brow']}" stroke-width="5"/>
<path d="M242 242Q260 230 279 242Q261 253 242 242M314 242Q333 232 350 244Q332 253 314 242" fill="{LEILA['eye']}"/>
<g fill="{LEILA['iris']}"><circle cx="261" cy="241" r="5"/><circle cx="331" cy="242" r="5"/></g>
<path d="M298 242L288 282Q298 290 309 282M268 313Q298 329 330 311" fill="none" stroke="{LEILA['lines']}" stroke-width="3" stroke-linecap="round"/>
<path d="M247 266L258 270M337 268L347 262M281 332Q299 336 315 329" fill="none" stroke="{LEILA['detail']}" stroke-width="2"/>''',
)

RINA_FACE = FrontalFace(
    head="M223 188Q245 145 297 148Q355 149 375 192L369 281Q359 333 318 363L276 362Q240 339 227 299L216 247Z",
    back_hair='<path d="M210 244Q194 226 200 201Q185 180 205 162Q203 136 231 133Q239 111 264 119Q283 100 304 116Q334 107 348 129Q375 122 389 151Q413 169 396 188Q409 214 387 237L368 254Z"/>',
    front_hair=f'<path d="M215 218L221 181Q236 170 253 184Q270 160 286 180Q307 164 326 180Q347 170 362 190L376 228L387 210L389 170L352 133L284 118L231 140L208 176Z"/><g fill="none" stroke="{RINA["hair_line"]}" stroke-width="6" stroke-linecap="round"><path d="M222 153Q212 177 228 185M248 134Q233 158 248 175M277 124Q264 149 277 167M307 123Q296 149 309 164M337 134Q326 157 340 176M367 151Q357 176 372 187"/></g>',
    features=f'''<path d="M241 223Q259 216 278 226M316 225Q335 216 355 226" fill="none" stroke="{RINA['brow']}" stroke-width="6"/>
<path d="M241 242Q261 231 281 243Q261 255 241 242M314 243Q335 231 356 242Q335 253 314 243" fill="{RINA['eye']}"/>
<g fill="{RINA['iris']}"><circle cx="263" cy="242" r="6"/><circle cx="333" cy="242" r="6"/></g>
<path d="M297 244L284 280Q297 295 317 281" fill="none" stroke="{RINA['lines']}" stroke-width="4"/>
<path d="M269 315Q283 306 299 311Q313 306 333 315Q300 340 269 315Z" fill="{RINA['lip']}"/>
<path d="M271 315Q300 321 330 315" fill="none" stroke="{RINA['ink']}" stroke-width="3"/>
<path d="M240 267L257 274M337 274L356 266" stroke="{RINA['detail']}" stroke-width="3"/>''',
)

TOMAS_FACE = FrontalFace(
    head="M227 170Q250 140 305 147Q356 154 370 195L363 289L345 338L311 373L281 368L246 329L229 281Z",
    back_hair='<path d="M222 250L211 187Q217 132 267 123Q330 109 367 152L378 207L368 261L353 231H238Z"/>',
    front_hair=f'<path d="M219 218L215 181Q226 139 270 132Q320 120 356 151L367 181L342 170L302 168L267 158L237 183L236 216L226 247Z"/><path d="M230 175Q237 155 254 151M256 147Q272 140 288 143M297 141Q319 141 335 152M341 154Q350 157 359 173" fill="none" stroke="{TOMAS["hair_line"]}" stroke-width="2" stroke-linecap="round" opacity="0.4"/>',
    features=f'''<path d="M244 222L277 226M317 227L350 222" stroke="{TOMAS['brow']}" stroke-width="5"/>
<path d="M244 240Q262 232 279 242Q260 249 244 240M315 242Q333 232 351 240Q333 249 315 242" fill="{TOMAS['eye']}"/>
<g fill="{TOMAS['iris']}"><circle cx="262" cy="240" r="5"/><circle cx="332" cy="240" r="5"/></g>
<path d="M299 233L289 286L309 288M275 319L318 321M283 334L310 337" fill="none" stroke="{TOMAS['lines']}" stroke-width="3" stroke-linecap="round"/>
<path d="M251 272L265 278M330 278L349 267M267 205L292 208" fill="none" stroke="{TOMAS['detail']}" stroke-width="2"/>''',
)

FACES = {'elena': ELENA_FACE, 'jonah': JONAH_FACE, 'leila': LEILA_FACE, 'rina': RINA_FACE, 'tomas': TOMAS_FACE}
FACE_COLORS = {'elena': ELENA, 'jonah': JONAH, 'leila': LEILA, 'rina': RINA, 'tomas': TOMAS}


def frontal_head(name):
    """Draw a registered forward-facing head with light shadow planes and fine ink."""
    face, colors = FACES[name], FACE_COLORS[name]
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
    elif name == 'jonah':
        art += path('M230 160Q249 140 274 141M307 140L329 146M347 160L359 177', stroke=colors['hair_line'], width=1.6)
        art += path('M251 319L256 325M262 331L267 335M278 339L283 341M307 340L312 337M333 324L338 318', stroke=colors['stubble'], width=1)
    return group(art, data_face=name, data_gaze='forward')
