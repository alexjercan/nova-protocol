#!/usr/bin/env python3
"""Generate lore ship-design and portrait proposals from the reusable art library."""

import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True
from nova_illustration.colors import LORE
from nova_illustration.portraits import elena_close, gantry_portrait
from nova_illustration.ships import bounds, render_ship
from nova_illustration.styles import present
from nova_illustration.svg import group, path, rect, tag, text

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'web/src/assets/lore'


def document(title, description, art, width, height):
    """Wrap one accessible, self-contained SVG export."""
    return tag('svg',tag('title',title,id='title')+tag('desc',description,id='description')+art,xmlns='http://www.w3.org/2000/svg',width=width,height=height,viewBox=f'0 0 {width} {height}',role='img',aria_labelledby='title description')+'\n'


def ship_sheet(name, scheme):
    """Use one model for aligned orthographic views and a three-quarter study."""
    views = [('top','PLAN',360,302),('side','SIDE',1070,302),('front','FORWARD',360,720),('aft-quarter','AFT THREE-QUARTER',1070,720)]
    art = rect(0,0,1500,1100,LORE['background'])
    for x in range(30,1500,30):
        art += path(f'M{x} 115V1000',stroke=LORE['grid'],width=.6)
    for y in range(130,1000,30):
        art += path(f'M30 {y}H1470',stroke=LORE['grid'],width=.6)
    art += text(42,47,'NOVA PROTOCOL / EXTERNAL FORM STUDY',15,LORE['label'],letter_spacing=2)
    art += text(42,91,name.upper(),37,LORE['title'],letter_spacing=4,font_weight='bold')
    art += text(1458,87,'DESIGN PROPOSAL / NOT AN ENGINEERING DRAWING',13,LORE['label'],text_anchor='end')
    scale = min(min(600/(bounds(name,v)[2]-bounds(name,v)[0]),260/(bounds(name,v)[3]-bounds(name,v)[1])) for v,_,_,_ in views[:3])
    for view,label,x,y in views:
        art += rect(x-318,y-163,636,339,'none',LORE['border'],1)
        art += path(f'M{x-308} {y}H{x+308}M{x} {y-130}V{y+140}',stroke=LORE['centerline'],width=.8,stroke_dasharray='7 6')
        art += text(x-299,y-138,label,16,LORE['label'],letter_spacing=2)
        lo_x,lo_y,hi_x,hi_y = bounds(name,view)
        s = scale if view!='aft-quarter' else min(565/(hi_x-lo_x),253/(hi_y-lo_y))
        drawing = render_ship(name,view,x-(lo_x+hi_x)*s/2,y-(lo_y+hi_y)*s/2+11,s,False)
        art += present(drawing,scheme,f'{name}-{view}')
    art += path('M42 938H1458',stroke=LORE['border'],width=1)
    notes = {
        'kaveri': ['Raised crew module / side transfer collar','Open work cradle / stowed handling arm','Paired aft engine housings / unarmed'],
        'ebro': ['Three framed pressure-vessel forms','Forward crew module / side transfer collar','Shared industrial detailing / unarmed'],
        'gantry': ['Broad plated cargo body / raised handling frame','Forward crew module / side transfer collar','Single aft engine housing / unarmed'],
    }[name]
    for n,note in enumerate(notes):
        art += text(44,970+n*27,note,17,LORE['label'])
    art += text(1458,984,'ALL VIEWS SHARE ONE MODEL',14,LORE['label'],text_anchor='end',letter_spacing=1)
    art += text(1458,1012,'NO PHYSICAL SCALE OR DIMENSIONS APPROVED',12,LORE['muted'],text_anchor='end')
    art += text(44,1080,'CONCEPT GEOMETRY AND PRESENTATION COLORS ONLY. NO LOAD, CAPACITY, OR PERFORMANCE VALUES IMPLIED.',12,LORE['muted'],letter_spacing=.6)
    return document(f'{name.title()}: ship design proposal','Plan, side, forward, and three-quarter views from one unscaled model. Structural forms and colors are proposals, not approved physical specifications.',art,1500,1100)


def elena_portrait(scheme):
    """Export the exact close-up pose used by the scene library."""
    art = rect(0,0,700,840,LORE['background'])
    art += tag('defs',tag('clipPath',rect(35,100,630,606,'white'),id='portrait-clip'))
    drawing = rect(35,100,630,606,LORE['portrait_background'])+group(elena_close(),'translate(154 139) scale(1.64)')
    art += present(group(drawing,clip_path='url(#portrait-clip)'),scheme,'elena-portrait')
    art += rect(35,100,630,606,'none',LORE['border'],1)
    art += text(35,57,'NOVA PROTOCOL / PORTRAIT CONCEPT',15,LORE['label'],letter_spacing=1.5)
    art += text(35,758,'ELENA WARD',33,LORE['title'],letter_spacing=2)
    art += text(35,793,'Appearance and clothing remain proposals.',16,LORE['label'])
    return document('Elena Ward: portrait concept','A forward-facing portrait study of Elena Ward. The comic uses this drawing in full color; this lore export applies a green presentation treatment. Appearance and clothing are provisional.',art,700,840)


CREW = {'nadia': ('Nadia Sen','CAPTAIN / PILOT'), 'owen': ('Owen Park','ENGINEER'), 'ivo': ('Ivo Marin','WORK OPERATIONS')}


def crew_portrait(name, scheme):
    """Export a starting-identity portrait, never a future injury or scene outcome."""
    full_name,role = CREW[name]
    art = rect(0,0,700,840,LORE['background'])
    art += tag('defs',tag('clipPath',rect(35,100,630,606,'white'),id='portrait-clip'))
    drawing = rect(35,100,630,606,LORE['portrait_background'])+group(gantry_portrait(name),'translate(68 140) scale(1.7)')
    art += present(group(drawing,clip_path='url(#portrait-clip)'),scheme,f'{name}-portrait')
    art += rect(35,100,630,606,'none',LORE['border'],1)
    art += text(35,57,'NOVA PROTOCOL / PORTRAIT CONCEPT',15,LORE['label'],letter_spacing=1.5)
    art += text(35,751,full_name.upper(),33,LORE['title'],letter_spacing=2)
    art += text(35,784,role,15,LORE['label'],letter_spacing=1)
    art += text(35,814,'Appearance and clothing remain proposals.',14,LORE['label'])
    return document(f'{full_name}: portrait concept',f'A forward-facing civilian workship portrait study of {full_name}. Head and body are reusable in comic color; this export applies the lore color treatment. No age, biography, rank, or final likeness is established.',art,700,840)


def main():
    """Regenerate the seven named lore assets, or verify them without writes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    outputs = {f'{name}-design-concept.svg':ship_sheet(name,'lore') for name in ('kaveri','ebro','gantry')}
    outputs['elena-ward-portrait-concept.svg'] = elena_portrait('lore')
    outputs.update({full_name.lower().replace(' ','-')+'-portrait-concept.svg':crew_portrait(name,'lore') for name,(full_name,_) in CREW.items()})
    for filename,content in outputs.items():
        target = OUTPUT/filename
        encoded = content.encode('utf-8')
        if args.check:
            if not target.is_file() or target.read_bytes()!=encoded:
                raise SystemExit(f'Out of date: {target}')
        else:
            target.write_bytes(encoded)
    print(f"{'Verified' if args.check else 'Rendered'} three ship design sheets and four portraits")


if __name__=='__main__':
    main()
