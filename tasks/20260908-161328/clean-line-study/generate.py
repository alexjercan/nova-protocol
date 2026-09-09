#!/usr/bin/env python3
"""Compose three private style studies from the reusable illustration library."""

import argparse
import sys
from dataclasses import dataclass
from html import escape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0,str(ROOT/'scripts'))
sys.path.insert(0,str(ROOT/'web/src/comics/season-1/episode-1/art'))
from opening_props import mug_hand
from nova_illustration.colors import INK, PAPER, INTERIOR, FOLIO, SKY, SATURN, REVIEW
from nova_illustration.lettering import speech
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener
from nova_illustration.scenery import baikal, saturn, stars
from nova_illustration.ships import render_ship
from nova_illustration.styles import lore_filter
from nova_illustration.svg import group, path, rect, tag, text


@dataclass(frozen=True)
class Study:
    """One test of a shot family, not a chronological comic page."""
    slug: str
    title: str
    purpose: str
    description: str
    art: str
    dialogue: tuple


def definitions():
    """Define scene lighting and clipping, not object geometry."""
    sky = tag('linearGradient',''.join(tag('stop',stop_color=SKY[k],offset=o) for k,o in [('top',0),('middle',0.6),('bottom',1)]),id='space',x2='0.8',y2='1')
    planet = tag('linearGradient',''.join(tag('stop',stop_color=SATURN[k],offset=o) for k,o in [('light',0),('middle',0.58),('shadow',1)]),id='planet',x2='0.9',y2='0.6')
    return tag('defs',sky+planet+'''<clipPath id="panel"><rect width="1416" height="840"/></clipPath>
<clipPath id="window"><rect x="423" y="24" width="991" height="612" rx="105"/></clipPath>
<clipPath id="globe"><circle r="310"/></clipPath>''')


def studies():
    """Stage existing dialogue and events with library drawings and local props."""
    lines = ("And bring the mug back.","We haven't budgeted","for a replacement.")
    art = rect(0,0,1416,840,INTERIOR['wall'])
    art += path('M0 0H785L585 840H0Z',INTERIOR['wall_light'],'none')
    art += path('M1152 0H1416V840H1220L1174 603Z',INTERIOR['panel'],width=2)
    art += path('M1234 0V840M1178 568H1416',stroke=INTERIOR['panel_line'],width=2)
    art += path('M0 705L520 607L900 712L545 840H0Z',INTERIOR['worktop'],width=2)
    art += path('M0 716L522 619L724 675',stroke=INTERIOR['edge'],width=3)
    art += group(elena_close(),'translate(713 158) scale(1.72)')
    art += mug_hand(360,650,1.72)
    close = Study('01-close-up','A person, not a portrait','The original forward-facing portrait treatment, a borrowed mug, and the same close framing.','Elena faces the reader; Jonah\'s hand and borrowed mug enter the foreground. A small smile accompanies the existing joke. A warm wall and cropped green panel suggest Baikal without an empty room.',art,(("Elena",lines,(62,68,568,(800,385),'right')),))

    art = rect(0,0,1416,840,INTERIOR['wall'])+path('M0 0H319L454 840H0Z',INTERIOR['wall_light'],'none')
    view = rect(423,24,991,612,'url(#space)')+stars(1416,640)
    view += path('M434 616L681 504L767 473',stroke=INTERIOR['structure'],width=29)
    view += path('M443 605L685 494L767 464',stroke=INTERIOR['structure_light'],width=6)
    view += render_ship('ebro','forward-quarter',858,407,0.91,False)
    art += group(view,clip_path='url(#window)')
    art += rect(408,9,1021,642,'none',INK,28,120)
    art += rect(413,14,1011,632,'none',INTERIOR['window_frame'],20,112)
    art += path('M468 571V124Q468 66 526 66M1370 592H1306',stroke=INTERIOR['window_light'],width=3)
    art += path('M1326 32V616',stroke=INTERIOR['panel_line'],width=10)
    art += path('M401 668H1416V705H390Z',INTERIOR['panel'],width=3)
    art += path('M424 673H1376',stroke=INTERIOR['edge'],width=3)
    art += group(elena_gesture(),'translate(157 247) scale(1.15)')
    art += group(jonah_listener(),'translate(1070 211) scale(1.35)')
    art += mug_hand(1055,781,0.96)
    window = Study('02-window','People and their work','Forward-facing heads on the same working gesture and listening shoulder. The window earns its space.','Elena and Jonah have forward-facing heads in the original portrait style. Elena keeps the open working gesture. Ebro is visible at the berth through a deep window frame. Its framed pressure vessels use the same model as the lore sheet.',art,(
        ('Elena',('Aquila has a replacement','assembly. Can Kaveri','collect it?'),(46,44,462,(412,380),'bottom')),
        ('Jonah',('Yes. We can leave as soon',"as everyone's aboard."),(923,42,461,(1230,361),'bottom')),
    ))

    art = rect(0,0,1416,840,'url(#space)')+stars(1416,840)
    art += saturn(1220,-211,1.38)+baikal(311,319,0.57)
    art += render_ship('ebro','forward-quarter',469,399,0.2,False)
    art += render_ship('kaveri','aft-quarter',1031,590,1.25,True)
    exterior = Study('03-exterior','Useful machinery, a larger world','One ship model per identity: the same shapes in scenes and orthographic design sheets.','Kaveri departs with its work cradle empty and handling arm stowed. The waiting Ebro and Baikal recede behind it, with Saturn beyond. No attack or pursuit appears. Plated hulls and industrial fixtures are proposals inspired by the game screenshots and asset recipes.',art,())
    return close,window,exterior


def render_svg(study):
    """Keep the color-treated art separate from speech, labels, and the page frame."""
    drawing = group(study.art,class_='scene-art',data_lore_filter=f'url(#{study.slug}-lore-color)')
    balloons = ''
    for _,lines,(x,y,w,tip,side) in study.dialogue:
        balloons += speech(x,y,w,lines,tip,side)
    art = definitions()+lore_filter(study.slug)+rect(0,0,1500,1000,PAPER)
    art += text(43,48,'NOVA PROTOCOL',21,FOLIO['title'],font_weight='bold',letter_spacing=3.5)
    art += text(1457,48,'CLEAN-LINE STUDY / '+study.slug[:2],13,FOLIO['muted'],text_anchor='end',letter_spacing=2)
    art += group(group(drawing+balloons,clip_path='url(#panel)')+rect(0,0,1416,840,'none',INK,2.5),'translate(42 86)')
    art += text(43,958,study.title,18,FOLIO['title'])
    art += text(43,983,'PRIVATE STYLE TEST / DESIGNS PROVISIONAL / NOT NEW STORY EVENTS',11,FOLIO['muted'],letter_spacing=1.3)
    art = art.replace('id="',f'id="{study.slug}-').replace('url(#',f'url(#{study.slug}-')
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="1500" height="1000" viewBox="0 0 1500 1000" role="img" aria-labelledby="{study.slug}-title {study.slug}-description"><title id="{study.slug}-title">{escape(study.title)}</title><desc id="{study.slug}-description">{escape(study.description)}</desc>{art}</svg>\n'


def render_review(authored,svgs):
    """Offer offline comparison and a color-only switch using the export shader."""
    sections = []
    for s,svg in zip(authored,svgs):
        transcript = ''.join(f'<li><strong>{escape(speaker)}:</strong> {escape(" ".join(lines))}</li>' for speaker,lines,_ in s.dialogue)
        sections.append(f'<section id="{s.slug}"><div class="heading"><h2>{escape(s.title)}</h2><a href="{s.slug}.svg">Full-size SVG</a></div>{svg}<p>{escape(s.purpose)}</p><details><summary>Description and dialogue</summary><p>{escape(s.description)}</p><ul>{transcript}</ul>{"<p>Silent exterior.</p>" if not s.dialogue else ""}</details></section>')
    links = ''.join(f'<a href="#{s.slug}">{s.slug[:2]} {escape(s.title)}</a>' for s in authored)
    theme = ';'.join(f'--{key.replace("_","-")}:{value}' for key,value in REVIEW.items())
    return '''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Nova / Clean-line style study</title><style>
:root{'''+theme+''';color-scheme:dark}*{box-sizing:border-box}body{margin:0;background:var(--background);color:var(--text);font:16px/1.6 system-ui,sans-serif}header,nav,main,footer{max-width:1500px;margin:auto;padding:24px 32px}header{padding-bottom:8px}h1{font-size:clamp(26px,4vw,44px);line-height:1.15;font-weight:500;margin:8px 0 16px}p{max-width:80ch;color:var(--paragraph)}.eyebrow{color:var(--mint);font-size:11px;letter-spacing:.2em;font-weight:700;margin:0}a{color:var(--mint)}a:focus-visible,button:focus-visible,summary:focus-visible{outline:3px solid var(--focus);outline-offset:4px}nav{display:flex;align-items:center;gap:12px 22px;flex-wrap:wrap;padding-top:8px;padding-bottom:16px}nav a{font-size:13px}button{font:inherit;font-size:13px;padding:9px 16px;border:1px solid var(--border);border-radius:4px;background:var(--button);color:var(--text);cursor:pointer}button[aria-pressed="true"]{background:var(--mint);color:var(--button-text)}main{padding-top:0}section{scroll-margin-top:18px;margin:16px 0 56px}.heading{display:flex;justify-content:space-between;align-items:baseline;gap:16px;margin-bottom:12px}h2{font-size:21px;font-weight:500;line-height:1.25;margin:0}.heading a{white-space:nowrap;font-size:13px}section>svg{width:100%;height:auto;display:block}details{padding:12px 18px;background:var(--surface);border-radius:4px;max-width:85ch}summary{cursor:pointer}details li{margin:8px 0}.art-only .speech{visibility:hidden}footer{font-size:13px;padding-top:0;border-top:1px solid var(--border)}@media(max-width:700px){header,nav,main,footer{padding-left:16px;padding-right:16px}h2{font-size:18px}.heading{align-items:start}.heading a{font-size:11px}section{margin-bottom:36px}p{font-size:15px}}@media print{body{background:white}header,nav,footer,.heading,section>p,details{display:none}main{padding:0;max-width:none}section{margin:0;break-after:page}@page{size:landscape;margin:0}}
</style></head><body><header><p class="eyebrow">NOVA PROTOCOL / PRIVATE STYLE STUDY</p><h1>People close. A world beyond them.</h1><p>Three shot tests using existing dialogue and events, not new story pages. Ships now share a solid model between every scene and design-sheet view.</p><p><a href="../proof/before-clean-line-opening/index.html">Compare with the previous four-page PoC</a> / <a href="../comic-opening-poc/index.html">Restyled four-page opening</a></p></header><nav aria-label="Study shots">''' + links + '''<button type="button" id="art" aria-pressed="false">Art only</button><button type="button" id="scheme" aria-pressed="false">Lore colors</button></nav><main>''' + ''.join(sections) + '''</main><footer><p>The Lore colors switch applies the same color transform used by the encyclopedia exports. It changes no geometry and does not tint the speech balloons or page labels.</p><p>Shared-source lore proposals: <a href="../../../web/src/assets/lore/kaveri-design-concept.svg">Kaveri design sheet</a> / <a href="../../../web/src/assets/lore/ebro-design-concept.svg">Ebro design sheet</a> / <a href="../../../web/src/assets/lore/elena-ward-portrait-concept.svg">Elena portrait</a>.</p><p>All appearances, dimensions, construction, and framing remain proposals. On a small screen, open an SVG to zoom or expand the text description. This is not a final mobile comic reader.</p><p><a href="README.md">Scope and regeneration</a></p></footer><script>
document.getElementById('art').addEventListener('click',event=>{const active=document.body.classList.toggle('art-only');event.currentTarget.setAttribute('aria-pressed',String(active))});
document.getElementById('scheme').addEventListener('click',event=>{const active=event.currentTarget.getAttribute('aria-pressed')!=='true';event.currentTarget.setAttribute('aria-pressed',String(active));document.querySelectorAll('.scene-art').forEach(art=>{if(active)art.setAttribute('filter',art.dataset.loreFilter);else art.removeAttribute('filter')})});
</script></body></html>
'''


def main():
    """Regenerate only the three study SVGs and their local review sheet."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    authored = studies()
    svgs = tuple(render_svg(s) for s in authored)
    outputs = {f'{s.slug}.svg':svg for s,svg in zip(authored,svgs)}
    outputs['index.html'] = render_review(authored,svgs)
    for filename,content in outputs.items():
        target = OUTPUT/filename
        encoded = content.encode('utf-8')
        if args.check:
            if not target.is_file() or target.read_bytes()!=encoded:
                raise SystemExit(f'Out of date: {target}')
        else:
            target.write_bytes(encoded)
    print(f"{'Verified' if args.check else 'Rendered'} three shared-source studies and their offline review sheet")


if __name__=='__main__':
    main()
