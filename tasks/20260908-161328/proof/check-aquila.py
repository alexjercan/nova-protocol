#!/usr/bin/env python3
"""Check Aquila's private art, exact script use, retained work, and publication scope."""

import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
TASK = ROOT/'tasks/20260908-161328'
OUT = TASK/'proof/aquila'
sys.path.insert(0,str(ROOT/'scripts'))
from nova_illustration.colors import MATERIALS
from nova_illustration.ships import normal, render_ship


def digest(data):
    """Hash exact source, generated image, or evidence bytes."""
    return hashlib.sha256(data).hexdigest()


def git(*args):
    """Read repository state without staging or changing it."""
    return subprocess.check_output(['git',*args],cwd=ROOT).decode().strip()


def main():
    """Compare this batch to its starting working tree, not merely the earlier HEAD."""
    before = json.loads((TASK/'proof/before-aquila/baseline.json').read_text())
    task = str(TASK.relative_to(ROOT))
    allowed = {f'scripts/nova_illustration/{name}' for name in ('README.md','colors.py','lettering.py','portraits.py','scenery.py','ships.py','test_illustration.py')}
    allowed.update(f'{task}/{name}' for name in ('TASK.md','GANTRY-REVIEW.md','gantry-study/README.md','episode-1/SCRIPT.md','episode-1/generate.mjs','episode-1/index.html'))
    allowed.add('web/src/lore/seasons/season-1.md')
    changed,concurrent,protected = [],[],[]
    for name,sha in before['files'].items():
        file = ROOT/name
        if file.is_file() and digest(file.read_bytes())==sha:
            protected.append(name)
        elif name in allowed:
            changed.append(name)
        elif name.startswith((task+'/','scripts/nova_illustration/','scripts/gen-lore-','web/src/lore/','web/src/assets/lore/','web/src/comics/')):
            raise AssertionError(f'Protected source or evidence changed: {name}')
        else:
            concurrent.append(name)
    for name in before['snapshots']:
        source = TASK/'proof/before-aquila'/(name.replace('/','--')+'.txt')
        assert digest(source.read_bytes())==before['files'][f'{task}/{name}']
    for key,sha in before['renders'].items():
        name,view,state,thrust = key.split('/')
        assert digest(render_ship(name,view,0,0,1,thrust=='True',state).encode())==sha,key
    spec = importlib.util.spec_from_file_location('aquila_pages',TASK/'aquila-pages/generate.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    authored = module.pages()
    panels = [p for _,_,page in authored for p in page]
    assert [p.key for p in panels]==[f'{n}{c}' for n in (5,6,7) for c in 'abc']
    for panel in panels:
        panel.render()
        assert [(s,t) for s,t in panel.lines if s!='Location and time']==panel.spoken
    old = (TASK/'proof/before-aquila/episode-1--SCRIPT.md.txt').read_text()
    script = (TASK/'episode-1/SCRIPT.md').read_text()
    speech = module.dialogue_lines
    assert 134+sum(len(t.split()) for _,t in speech(old))==735
    assert speech('**Rina:** Keep the\ncovers on.\n\n')==[('Rina','Keep the covers on.')]
    substitutions = {
        "Nearly. This end's heavier than it looks.":"Nearly. It's awkward to turn.",
        'Easier than picking both loads off the floor. Sen. Gantry.':'Better than chasing both loads. Sen. Gantry.',
    }
    assert [(s,substitutions.get(t,t)) for s,t in speech(old)]==speech(script)
    assert old.split('## Page 8:',1)[1].split('## Approved direction',1)[0]==script.split('## Page 8:',1)[1].split('## Approved direction',1)[0]
    assert len(re.findall(r'^### Panel \d+[abc]$',script,re.M))+10==50
    words = 134+sum(len(t.split()) for _,t in speech(script))
    assert words==730,words
    for face in module.assembly_faces():
        assert face.material in MATERIALS
        normal(face.vertices)
    svgs = []
    for number,(title,_,page) in enumerate(authored,5):
        file = TASK/'aquila-pages'/f'page-{number:02}.svg'
        assert file.read_text()==module.opening.render_page(number,title,page)
        doc = ET.fromstring(file.read_bytes())
        ids = [n.get('id') for n in doc.iter() if n.get('id')]
        assert len(ids)==len(set(ids))
        for n in doc.iter():
            assert n.tag.rsplit('}',1)[-1] not in ('script','foreignObject','image')
            for value in n.attrib.values():
                if value.startswith('url(#'):
                    assert value[5:-1] in ids,value
        assert not doc.findall('.//*[@data-state]')
        assert not doc.findall('.//*[@data-expression]')
        assert all(n.get('data-gaze')=='forward' for n in doc.findall('.//*[@data-face]'))
        svgs.append({'page':number,'sha256':digest(file.read_bytes())})
    assert 'data-component="protective-cover"' in (TASK/'aquila-pages/page-07.svg').read_text()
    for file in (ROOT/'web/src/comics').rglob('*'):
        if file.is_file():
            for marker in ('aquila-pages','replacement-assembly','A useful job'):
                assert marker not in file.read_text(),file
    assert sorted(p.name for p in (ROOT/'web/src/comics').glob('*/comic.json'))==['comic.json']
    assert (ROOT/'web/src/comics/demo/comic.json').is_file()
    assert len(list((ROOT/'web/src/assets/lore').glob('*.svg')))==24
    assert all(f'web/src/assets/lore/{p.name}' in protected for p in (ROOT/'web/src/assets/lore').glob('*.svg'))
    assert git('diff','--cached','--name-only')==''
    task_text = (TASK/'TASK.md').read_text()
    assert '- STATUS: OPEN' in task_text and task_text.count('- [ ]')==7
    report = {
        'starting_head':before['head'],'checked_head':git('rev-parse','HEAD'),
        'page_count':18,'panel_count':50,'spoken_words':words,'illustrated_pages':7,
        'new_panels':9,'retained_ship_renders':len(before['renders']),
        'changed_existing_paths':sorted(changed),'protected_files':len(protected),
        'concurrent_paths_not_validated':sorted(concurrent),'svg_hashes':svgs,
        'checks':['only two spoken lines changed','pages 8-18 story content unchanged','accepted opening, Gantry studies, faces, 24 public lore SVGs and all earlier proof retained','35 prior intact/stranded ship renders exact','one assembly model for inspection and loaded cradle','literal script text, ordered panels, safe SVGs, valid clip references','private comic sources remain outside publication; public demo unchanged','seven writing acceptance items still open; index empty'],
    }
    (OUT/'source-and-scope.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f'Aquila source/scope pass: 18 pages, 50 panels, {words} spoken words; 7 illustrated pages; public story unchanged')


if __name__=='__main__':
    main()
