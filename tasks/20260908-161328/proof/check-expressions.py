#!/usr/bin/env python3
"""Check the expression-only trial against the accepted first-picture-card revision."""

import hashlib
import json
import re
import subprocess
import sys
import types
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
TASK = 'tasks/20260908-161328'
OUTPUT = ROOT/TASK/'proof/expressions'
BASE = 'b716d27b0618b914885cf512bc6bfb2b7463b6f2'
sys.dont_write_bytecode = True
sys.path.insert(0,str(ROOT/'scripts'))
from nova_illustration import faces, portraits
from nova_illustration.expressions import facial_features


def git(*args):
    """Read the repository without staging or changing its contents."""
    return subprocess.check_output(['git',*args],cwd=ROOT)


def prior(path):
    """Read a source or generated asset from the approved comparison revision."""
    return git('show',f'{BASE}:{path}')


def baseline_module(name):
    """Load the old local drawing definitions without running an exporter."""
    key = f'nova_illustration._expression_baseline_{name}'
    module = types.ModuleType(key)
    module.__package__ = 'nova_illustration'
    module.__file__ = str(ROOT/f'scripts/nova_illustration/{name}.py')
    sys.modules[key] = module
    exec(compile(prior(f'scripts/nova_illustration/{name}.py'),module.__file__,'exec'),module.__dict__)
    return module


def main():
    """Write bounded proof; compare art, source ownership, and unchanged defaults."""
    original_faces = baseline_module('faces')
    original_portraits = baseline_module('portraits')
    defaults = {}
    for name in faces.FACES:
        current = faces.frontal_head(name)
        assert current == original_faces.frontal_head(name), name
        defaults[name] = hashlib.sha256(current.encode()).hexdigest()
    for name in ('elena_close','elena_gesture','jonah_listener'):
        assert getattr(portraits,name)() == getattr(original_portraits,name)(), name
    for name in ('leila','rina','tomas'):
        assert portraits.work_portrait(name) == original_portraits.work_portrait(name), name

    comic = f'{TASK}/comic-opening-poc'
    old_page = prior(f'{comic}/page-01.svg').decode()
    new_page = (ROOT/comic/'page-01.svg').read_text()
    restored = new_page
    for name,expression in [('rina','amused'),('jonah','wry')]:
        new_head = faces.frontal_head(name,expression)
        assert restored.count(new_head) == 1
        restored = restored.replace(new_head,faces.frontal_head(name),1)
        features = facial_features(name,expression)
        identity = new_head.replace(features,'FEATURES',1).replace(f' data-expression="{expression}"','')
        assert identity == faces.frontal_head(name).replace(faces.FACES[name].features,'FEATURES',1)
    assert restored == old_page, 'Page 1 changes only the two facial feature layers'
    old_board = prior(f'{comic}/index.html').decode()
    new_board = (ROOT/comic/'index.html').read_text()
    assert old_board.count(old_page.strip()) == new_board.count(new_page.strip()) == 1
    assert new_board == old_board.replace(old_page.strip(),new_page.strip(),1)
    for n in range(2,5):
        path = f'{comic}/page-0{n}.svg'
        assert (ROOT/path).read_bytes() == prior(path), path

    protected = [
        'web/src', 'web/webpack.config.js', 'web/comic-build.js',
        'scripts/gen-lore-designs.py', 'scripts/gen-lore-portraits.py',
        *[f'scripts/nova_illustration/{name}.py' for name in ('__init__','colors','ships','styles','scenery','svg','lettering')],
        f'{TASK}/clean-line-study', f'{TASK}/opening_props.py',
        *[f'{TASK}/proof/{name}' for name in ('baseline','revised','before-clean-line-opening','restyled','first-panel-card','clean-line','clean-line-initial','clean-line-before-frontal')],
    ]
    assert not git('diff','--name-only',BASE,'--',*protected), 'Protected sources and prior evidence stay unchanged'
    public_art = {}
    for path in git('ls-tree','-r','--name-only',BASE,'--','web/src/assets/lore').decode().splitlines():
        current = (ROOT/path).read_bytes()
        assert current == prior(path), path
        public_art[path] = hashlib.sha256(current).hexdigest()

    svg_reports = []
    svgs = sorted((ROOT/comic).glob('page-*.svg'))+sorted((ROOT/TASK/'expression-study').glob('*.svg'))
    assert len(svgs) == 6
    for path in svgs:
        root = ET.parse(path).getroot()
        ids = [n.get('id') for n in root.iter() if 'id' in n.attrib]
        assert len(ids) == len(set(ids)), path
        for node in root.iter():
            assert node.tag.rsplit('}',1)[-1] not in ('script','foreignObject','image')
            if 'data-face' in node.attrib:
                assert node.get('data-gaze') == 'forward'
        for target in re.findall(r'url\(#([^)]+)\)',path.read_text()):
            assert target in ids, (path,target)
        svg_reports.append(str(path.relative_to(ROOT)))

    allowed = {
        *[f'scripts/nova_illustration/{name}' for name in ('faces.py','expressions.py','portraits.py','test_illustration.py','README.md')],
        *[f'{TASK}/{name}' for name in ('TASK.md','EXPRESSION-REVIEW.md','comic-opening-poc/generate.py','comic-opening-poc/README.md','comic-opening-poc/index.html','comic-opening-poc/page-01.svg','proof/inspect.mjs','proof/check-expressions.py')],
        *[f'{TASK}/expression-study/{name}' for name in ('generate.py','README.md','expressions-comic.svg','expressions-lore.svg')],
    }
    changed = set(git('diff','--name-only','HEAD').decode().splitlines())
    changed.update(git('ls-files','--others','--exclude-standard').decode().splitlines())
    excluded = []
    for path in sorted(changed):
        if path == '.scufris.toml' or (path.startswith('tasks/') and not path.startswith(TASK+'/')):
            excluded.append(path)
        else:
            assert path in allowed or path.startswith(f'{TASK}/proof/expressions/'), path
    assert not git('diff','--cached','--name-only'), 'No files staged by this trial'
    OUTPUT.mkdir(parents=True,exist_ok=True)
    report = {
        'baseline': BASE,
        'checked_head': git('rev-parse','HEAD').decode().strip(),
        'checks': ['five default heads and all body helpers byte-identical',
                   'only page-1 Rina/Jonah features change; retained likenesses match',
                   'board only substitutes page-1 SVG; pages 2-4 byte-identical',
                   'public sources, exporters, accepted study and earlier evidence unchanged',
                   'six SVGs parse, use safe elements, and have unique resolved IDs',
                   'bounded working changes; empty index'],
        'default_head_sha256': defaults,
        'public_art_sha256': public_art,
        'svg_paths': svg_reports,
        'protected_paths': protected,
        'excluded_unrelated_paths_not_validated': excluded,
    }
    (OUTPUT/'source-and-scope.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f'Expression-only scope passes: five unchanged default heads, {len(public_art)} unchanged public assets, six valid SVGs')


if __name__=='__main__':
    main()
