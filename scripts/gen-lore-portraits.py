#!/usr/bin/env python3
"""Render the five authored crew portrait concepts; --check verifies saved SVGs."""

import argparse
import sys
from dataclasses import dataclass
from html import escape
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from nova_illustration.faces import JONAH_FACE


@dataclass(frozen=True)
class Portrait:
    """Explicit face, palette, and clothing choices for one portrait study."""

    name: str
    role: str
    skin: str
    shadow: str
    light: str
    hair: str
    coat: str
    head: str
    back_hair: str
    front_hair: str
    features: str
    collar: str


PORTRAITS = (
    Portrait(
        name="Jonah Mercer",
        role="WORKSHIP CAPTAIN",
        skin="#b58e72", shadow="#695346", light="#dec0a1",
        hair="#39332f", coat="#4a5a64",
        head=JONAH_FACE.head,
        back_hair=JONAH_FACE.back_hair,
        front_hair=JONAH_FACE.front_hair,
        features=JONAH_FACE.features,
        collar='<path d="M253 401L297 447L270 484L220 415M347 401L297 447L323 484L382 416" fill="#728088" stroke="#34454e" stroke-width="3"/><path d="M297 452V590" stroke="#a7aaa0" stroke-width="3"/>',
    ),
    Portrait(
        name="Leila Haddad",
        role="CHIEF ENGINEER",
        skin="#b39577", shadow="#6f5746", light="#dfc5a1",
        hair="#35312d", coat="#68716b",
        head="M223 185Q237 143 296 145Q352 150 370 198L363 290Q351 343 303 366Q256 355 236 315L221 266Z",
        back_hair='<path d="M216 310Q181 307 195 260Q176 217 200 173Q193 141 230 126Q243 105 289 116Q342 100 372 142Q402 157 399 207Q429 235 403 270Q414 312 377 331L342 325L244 332Z"/><ellipse cx="392" cy="204" rx="40" ry="52"/>',
        front_hair='<path d="M213 252Q199 215 216 171Q238 130 282 136Q336 120 369 172L377 238L356 219L350 183Q315 185 297 163Q268 188 240 186L234 237L224 264Z"/><path d="M223 168Q247 140 281 148M311 142Q348 143 366 178" fill="none" stroke="#807665" stroke-width="4"/>',
        features='''<path d="M242 228Q261 217 278 227M315 228Q332 218 350 230" fill="none" stroke="#514030" stroke-width="5"/>
<path d="M242 242Q260 230 279 242Q261 253 242 242M314 242Q333 232 350 244Q332 253 314 242" fill="#d2c5ab"/>
<g fill="#3b4137"><circle cx="261" cy="241" r="5"/><circle cx="331" cy="242" r="5"/></g>
<path d="M298 242L288 282Q298 290 309 282M268 313Q298 329 330 311" fill="none" stroke="#785c49" stroke-width="3" stroke-linecap="round"/>
<path d="M247 266L258 270M337 268L347 262M281 332Q299 336 315 329" fill="none" stroke="#99785a" stroke-width="2"/>''',
        collar='<path d="M248 397L292 441L276 468L218 417M350 397L307 441L325 468L383 417" fill="#b1ab8f" stroke="#555e55" stroke-width="3"/><path d="M187 477H259V539H187ZM337 477H409V539H337Z" fill="#53625c" stroke="#8e9a87" stroke-width="2"/>',
    ),
    Portrait(
        name="Tomas Vega",
        role="PILOT / NAVIGATOR",
        skin="#be9c83", shadow="#715a4b", light="#e0c5a9",
        hair="#3e3e3a", coat="#525c60",
        head="M227 170Q250 140 305 147Q356 154 370 195L363 289L345 338L311 373L281 368L246 329L229 281Z",
        back_hair='<path d="M222 250L211 187Q217 132 267 123Q330 109 367 152L378 207L368 261L353 231H238Z"/>',
        front_hair='<path d="M219 218L215 181Q226 139 270 132Q320 120 356 151L367 181L342 170L302 168L267 158L237 183L236 216L226 247Z"/><path d="M230 175Q237 155 254 151M256 147Q272 140 288 143M297 141Q319 141 335 152M341 154Q350 157 359 173" fill="none" stroke="#a3a398" stroke-width="2" stroke-linecap="round" opacity="0.4"/>',
        features='''<path d="M244 222L277 226M317 227L350 222" stroke="#4a4137" stroke-width="5"/>
<path d="M244 240Q262 232 279 242Q260 249 244 240M315 242Q333 232 351 240Q333 249 315 242" fill="#cabfa7"/>
<g fill="#36443d"><circle cx="262" cy="240" r="5"/><circle cx="332" cy="240" r="5"/></g>
<path d="M299 233L289 286L309 288M275 319L318 321M283 334L310 337" fill="none" stroke="#785c48" stroke-width="3" stroke-linecap="round"/>
<path d="M251 272L265 278M330 278L349 267M267 205L292 208" fill="none" stroke="#a48165" stroke-width="2"/>''',
        collar='<path d="M243 393L293 410V470L232 453L212 420M356 393L306 410V470L369 453L389 420" fill="#87928d" stroke="#394d53" stroke-width="3"/><path d="M298 429V590M176 492H244M354 492H421" stroke="#a8b2a5" stroke-width="3"/>',
    ),
    Portrait(
        name="Rina Okafor",
        role="WORK OPERATIONS LEAD",
        skin="#8b6755", shadow="#4d3c33", light="#c09a78",
        hair="#292b2a", coat="#857b62",
        head="M223 188Q245 145 297 148Q355 149 375 192L369 281Q359 333 318 363L276 362Q240 339 227 299L216 247Z",
        back_hair='<path d="M210 244Q194 226 200 201Q185 180 205 162Q203 136 231 133Q239 111 264 119Q283 100 304 116Q334 107 348 129Q375 122 389 151Q413 169 396 188Q409 214 387 237L368 254Z"/>',
        front_hair='<path d="M215 218L221 181Q236 170 253 184Q270 160 286 180Q307 164 326 180Q347 170 362 190L376 228L387 210L389 170L352 133L284 118L231 140L208 176Z"/><g fill="none" stroke="#625b4e" stroke-width="6" stroke-linecap="round"><path d="M222 153Q212 177 228 185M248 134Q233 158 248 175M277 124Q264 149 277 167M307 123Q296 149 309 164M337 134Q326 157 340 176M367 151Q357 176 372 187"/></g>',
        features='''<path d="M241 223Q259 216 278 226M316 225Q335 216 355 226" fill="none" stroke="#41322a" stroke-width="6"/>
<path d="M241 242Q261 231 281 243Q261 255 241 242M314 243Q335 231 356 242Q335 253 314 243" fill="#c6b799"/>
<g fill="#2c342e"><circle cx="263" cy="242" r="6"/><circle cx="333" cy="242" r="6"/></g>
<path d="M297 244L284 280Q297 295 317 281" fill="none" stroke="#554035" stroke-width="4"/>
<path d="M269 315Q283 306 299 311Q313 306 333 315Q300 340 269 315Z" fill="#795346"/>
<path d="M271 315Q300 321 330 315" fill="none" stroke="#4c3830" stroke-width="3"/>
<path d="M240 267L257 274M337 274L356 266" stroke="#b38a69" stroke-width="3"/>''',
        collar='<path d="M243 399L298 455L253 482L210 418M357 399L298 455L346 482L392 418" fill="#b8ab86" stroke="#615b4b" stroke-width="3"/><path d="M173 497H256V555H173ZM341 497H424V555H341Z" fill="#6a6957" stroke="#b4b191" stroke-width="3"/><path d="M299 459V590" stroke="#d3c6a0" stroke-width="4"/>',
    ),
    Portrait(
        name="Samir Bell",
        role="SYSTEMS / FIRST AID",
        skin="#ab8267", shadow="#644c3e", light="#d5b28c",
        hair="#35322d", coat="#596961",
        head="M225 188Q239 145 300 149Q352 152 370 194L361 291Q347 338 304 372Q268 358 244 330L228 280Z",
        back_hair='<path d="M217 247Q201 232 205 209Q187 187 205 167Q199 141 227 133Q236 108 265 119Q286 98 310 116Q339 101 359 133Q388 135 389 159Q410 178 391 202Q400 225 375 250Z"/>',
        front_hair='<path d="M216 224L218 173L244 151L270 136L312 126L351 148L381 174L380 221L361 237L357 194Q337 207 326 182Q299 205 286 181Q267 201 253 177L236 198L232 234Z"/><path d="M235 149Q220 170 240 176M270 135Q247 156 269 170M310 133Q286 159 311 176M344 149Q326 168 349 187" fill="none" stroke="#776957" stroke-width="4"/>',
        features='''<path d="M242 224L276 226M316 226L349 222" stroke="#48382c" stroke-width="5"/>
<path d="M243 243Q260 235 278 244Q260 251 243 243M316 243Q332 234 350 242Q333 252 316 243" fill="#c5b59a"/>
<g fill="#323b32"><circle cx="261" cy="244" r="5"/><circle cx="333" cy="243" r="5"/></g>
<g fill="none" stroke="#444e49" stroke-width="4"><rect x="233" y="228" width="53" height="35" rx="11"/><rect x="307" y="228" width="53" height="35" rx="11"/><path d="M286 239Q296 232 307 239M222 234L233 238M360 238L370 232"/></g>
<path d="M299 250L288 286L306 290M273 320Q297 325 323 315M286 336L308 335" fill="none" stroke="#70503d" stroke-width="3" stroke-linecap="round"/>''',
        collar='<path d="M249 403L291 445L276 465L222 420M351 403L308 445L326 465L380 421" fill="#a5ad98" stroke="#455b50" stroke-width="3"/><path d="M302 461V590M338 491H415V543H338" fill="none" stroke="#9da995" stroke-width="3"/><path d="M352 510H384M352 523H397" stroke="#b8c2ac" stroke-width="3"/>',
    ),
)


def render(portrait: Portrait) -> str:
    """Compose one self-contained SVG without random or external resources."""
    name, role = escape(portrait.name), escape(portrait.role)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="600" height="720" viewBox="0 0 600 720" role="img" aria-labelledby="title description">
<title id="title">{name}: portrait concept</title>
<desc id="description">An authored civilian workship portrait study for {name}, {role.lower()}. Appearance, clothing, and underlying colors are proposals, not established character designs. Generated by scripts/gen-lore-portraits.py.</desc>
<defs>
  <filter id="encyclopedia-green" filterUnits="userSpaceOnUse" x="0" y="0" width="600" height="720" color-interpolation-filters="sRGB">
    <feColorMatrix type="saturate" values="0"/>
    <feComponentTransfer><feFuncR type="gamma" amplitude="0.86" exponent="1.25" offset="0"/><feFuncG type="linear" slope="0.96" intercept="0"/><feFuncB type="gamma" amplitude="0.81" exponent="1.1" offset="0"/></feComponentTransfer>
  </filter>
  <radialGradient id="backdrop" cx="40%" cy="35%" r="70%"><stop stop-color="#6d7b70"/><stop offset="0.65" stop-color="#283e36"/><stop offset="1" stop-color="#101d1c"/></radialGradient>
  <linearGradient id="skin" x2="0.9" y2="0.2"><stop stop-color="{portrait.light}"/><stop offset="0.42" stop-color="{portrait.skin}"/><stop offset="1" stop-color="{portrait.shadow}"/></linearGradient>
  <linearGradient id="cloth" x2="1" y2="0.5"><stop stop-color="#a4aea0"/><stop offset="0.18" stop-color="{portrait.coat}"/><stop offset="1" stop-color="#263c39"/></linearGradient>
  <linearGradient id="fade" x2="0" y2="1"><stop stop-color="#101d1c" stop-opacity="0"/><stop offset="1" stop-color="#101d1c"/></linearGradient>
</defs>
<g id="artwork" filter="url(#encyclopedia-green)">
  <rect width="600" height="720" fill="url(#backdrop)"/>
  <path d="M40 112H170M40 112V594H110M560 112H430M560 112V594H490" fill="none" stroke="#a9b7a3" stroke-opacity="0.25"/>
  <text x="40" y="62" fill="#b7c5b3" font-family="DejaVu Sans Mono, monospace" font-size="17" letter-spacing="2">PORTRAIT STUDY</text>
  <g fill="{portrait.hair}">{portrait.back_hair}</g>
  <path d="M259 328L255 400L211 423L298 487L389 422L346 400L341 326Z" fill="url(#skin)"/>
  <path d="M258 353Q298 387 341 349L345 384Q298 418 257 382Z" fill="{portrait.shadow}" opacity="0.45"/>
  <path d="M255 403L299 444L346 403L416 429Q480 449 502 517L523 611H76L100 520Q120 453 181 431Z" fill="url(#cloth)" stroke="#233732" stroke-width="4"/>
  <path d="M248 408L299 437L350 408L328 587H273Z" fill="#303d37"/>
  <path d="M169 455L146 581M430 455L453 581" fill="none" stroke="#acb9a5" stroke-width="2" opacity="0.45"/>
  {portrait.collar}
  <g fill="url(#skin)" stroke="{portrait.shadow}" stroke-width="2"><ellipse cx="221" cy="253" rx="14" ry="25"/><ellipse cx="368" cy="253" rx="14" ry="25"/></g>
  <path d="{portrait.head}" fill="url(#skin)" stroke="{portrait.shadow}" stroke-width="2"/>
  <path d="M236 259Q244 297 268 307L253 326Q229 290 236 259" fill="{portrait.light}" opacity="0.18"/>
  <g fill="{portrait.hair}">{portrait.front_hair}</g>
  {portrait.features}
  <rect y="552" width="600" height="168" fill="url(#fade)"/>
  <path d="M40 620H128" stroke="#aabfaa" stroke-width="3"/>
  <g font-family="DejaVu Sans, sans-serif"><text x="40" y="662" fill="#e1e4cf" font-size="32" letter-spacing="2">{name.upper()}</text><text x="40" y="694" fill="#9fb8ac" font-size="18" letter-spacing="2">{role}</text></g>
  <rect x="16" y="16" width="568" height="688" fill="none" stroke="#8f9e8c" stroke-opacity="0.25"/>
</g>
</svg>
'''


def main() -> None:
    """Write only these portrait assets, or fail if their saved renders differ."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Verify without writing files")
    args = parser.parse_args()
    output = Path(__file__).resolve().parents[1] / "web/src/assets/lore"
    if not args.check:
        output.mkdir(parents=True, exist_ok=True)
    for portrait in PORTRAITS:
        slug = portrait.name.lower().replace(" ", "-")
        target = output / f"{slug}-portrait-concept.svg"
        content = render(portrait).encode("utf-8")
        if args.check:
            if not target.exists() or target.read_bytes() != content:
                raise SystemExit(f"Out of date: {target}")
        else:
            target.write_bytes(content)
    print(f"{'Verified' if args.check else 'Rendered'} {len(PORTRAITS)} crew portraits")


if __name__ == "__main__":
    main()
