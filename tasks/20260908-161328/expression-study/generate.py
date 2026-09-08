#!/usr/bin/env python3
"""Compare the page-1 expression trials using the maintained character drawings."""

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0,str(ROOT/'scripts'))
from nova_illustration.colors import INK, PAPER, INTERIOR, FOLIO
from nova_illustration.portraits import jonah_listener, work_portrait
from nova_illustration.styles import SCHEMES, present
from nova_illustration.svg import group, rect, tag, text


TRIALS = (
    ('rina','original','The retained features, for comparison.'),
    ('rina','amused','Softer eyes and brows; a small closed-mouth smile.'),
    ('jonah','original','The retained features, for comparison.'),
    ('jonah','wry','One raised brow and an uneven half-smile.'),
)


def render(scheme):
    """Keep paired poses and labels fixed; the scheme only recolors the drawings."""
    art = rect(0,0,1500,1000,PAPER)
    art += text(42,48,'NOVA PROTOCOL / EXPRESSION TRIAL',21,FOLIO['title'],font_weight='bold',letter_spacing=2)
    art += text(42,93,'Original likenesses. Different reactions. The same reusable sources as page 1.',19,INK)
    for i,(name,expression,note) in enumerate(TRIALS):
        x,y = 42+(i%2)*728,135+(i//2)*411
        key = f'expression-{name}-{expression}'
        pose = work_portrait(name,expression) if name=='rina' else jonah_listener(expression)
        scene = rect(0,0,688,342,INTERIOR['wall_light'])
        scene += group(pose,'translate(182 36) scale(.97)')
        art += tag('defs',tag('clipPath',rect(0,0,688,342,'white'),id=key))
        clipped = group(present(scene,scheme,key),clip_path=f'url(#{key})')
        label = text(20,32,f'{name.title()} / {expression}',22,INK,font_weight='bold')
        art += group(clipped+rect(0,0,688,342,'none',INK,2)+label,f'translate({x} {y})')
        art += text(x,y+377,note,17,FOLIO['title'])
    art += text(42,970,'PRIVATE EXPRESSION STUDY / HEADS, HAIR, NECKS, AND BODY POSES RETAINED',12,FOLIO['muted'],letter_spacing=1)
    return tag('svg',tag('title','Rina and Jonah: original and expression variants',id='title')+tag('desc','Rina tries an amused expression. Jonah tries a dry half-smile. Each is compared with the original features on the same body pose.',id='description')+art,xmlns='http://www.w3.org/2000/svg',width=1500,height=1000,viewBox='0 0 1500 1000',role='img',aria_labelledby='title description')+'\n'


def main():
    """Write the two private comparison sheets, or check their deterministic bytes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args = parser.parse_args()
    for scheme in SCHEMES:
        target = OUTPUT/f'expressions-{scheme}.svg'
        data = render(scheme).encode('utf-8')
        if args.check:
            if not target.is_file() or target.read_bytes()!=data:
                raise SystemExit(f'Out of date: {target}')
        else:
            target.write_bytes(data)
    print(f"{'Verified' if args.check else 'Rendered'} two private expression comparison sheets")


if __name__=='__main__':
    main()
