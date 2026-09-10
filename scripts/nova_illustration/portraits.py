"""Forward-facing portraits on the close and conversational body staging."""

from .colors import ELENA, JONAH, JADE
from .faces import FACE_COLORS, frontal_head
from .svg import group, path

WORK_CREW = ('leila', 'rina', 'samir', 'tomas')
"""Characters whose standing body is the shared work bust."""

GANTRY_CREW = ('nadia', 'owen', 'ivo')
"""Characters whose standing body is a separately authored civilian workship body."""


def elena_close(expression='original'):
    """Keep the close-up body; expression selects features and defaults to the original."""
    art = path("M127 217L213 212L209 285L241 308L181 356L95 308L128 272Z", ELENA['skin'])
    art += path("M131 237Q171 263 210 231L209 271L175 293L128 275Z", ELENA['shadow'], "none")
    art += path("M112 286L66 305Q-9 323 -47 419L-79 564H326L311 391Q298 327 237 309L210 286L175 317Z", JADE)
    art += path("M210 287L237 309Q298 327 311 391L326 564H219L212 349Z", ELENA['coat_shadow'], "none")
    art += path("M110 287L175 317L210 287L221 316L196 353L140 339L97 313Z", ELENA['shirt'])
    art += path("M139 339L195 351L184 564H119Z", ELENA['shirt_shadow'], "none")
    art += path("M108 287L132 335L107 351L76 320M210 287L197 333L218 350L243 316", JADE)
    art += path("M80 322L111 354L93 419M216 354L227 407M-12 374L17 426L14 532M272 376L256 425", stroke=ELENA['coat_line'], width=2.5)
    art += path("M65 313Q17 331 -7 370M246 322Q275 333 287 358", stroke=ELENA['coat_light'], width=3)
    art += path("M-18 452L72 470L61 510L-27 492Z", ELENA['coat'], width=2)
    art += path("M-13 460L64 477", stroke=ELENA['coat_light'], width=1.5)
    art += group(frontal_head('elena', expression), 'translate(-159 -131) scale(1.08)')
    return art


def elena_gesture(expression='original'):
    """Keep the working gesture; expression selects features and defaults to the original."""
    art = path("M104 206L179 203L180 268L212 291L157 331L79 283L105 252Z", ELENA['skin'])
    art += path("M107 229L177 214L179 254L137 272L104 254Z", ELENA['shadow'], "none")
    art += path("M94 269L47 289Q-10 319 -35 401L-56 563H241L233 357L197 285L178 266L140 302Z", ELENA['coat'])
    art += path("M53 293L87 322L85 411L101 563H-56L-35 401Q-10 319 53 293Z", ELENA['coat_shadow'], "none")
    art += path("M94 269L140 302L178 266L191 286L170 326L139 348L104 329L77 291Z", ELENA['shirt'])
    art += path("M95 269L105 318L87 331L65 300M179 269L159 313L182 330L200 289", ELENA['coat'], width=2.2)
    art += path("M105 335L119 542M43 316Q6 339-5 378M17 414L11 509", stroke=ELENA['coat_line'], width=2)
    art += path("M186 291Q226 305 233 351L242 402L309 375L326 404L240 451Q218 456 201 432L173 359Z", JADE)
    art += path("M187 342L208 414Q221 439 240 436L315 394", stroke=ELENA['coat_light'], width=2.5)
    art += path("M310 377L328 363L346 333Q352 323 357 331L351 353L379 343Q391 339 392 347L367 360L400 354Q410 353 409 361L377 370L399 371Q408 372 405 379L369 386L388 388Q397 391 392 398L352 401L327 408Z", ELENA['skin'], width=2)
    art += path("M330 381L352 369M347 389L369 386M345 398L363 397", stroke=ELENA['lines'], width=1.5)
    art += group(frontal_head('elena', expression), 'translate(-137 -104) scale(0.94)')
    return art


def jonah_listener(expression='original'):
    """Keep the listening body; expression selects features and defaults to the original."""
    art = path("M124 223L220 218L228 279L289 309L196 392L83 328L135 284Z", JONAH['skin'])
    art += path("M132 246L218 224L225 268L178 299L134 282Z", JONAH['shadow'], "none")
    art += path("M123 290L186 326L228 283L296 319Q355 347 390 437L419 601H-56L-17 399Q10 340 80 317Z", JONAH['coat'], width=3)
    art += path("M229 284L296 319Q355 347 390 437L419 601H198L183 402Z", JONAH['coat_shadow'], "none")
    art += path("M113 296L183 327L228 283L244 311L206 359L172 372L97 328Z", JONAH['shirt'], width=2.2)
    art += path("M127 297L139 343L167 365L151 390L94 333M228 283L206 341L232 357L266 313", JONAH['coat'], width=2.2)
    art += path("M88 334Q41 357 26 390M267 327Q307 342 324 369M194 371L223 427M24 436L-5 550M322 403L352 491", stroke=JONAH['coat_light'], width=2)
    art += group(frontal_head('jonah', expression), 'translate(-160 -139) scale(1.12)')
    return art


def work_portrait(name, expression='original'):
    """Stage a work-crew body with the original features unless overridden."""
    if name not in WORK_CREW:
        raise KeyError(name)
    c = FACE_COLORS[name]
    art = path('M125 209L211 210L211 276L240 297L170 348L97 297L126 267Z',c['skin'],width=2.2)
    art += path('M126 226Q170 251 211 224L211 266L168 291L126 269Z',c['shadow'],'none',opacity=.75)
    art += path('M114 277L70 299Q14 314-14 374L-56 567H339L314 387Q303 331 248 305L210 276L169 310Z',c['coat'],width=2.8)
    art += path('M210 277L248 305Q303 331 314 387L339 567H219L209 347Z',c['coat_shadow'],'none')
    art += path('M114 278L169 310L211 277L225 303L190 347L143 339L96 304Z',c['shirt'],width=2.2)
    art += path('M114 278L135 327L117 347L85 313M211 277L194 327L218 347L244 309',c['coat'],width=2.2)
    art += path('M132 349L134 561M75 315Q33 331 18 359M245 315Q276 330 287 354M12 390L-8 490M273 402L291 501',stroke=c['coat_light'],width=2)
    art += path('M184 395H252V452H184Z',c['coat_shadow'],width=1.8)
    art += path('M190 403H246',stroke=c['coat_light'],width=1.4)
    art += group(frontal_head(name, expression),'translate(-130 -121)')
    return group(art,data_character=name,data_pose='work-bust')


def gantry_portrait(name, expression='original'):
    """Stage the three new frontal identities in separately authored civilian work clothes."""
    if name not in GANTRY_CREW:
        raise KeyError(name)
    c = FACE_COLORS[name]
    art = path('M126 211L211 211L210 274L232 297L167 337L103 295L126 265Z',c['skin'],width=2)
    art += path('M127 232Q166 254 210 229L210 266L166 289L127 266Z',c['shadow'],'none',opacity=.65)
    if name == 'nadia':
        art += path('M114 275L62 301Q14 323-4 380L-38 567H332L301 385Q286 327 239 301L210 274L167 310Z',c['coat'],width=2.6)
        art += path('M210 275L239 301Q286 327 301 385L332 567H195L192 334Z',c['coat_shadow'],'none')
        art += path('M115 276L167 310L210 275L216 304L185 339H145L107 306Z',c['shirt'],width=2)
        art += path('M111 278L138 326L125 345L87 307M210 276L190 332L213 341L240 309',c['coat'],width=2)
        art += path('M135 349L148 557M70 316L39 360M242 316L272 355M4 398L-8 486M283 392L302 483',stroke=c['coat_light'],width=2)
        art += path('M79 304L114 326M232 305L219 325',stroke=c['trim'],width=4)
        art += path('M210 376H265V420H210ZM216 384H259',stroke=c['coat_light'],width=1.5)
    elif name == 'owen':
        art += path('M111 276L50 297Q-13 319-34 398L-67 567H362L333 394Q313 320 248 300L210 275L169 310Z',c['coat'],width=2.8)
        art += path('M210 276L248 300Q313 320 333 394L362 567H207L198 341Z',c['coat_shadow'],'none')
        art += path('M113 278L165 312L210 277L222 317L197 358L171 347L145 361L94 318Z',c['shirt'],width=2)
        art += path('M117 279L148 329L126 353L78 310M210 278L194 333L219 351L259 309',c['coat'],width=2)
        art += path('M149 361L152 558M186 359L190 558M48 315Q10 341-5 381M260 317Q300 341 309 379',stroke=c['coat_light'],width=2)
        art += path('M29 400H117V467H29ZM218 400H302V467H218Z',c['coat_shadow'],width=1.8)
        art += path('M35 410H111M224 410H296M45 414V451M61 414V451',stroke=c['coat_light'],width=1.4)
        art += path('M-24 420L13 431L3 459L-33 448M311 429L341 416L347 444L318 457',c['shirt'],width=2)
    else:
        art += path('M117 279L66 306Q16 329-2 387L-34 567H324L305 391Q288 333 238 307L210 276L167 314Z',c['shirt'],width=2.6)
        art += path('M84 300L117 282L149 337L156 567H-14L13 386Q30 333 84 300Z',c['coat'],width=2)
        art += path('M211 280L244 310Q287 337 294 388L312 567H180L184 338Z',c['coat_shadow'],width=2)
        art += path('M119 281L146 315L162 339L167 320L184 338L211 280',stroke=c['coat_light'],width=2)
        art += path('M89 309L116 347L91 379M232 312L207 349L231 378M158 362V554M177 362V554',stroke=c['trim'],width=3)
        art += path('M57 409H133V482H57ZM200 409H273V482H200Z',c['coat'],width=2)
        art += path('M62 420H128M205 420H268M15 393L-3 478M288 391L303 476',stroke=c['coat_light'],width=2)
    art += group(frontal_head(name,expression),'translate(-130 -121)')
    return group(art,data_character=name,data_pose='gantry-work')


def standing_body(name, expression='original'):
    """Return the one full body drawn for a character, whichever pose helper owns it.

A scene places a person, not the helper that happens to hold their clothes, so
this is the only place that answers the question. Every registered face has a
standing body except Elena, whose authored drawings are the close and gesturing
conversational busts rather than a body a scene can hang on a rail.
"""
    if name in WORK_CREW:
        return work_portrait(name, expression)
    if name in GANTRY_CREW:
        return gantry_portrait(name, expression)
    if name == 'jonah':
        return jonah_listener(expression)
    raise KeyError(f'No standing body is drawn for {name!r}')


def reaching_arm(name):
    """Draw an open hand reaching for the next hold, independent of head and rail."""
    c = FACE_COLORS[name]
    art = path('M246 305Q268 290 290 256L386 201L402 228L319 294Q286 350 259 355Z',c['coat'],width=2.5)
    art += path('M275 310L307 275L388 218',stroke=c['coat_light'],width=2)
    art += path('M382 202L398 196L415 224L399 237Z',c['coat_shadow'],width=2)
    art += path('M399 200L415 189L423 167Q428 158 433 166L430 186L450 173Q460 168 462 175L443 191L467 182Q477 181 475 188L449 202L470 198Q480 201 473 207L447 213L459 214Q467 218 459 222L433 224L414 223Z',c['skin'],width=1.8)
    art += path('M417 203L434 194M428 212L447 213',stroke=c['lines'],width=1.3)
    return group(art,data_character=name,data_pose='reach-handhold')


def gripping_arm(name):
    """Draw a bent sleeve and closed hand; the scene supplies a rail at (5, 425).

Place this layer over the character's torso. It changes neither head nor
expression, and owns no equipment. The grip fits the work-bust drawing frame.
"""
    c = FACE_COLORS[name]
    art = path('M57 327Q13 333-9 373L-29 416L3 449L33 423L28 394L73 368Z',c['coat'],width=2.5)
    art += path('M45 345L14 375L-5 410M-18 418L2 434',stroke=c['coat_light'],width=2)
    art += path('M-10 403L17 405L24 431L-3 440L-18 423Z',c['coat_shadow'],width=2)
    art += path('M-2 410L-11 395Q-14 387-7 386Q-1 386 6 402L17 404Q29 405 28 416L23 438Q21 446 13 445L-5 438Q-11 434-9 428L-1 422Z',c['skin'],width=1.8)
    art += path('M4 411L24 416M1 420L23 425M0 430L20 435',stroke=c['lines'],width=1.3)
    return group(art,data_character=name,data_pose='handhold-grip')


def work_inspection(name, expression='original'):
    """Return body and posed forearms with space between layers for a work surface.

The scene supplies the equipment. Neither the pose nor its hands own a tool.
"""
    body = work_portrait(name, expression)
    c = FACE_COLORS[name]
    arms = path('M48 340Q18 369 36 413L100 477L128 451L81 399L87 372Z',c['coat'],width=2.5)
    arms += path('M39 390L63 419L109 463M57 352L72 382',stroke=c['coat_light'],width=2)
    arms += path('M100 446L128 448L139 461L112 485L94 470Z',c['coat_shadow'],width=2)
    arms += path('M127 450L149 434Q159 429 168 435L187 444Q194 447 190 453L171 448L197 458Q204 462 200 468L171 459L190 473Q195 479 190 482L158 465L137 470L122 465Z',c['skin'],width=1.8)
    arms += path('M145 445L158 451M151 455L166 457',stroke=c['lines'],width=1.3)
    arms += path('M270 340Q332 380 322 420L292 460L269 435L282 403L240 374Z',c['coat'],width=2.5)
    arms += path('M308 379L308 415L289 445',stroke=c['coat_light'],width=2)
    arms += path('M280 423L305 448L292 466L264 443Z',c['coat_shadow'],width=2)
    hand = path('M203 448L182 429Q175 424 170 430L176 443L154 440Q145 439 145 446L173 452L150 452Q142 453 145 460L175 462L153 465Q147 469 154 474L184 471L206 461Z',c['skin'],width=1.8)
    hand += path('M178 443L187 452M175 462L188 459',stroke=c['lines'],width=1.3)
    arms += group(hand,'translate(70 -15)')
    return body, group(arms,data_pose='inspection-forearms',data_character=name)


def supported_owen():
    """Recline Owen toward a foot-end viewer, keeping the original frontal head.

The foreshortened body owns no stretcher, restraints, injury, or treatment.
The scene must supply support behind him and retain control of its motion.
"""
    c = FACE_COLORS['owen']
    art = path('M104 423L241 423L267 493L306 587L274 635L196 618L169 513L142 618L65 635L29 587L76 487Z', c['coat'], width=2.5)
    art += path('M171 467L194 502L220 599L271 617L274 635L196 618L169 513L142 618L109 625L142 498Z', c['coat_shadow'], 'none')
    art += path('M73 502L105 521L72 577M222 509L256 534L279 580', stroke=c['coat_light'], width=2.5)
    art += path('M61 596L139 599L151 650Q151 674 118 680L28 679Q11 674 19 652Z', c['coat_shadow'], width=2.5)
    art += path('M199 599L279 596L320 652Q329 674 310 679L222 680Q190 674 190 650Z', c['coat_shadow'], width=2.5)
    art += path('M27 657L139 655M203 655L310 657M45 637L131 635M209 635L294 637', stroke=c['coat_light'], width=3)
    art += path('M143 211L197 211L212 273L237 296L168 330L104 292L126 264Z', c['skin'], width=2)
    art += path('M141 237Q168 253 198 237L201 258L168 277L137 257Z', c['shadow'], 'none', opacity=.5)
    art += path('M115 276L57 297Q16 324 24 371L67 472L270 472L307 368Q316 324 249 298L211 275L168 308Z', c['coat'], width=2.5)
    art += path('M211 276L249 298Q316 324 307 368L270 472H186L192 333Z', c['coat_shadow'], 'none')
    art += path('M115 278L167 310L211 277L223 312L197 350L169 340L144 353L98 313Z', c['shirt'], width=2)
    art += path('M117 279L147 325L127 345L81 309M211 278L192 328L219 346L258 308', c['coat'], width=2)
    art += path('M146 354L148 463M185 352L187 463M61 364H122V415H61ZM213 364H273V415H213Z', stroke=c['coat_light'], width=1.8)
    art += path('M56 300Q16 314 14 356L20 421L71 486L102 468L63 404L77 344M250 300Q313 316 320 357L314 421L264 486L232 468L271 404L243 344', c['coat'], width=2.5)
    art += path('M30 357L38 416L81 468M301 358L296 416L251 468', stroke=c['coat_light'], width=2)
    art += path('M72 468L95 456L114 480L119 502Q116 510 110 503L99 486L107 511Q106 520 100 514L86 490L93 513Q92 521 86 514L73 490Z', c['skin'], width=1.8)
    art += path('M263 468L240 456L221 480L216 502Q219 510 225 503L236 486L228 511Q229 520 235 514L249 490L242 513Q243 521 249 514L262 490Z', c['skin'], width=1.8)
    head = group(frontal_head('owen'), 'translate(-130 -121)')
    art += group(head, 'translate(167 215) scale(.7) translate(-167 -215)')
    return group(art, data_character='owen', data_pose='supported-recline')


def freefall_guide(name):
    """Continue Rina's or Ivo's work bust into a body, so a distant guide is not a cutout."""
    if name not in ('rina', 'ivo'):
        raise KeyError(name)
    c = FACE_COLORS[name]
    art = path('M-20 540H318L303 694L267 825L278 1108L212 1147L165 893L150 730L130 886L90 1138L25 1107L35 817L-9 678Z', c['coat_shadow'], width=2.5)
    art += path('M20 641L68 725L73 825L51 1048M259 644L222 728L221 827L250 1046M149 628L150 730', stroke=c['coat_light'], width=2.5)
    art += path('M23 1091L94 1108L90 1179L-22 1204Q-49 1200-36 1180L9 1141ZM210 1108L278 1091L293 1141L337 1180Q351 1200 323 1204L213 1179Z', c['coat_shadow'], width=2.5)
    art += path('M-22 1186L78 1165M222 1165L324 1186', stroke=c['coat_light'], width=3)
    body = standing_body(name)
    return group(art + body, data_character=name, data_pose='freefall-guide')


def stretcher_grip(name):
    """Reach from the work-body shoulder to a scene rail at (430, 433).

Mirror this arm layer alone for a left-hand grip. The scene owns the rail;
this layer contains no head, equipment, or assumption about gravity.
"""
    c = FACE_COLORS[name]
    fabric = c['shirt'] if name == 'ivo' else c['coat']
    art = path('M246 305Q280 300 295 348L321 397L421 415L418 449L304 435Q285 433 277 412L246 354Z', fabric, width=2.5)
    art += path('M270 320L299 403Q304 415 321 418L410 433', stroke=c['coat_light'], width=2)
    art += path('M409 412L428 416L429 448L411 451Z', c['coat_shadow'], width=2)
    art += group(rail_grip_hand(name), 'translate(430 433) scale(-1 1)')
    return group(art, data_character=name, data_pose='stretcher-grip', data_grip='430 433')


def rail_grip_hand(name):
    """Close a hand around a vertical scene rail at the local origin, without an arm."""
    c = FACE_COLORS[name]
    art = path('M-2 410L-11 395Q-14 387-7 386Q-1 386 6 402L17 404Q29 405 28 416L23 438Q21 446 13 445L-5 438Q-11 434-9 428L-1 422Z', c['skin'], width=1.8)
    art += path('M4 411L24 416M1 420L23 425M0 430L20 435', stroke=c['lines'], width=1.3)
    return group(group(art, 'translate(-5 -425)'), data_character=name, data_pose='rail-grip-hand')
