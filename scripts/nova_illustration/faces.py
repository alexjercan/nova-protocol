"""Forward-facing head drawings retained from the original portrait treatment."""

from dataclasses import dataclass

from .colors import ELENA, IVO, JONAH, LEILA, NADIA, OWEN, RINA, SAMIR, TOMAS
from .expressions import EXPRESSIONS, facial_features
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
    features=facial_features('jonah', 'original'),
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
    features=facial_features('rina', 'original'),
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

SAMIR_FACE = FrontalFace(
    head="M225 188Q239 145 300 149Q352 152 370 194L361 291Q347 338 304 372Q268 358 244 330L228 280Z",
    back_hair='<path d="M217 247Q201 232 205 209Q187 187 205 167Q199 141 227 133Q236 108 265 119Q286 98 310 116Q339 101 359 133Q388 135 389 159Q410 178 391 202Q400 225 375 250Z"/>',
    front_hair=f'<path d="M216 224L218 173L244 151L270 136L312 126L351 148L381 174L380 221L361 237L357 194Q337 207 326 182Q299 205 286 181Q267 201 253 177L236 198L232 234Z"/><path d="M235 149Q220 170 240 176M270 135Q247 156 269 170M310 133Q286 159 311 176M344 149Q326 168 349 187" fill="none" stroke="{SAMIR["hair_line"]}" stroke-width="4"/>',
    features=f'''<path d="M242 224L276 226M316 226L349 222" stroke="{SAMIR['brow']}" stroke-width="5"/>
<path d="M243 243Q260 235 278 244Q260 251 243 243M316 243Q332 234 350 242Q333 252 316 243" fill="{SAMIR['eye']}"/>
<g fill="{SAMIR['iris']}"><circle cx="261" cy="244" r="5"/><circle cx="333" cy="243" r="5"/></g>
<g fill="none" stroke="{SAMIR['glasses']}" stroke-width="4"><rect x="233" y="228" width="53" height="35" rx="11"/><rect x="307" y="228" width="53" height="35" rx="11"/><path d="M286 239Q296 232 307 239M222 234L233 238M360 238L370 232"/></g>
<path d="M299 250L288 286L306 290M273 320Q297 325 323 315M286 336L308 335" fill="none" stroke="{SAMIR['lines']}" stroke-width="3" stroke-linecap="round"/>''',
)

NADIA_FACE = FrontalFace(
    head='M226 184Q248 146 295 147Q346 143 368 188L364 278L346 324Q327 349 300 359L271 349L244 324L229 279Z',
    back_hair='<path d="M215 291L207 193Q200 150 237 130Q265 112 309 121Q364 115 385 162L383 290L355 317L237 316Z"/>',
    front_hair=f'<path d="M214 251L213 183Q222 134 268 131Q325 114 367 152L378 205L367 245L354 218L353 173Q308 191 263 168Q246 183 234 215L231 257Z"/><path d="M232 173Q244 148 267 145M271 144Q317 128 350 153M355 167L365 195" fill="none" stroke="{NADIA["hair_line"]}" stroke-width="3"/>',
    features=f'''<path d="M242 223L258 219L279 225M315 225L334 220L350 224" fill="none" stroke="{NADIA['brow']}" stroke-width="4"/>
<path d="M242 241Q259 230 278 242Q262 252 242 241M315 242Q334 230 350 240Q335 252 315 242" fill="{NADIA['eye']}"/>
<g fill="{NADIA['iris']}"><ellipse cx="261" cy="241" rx="5" ry="5.5"/><ellipse cx="333" cy="241" rx="5" ry="5.5"/></g>
<path d="M298 238L287 279Q294 287 305 280M270 310Q297 315 327 308M283 325L309 325M241 263L254 267M337 267L350 261" fill="none" stroke="{NADIA['lines']}" stroke-width="2.8" stroke-linecap="round"/>''',
)

OWEN_FACE = FrontalFace(
    head='M223 190Q232 150 294 149Q354 147 372 190L368 279Q365 315 344 340Q324 363 297 365Q266 362 244 337Q225 309 222 271Z',
    back_hair='<path d="M213 269L204 212Q200 165 227 149Q251 125 294 130Q341 121 374 156L386 215L376 274L360 278L229 281Z"/>',
    front_hair=f'<path d="M214 245L211 202L222 168L244 152L260 147Q292 130 325 147L344 151L370 176L375 230L364 248L352 210L350 186Q297 176 243 188L238 220L229 250Z"/><path d="M219 190L226 175M226 203L233 179M359 178L367 199M361 203L369 220M273 145L287 141M307 141L322 145" fill="none" stroke="{OWEN["hair_line"]}" stroke-width="3"/>',
    features=f'''<path d="M240 226Q258 216 279 225M313 225Q333 215 353 227" fill="none" stroke="{OWEN['brow']}" stroke-width="5"/>
<path d="M242 243Q262 233 280 243Q263 252 242 243M313 243Q333 233 353 243Q332 253 313 243" fill="{OWEN['eye']}"/>
<g fill="{OWEN['iris']}"><circle cx="262" cy="243" r="5"/><circle cx="333" cy="243" r="5"/></g>
<path d="M294 245L283 280Q296 292 314 281M268 316Q296 328 327 314M285 337Q299 340 313 335M240 261L259 266M333 267L353 260" fill="none" stroke="{OWEN['lines']}" stroke-width="3" stroke-linecap="round"/>
<path d="M250 325L256 332M260 335L266 340M274 345L281 348M315 346L322 343M332 334L338 326" fill="none" stroke="{OWEN['hair_line']}" stroke-width="2"/>''',
)

IVO_FACE = FrontalFace(
    head='M228 177Q253 142 300 148Q351 150 368 185L365 274L352 320L329 351L303 377L276 363L248 331L232 283Z',
    back_hair='<path d="M216 251L211 190Q218 148 250 133Q278 118 313 125Q350 115 374 156L382 212L371 259L356 247H232Z"/>',
    front_hair=f'<path d="M219 229L218 183Q230 146 262 141L271 124L295 133L313 118L335 139L352 134L370 170L371 218L359 230L350 181Q312 189 278 166L240 185L235 226L228 253Z"/><path d="M240 161L253 151M272 146L281 138M297 150L313 139M337 154L347 159" fill="none" stroke="{IVO["hair_line"]}" stroke-width="3"/><path d="M243 301L259 321L278 326L287 343L303 348L320 337L327 320L346 298L346 320L325 351L303 368L280 358L254 332Z" opacity=".7"/>',
    features=f'''<path d="M243 222L278 229M314 229L349 221" fill="none" stroke="{IVO['brow']}" stroke-width="4.5"/>
<path d="M243 242Q261 234 278 243Q261 252 243 242M315 244Q333 233 350 241Q333 252 315 244" fill="{IVO['eye']}"/>
<g fill="{IVO['iris']}"><circle cx="261" cy="242" r="5"/><circle cx="333" cy="242" r="5"/></g>
<path d="M300 238L291 281L309 285M273 314L295 318L322 311M286 331L312 329M245 268L260 276M334 275L350 264" fill="none" stroke="{IVO['lines']}" stroke-width="2.8" stroke-linecap="round"/>''',
)

FACES = {'elena': ELENA_FACE, 'jonah': JONAH_FACE, 'leila': LEILA_FACE, 'rina': RINA_FACE, 'samir': SAMIR_FACE, 'tomas': TOMAS_FACE, 'nadia': NADIA_FACE, 'owen': OWEN_FACE, 'ivo': IVO_FACE}
FACE_COLORS = {'elena': ELENA, 'jonah': JONAH, 'leila': LEILA, 'rina': RINA, 'samir': SAMIR, 'tomas': TOMAS, 'nadia': NADIA, 'owen': OWEN, 'ivo': IVO}


def expression_names(name):
    """List the expressions authored for a known head; never invent a missing variant."""
    if name not in FACES:
        raise KeyError(name)
    return tuple(EXPRESSIONS[name]) if name in EXPRESSIONS else ('original',)


def frontal_head(name, expression='original'):
    """Draw the frontal likeness with selected features and unchanged head geometry.

Omitting expression keeps the original drawing, not a universal neutral mood.
Only an override adds data-expression to the SVG; original exports keep their bytes.
"""
    face, colors = FACES[name], FACE_COLORS[name]
    features = face.features if expression == 'original' else facial_features(name, expression)
    art = group(face.back_hair, fill=colors['hair'])
    for x in (221, 368):
        art += ellipse(x, 253, 14, 25, colors['skin'], colors['lines'], 1.6)
        art += path(f'M{x-3} 246Q{x+8} 240 {x+5} 256L{x} 263', stroke=colors['lines'], width=1.4)
    art += path(face.head, colors['skin'], colors['lines'], 2)
    art += path('M343 197L361 218L357 282L345 322L317 346L303 354L320 331L339 288Z', colors['shadow'], 'none', opacity=.42)
    art += path('M237 212L252 215L249 249L256 285L246 292L233 263Z', colors['light'], 'none', opacity=.45)
    art += path('M264 283L277 283M324 282L338 278M279 344L300 349', stroke=colors['light'], width=1.3)
    art += group(face.front_hair, fill=colors['hair']) + features
    if name == 'elena' and expression == 'original':
        art += path('M244 242Q261 232 277 244Q262 253 244 242ZM315 244Q333 233 351 243Q333 254 315 244Z', colors['eye'], colors['ink'], 1.5)
        art += ellipse(261, 243, 4.5, 5, colors['iris']) + ellipse(333, 243, 4.5, 5, colors['iris'])
        art += path('M246 253L259 256M337 256L350 251M284 332Q299 335 314 329', stroke=colors['lines'], width=1.2)
    elif name == 'jonah':
        art += path('M230 160Q249 140 274 141M307 140L329 146M347 160L359 177', stroke=colors['hair_line'], width=1.6)
        art += path('M251 319L256 325M262 331L267 335M278 339L283 341M307 340L312 337M333 324L338 318', stroke=colors['stubble'], width=1)
    override = {} if expression == 'original' else {'data_expression': expression}
    return group(art, data_face=name, data_gaze='forward', **override)
