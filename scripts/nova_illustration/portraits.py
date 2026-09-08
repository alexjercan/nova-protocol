"""Forward-facing portraits on the close and conversational body staging."""

from .colors import ELENA, JONAH, JADE
from .faces import FACE_COLORS, frontal_head
from .svg import group, path


def elena_close():
    """Keep the close-up's neck, collar, and shoulders under Elena's original face."""
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
    art += group(frontal_head('elena'), 'translate(-159 -131) scale(1.08)')
    return art


def elena_gesture():
    """Keep the open working gesture and window-shot body, with a forward-facing head."""
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
    art += group(frontal_head('elena'), 'translate(-137 -104) scale(0.94)')
    return art


def jonah_listener():
    """Keep the near-shoulder framing and collar, with Jonah's original frontal face."""
    art = path("M124 223L220 218L228 279L289 309L196 392L83 328L135 284Z", JONAH['skin'])
    art += path("M132 246L218 224L225 268L178 299L134 282Z", JONAH['shadow'], "none")
    art += path("M123 290L186 326L228 283L296 319Q355 347 390 437L419 601H-56L-17 399Q10 340 80 317Z", JONAH['coat'], width=3)
    art += path("M229 284L296 319Q355 347 390 437L419 601H198L183 402Z", JONAH['coat_shadow'], "none")
    art += path("M113 296L183 327L228 283L244 311L206 359L172 372L97 328Z", JONAH['shirt'], width=2.2)
    art += path("M127 297L139 343L167 365L151 390L94 333M228 283L206 341L232 357L266 313", JONAH['coat'], width=2.2)
    art += path("M88 334Q41 357 26 390M267 327Q307 342 324 369M194 371L223 427M24 436L-5 550M322 403L352 491", stroke=JONAH['coat_light'], width=2)
    art += group(frontal_head('jonah'), 'translate(-160 -139) scale(1.12)')
    return art


def work_portrait(name):
    """Stage Leila, Rina, or Tomas with a frontal head and a resting work-jacket bust."""
    if name not in ('leila', 'rina', 'tomas'):
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
    art += group(frontal_head(name),'translate(-130 -121)')
    return group(art,data_character=name,data_pose='work-bust')
