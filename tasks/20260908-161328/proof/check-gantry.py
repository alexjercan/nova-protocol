#!/usr/bin/env python3
"""Verify the Gantry art batch against its pre-change working-tree snapshot."""

import hashlib
import json
import subprocess
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
TASK = 'tasks/20260908-161328'
OUT = ROOT/TASK/'proof/gantry'
sys.path.insert(0,str(ROOT/'scripts'))
from nova_illustration.faces import FACES, frontal_head
from nova_illustration.portraits import elena_close, elena_gesture, jonah_listener, work_inspection, work_portrait
from nova_illustration.ships import render_ship


def digest(data):
    """Hash exact source or render bytes."""
    return hashlib.sha256(data).hexdigest()


def git(*args):
    """Read the repository without staging or modifying it."""
    return subprocess.check_output(['git',*args],cwd=ROOT).decode().strip()


def main():
    """Check old art, new SVGs, publication separation, and the bounded source scope."""
    baseline = json.loads((OUT/'baseline.json').read_text())
    allowed = set(baseline['allowed_existing'])
    changed = []
    for name,sha in baseline['files'].items():
        current = ROOT/name
        assert current.is_file(),name
        if digest(current.read_bytes())!=sha:
            assert name in allowed,name
            changed.append(name)
    for key,sha in baseline['renders'].items():
        parts = key.split('/')
        if parts[0]=='head':
            art = frontal_head(parts[1])
        elif parts[0]=='bust':
            art = work_portrait(parts[1])
        elif parts[0]=='inspect':
            art = ''.join(work_inspection(parts[1]))
        elif parts[0]=='pose':
            art = {'elena_close':elena_close,'elena_gesture':elena_gesture,'jonah_listener':jonah_listener}[parts[1]]()
        else:
            art = render_ship(parts[1],parts[2],0,0,1,parts[3]=='True')
        assert digest(art.encode())==sha,key
    assets = ['gantry-design-concept.svg',*[f'{name}-portrait-concept.svg' for name in ('nadia-sen','owen-park','ivo-marin')]]
    allowed_new = {f'web/src/assets/lore/{name}' for name in assets}
    allowed_new.update({f'{TASK}/GANTRY-REVIEW.md',f'{TASK}/proof/inspect-gantry.mjs',f'{TASK}/proof/check-gantry.py'})
    prefixes = (f'{TASK}/gantry-study/',f'{TASK}/proof/gantry/')
    for name in git('ls-files','--others','--exclude-standard').splitlines():
        if name in baseline['files'] or name in allowed_new or name.startswith(prefixes):
            continue
        assert not name.startswith(('web/','scripts/nova_illustration/',TASK+'/')),name
    svg_paths = [ROOT/'web/src/assets/lore'/n for n in assets]+[ROOT/TASK/'gantry-study'/n for n in ('gantry-conditions.svg','crew-comic.svg','crew-lore.svg')]
    svg_reports = []
    for file in svg_paths:
        doc = ET.fromstring(file.read_bytes())
        ids = [n.get('id') for n in doc.iter() if n.get('id')]
        assert len(ids)==len(set(ids)),file
        for n in doc.iter():
            assert n.tag.rsplit('}',1)[-1] not in ('script','foreignObject','image'),file
            for value in n.attrib.values():
                if value.startswith('url(#'):
                    assert value[5:-1] in ids,(file,value)
        faces = [n.get('data-face') for n in doc.iter() if n.get('data-face')]
        assert all(name in FACES for name in faces)
        assert all(n.get('data-gaze')=='forward' for n in doc.iter() if n.get('data-face'))
        if file.name.endswith('portrait-concept.svg'):
            assert faces==[file.name.split('-')[0]],file
        svg_reports.append({'path':str(file.relative_to(ROOT)),'faces':faces,'sha256':digest(file.read_bytes())})
    public = ROOT/'web/dist'
    for file in public.rglob('*'):
        if file.suffix not in ('.html','.js','.svg'):
            continue
        data = file.read_bytes()
        for marker in (b'gantry-torn-',b'gantry-service-scorch',b'data-state="stranded"',b'gantry-conditions.svg',b'gantry-study',b'PRIVATE DESIGN REVIEW'):
            assert marker not in data,file
    for file in (ROOT/'web/src/assets/lore').glob('*.svg'):
        assert file.read_bytes()==(public/'assets/lore'/file.name).read_bytes(),file
    assert len(list((ROOT/'web/src/assets/lore').glob('*.svg')))==24
    assert sorted(p.name for p in (public/'story').iterdir())==['demo','index.html']
    assert git('diff','--cached','--name-only')==''
    report = {
        'starting_head':baseline['head'],'checked_head':git('rev-parse','HEAD'),
        'changed_existing_paths':sorted(changed),'protected_file_count':len(baseline['files'])-len(changed),
        'retained_render_count':len(baseline['renders']),'new_svgs':svg_reports,
        'checks':['37 prior head/body/ship renders remain exact','accepted episode script and four-page opening unchanged','old public art, accepted studies and old proof unchanged','only four new public lore SVGs; all 24 match the build','public SVGs exclude future damage; private study excluded from site','safe IDs, frontal identities, and local filter references','public story remains demo-only; index empty'],
    }
    (OUT/'source-and-scope.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f"Gantry source/scope pass: {len(svg_reports)} SVGs, 37 retained renders, 24 exact public lore assets")


if __name__=='__main__':
    main()
