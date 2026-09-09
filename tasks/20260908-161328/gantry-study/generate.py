#!/usr/bin/env python3
"""Render the private Gantry condition comparison and reusable crew studies."""

import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'scripts'))
from nova_illustration.colors import FOLIO, INK, PAPER, REVIEW, WORK
from nova_illustration.portraits import gantry_portrait
from nova_illustration.ships import bounds, render_ship
from nova_illustration.styles import present
from nova_illustration.svg import group, path, rect, tag, text

OUTPUT = Path(__file__).resolve().parent
CREW = [('nadia','NADIA SEN','Captain / pilot'),('owen','OWEN PARK','Engineer'),('ivo','IVO MARIN','Work operations')]


def document(title, description, art, height):
    """Keep local review labels outside the shared artwork."""
    return tag('svg',tag('title',title,id='title')+tag('desc',description,id='description')+art,xmlns='http://www.w3.org/2000/svg',width=1500,height=height,viewBox=f'0 0 1500 {height}',role='img',aria_labelledby='title description')+'\n'


def conditions():
    """Use matching cameras and scales so damage cannot silently reshape the ship."""
    art = rect(0,0,1500,1100,PAPER)
    art += text(40,48,'GANTRY / ONE SHIP, TWO CONDITIONS',30,FOLIO['title'],letter_spacing=1)
    art += text(40,82,'PRIVATE STORY STUDY / DAMAGE IS NOT PUBLIC LORE',15,FOLIO['muted'],letter_spacing=1)
    for label,x in [('INTACT',380),('STRANDED',1120)]:
        art += text(x,132,label,20,INK,text_anchor='middle',letter_spacing=2)
    for row,view in enumerate(['forward-quarter','aft-quarter']):
        both = [bounds('gantry',view,state) for state in ('intact','stranded')]
        lx,ly = min(b[0] for b in both),min(b[1] for b in both)
        hx,hy = max(b[2] for b in both),max(b[3] for b in both)
        scale = min(630/(hx-lx),310/(hy-ly))
        y = 340+row*385
        for x,state in [(380,'intact'),(1120,'stranded')]:
            art += rect(x-350,y-175,700,350,'none',FOLIO['muted'],1)
            art += text(x-331,y-148,view.upper(),14,FOLIO['muted'],letter_spacing=1)
            art += render_ship('gantry',view,x-(lx+hx)*scale/2,y-(ly+hy)*scale/2+10,scale,False,state)
    art += path('M40 940H1460',stroke=FOLIO['muted'],width=1)
    for n,line in enumerate([
        'Same cargo body, frame, crew module, and transfer collar in both views.',
        'Damage is limited to the aft service cover and roof. Main-drive thrust is disabled.',
        'One usable transfer port remains. No explosion, attack direction, or exact failure mechanism is established.',
        'Unscaled shape proposal. No capacity, measurements, final livery, or Gantry hull fate is selected.',
    ]):
        art += text(42,975+n*28,line,17,INK)
    return document('Gantry: intact and stranded comparison','Two matching views of Gantry, each before and after localized service-area damage. The hull and surviving port remain the same. Private story staging, not public history or an engineering simulation.',art,1100)


def crew_sheet(scheme):
    """Show each authored likeness and body with color-only presentation changes."""
    art = rect(0,0,1500,1000,PAPER)
    art += text(40,48,'GANTRY / THE WORKING CREW',30,FOLIO['title'],letter_spacing=1)
    art += text(40,82,f'{scheme.upper()} COLORS / STARTING IDENTITIES / NO INJURY STAGING',15,FOLIO['muted'],letter_spacing=1)
    for n,(name,full_name,role) in enumerate(CREW):
        x = 30+500*n
        clip = f'{name}-clip'
        art += tag('defs',tag('clipPath',rect(x,122,440,685,'white'),id=clip))
        drawing = rect(x,122,440,685,WORK['wall'])+group(gantry_portrait(name),f'translate({x-42} 170) scale(1.65)')
        art += present(group(drawing,clip_path=f'url(#{clip})'),scheme,name)
        art += rect(x,122,440,685,'none',FOLIO['muted'],1)
        art += text(x+15,851,full_name,28,INK,letter_spacing=1)
        art += text(x+15,886,role,19,FOLIO['title'])
    art += text(40,955,'Frontal head identities and separate civilian bodies. Appearance, clothing, and colors remain proposals.',17,INK)
    return document(f'Gantry crew: {scheme} color study','Nadia Sen, Owen Park, and Ivo Marin with original frontal heads, separately authored work clothes, and the shared color presentation. No ages, biographies, rank, or later injuries are established.',art,1000)


def review():
    """Link public starting-identity exports without copying private damage into the site."""
    theme = ';'.join(f'--{k}:{v}' for k,v in REVIEW.items())
    cards = [('gantry-conditions.svg','One ship before and after','Same views and scale; the forward transfer collar and crew module survive.'),('crew-comic.svg','People in comic color','Original frontal heads and separate work clothes, not portraits derived from injury.'),('crew-lore.svg','The same people in lore color','Only the color treatment changes. Labels and geometry stay separate.')]
    sections = ''.join(f'<section><h2>{title}</h2><a href="{file}"><img src="{file}" alt="{title}" width="1500" height="{1100 if file.startswith("gantry") else 1000}"></a><p>{note} <a href="{file}">Open full-size SVG</a>.</p></section>' for file,title,note in cards)
    links = ''.join(f'<li><a href="../../../web/src/assets/lore/{name}">{label}</a></li>' for name,label in [('gantry-design-concept.svg','Gantry: intact design sheet'),('nadia-sen-portrait-concept.svg','Nadia portrait'),('owen-park-portrait-concept.svg','Owen portrait'),('ivo-marin-portrait-concept.svg','Ivo portrait')])
    return f'''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Gantry and its crew / Private design review</title><style>
:root{{{theme};color-scheme:dark}}*{{box-sizing:border-box}}body{{margin:0;background:var(--background);color:var(--text);font:17px/1.65 system-ui,sans-serif}}main{{max-width:1500px;margin:auto;padding:28px}}a{{color:var(--mint)}}a:focus-visible{{outline:3px solid var(--focus)}}img{{display:block;width:100%;height:auto}}section{{margin:44px 0}}p{{max-width:85ch}}h1,h2{{line-height:1.2}}@media(max-width:600px){{main{{padding:16px}}}}@media print{{section{{break-before:page}}}}
</style></head><body><main><p>PRIVATE DESIGN REVIEW / NOT A RELEASED EPISODE</p><h1>Gantry and its crew</h1><p>Reusable art proposals for the Aquila pickup and later rescue. Public lore gets only the intact ship and starting portraits. No new dimensions, biographies, injury details, or story outcomes are selected.</p><p><a href="../episode-1/index.html">Episode script</a> / <a href="README.md">Source and review notes</a></p>{sections}<h2>Public lore exports</h2><ul>{links}</ul></main></body></html>\n'''


def main():
    """Write four private review files, or check their exact regeneration."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    outputs = {'gantry-conditions.svg':conditions(),'crew-comic.svg':crew_sheet('comic'),'crew-lore.svg':crew_sheet('lore'),'index.html':review()}
    for name,content in outputs.items():
        target = OUTPUT/name
        if args.check:
            if not target.is_file() or target.read_text()!=content:
                raise SystemExit(f'Out of date: {target}')
        else:
            target.write_text(content)
    print(f"{'Verified' if args.check else 'Rendered'} Gantry condition and crew studies")


if __name__=='__main__':
    main()
