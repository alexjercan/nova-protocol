#!/usr/bin/env python3
"""Check this restyle against the committed comparison revision and frozen draft."""

import ast
import hashlib
import json
import re
import subprocess
import sys
import types
from pathlib import Path
from xml.etree import ElementTree as ET

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
TASK = ROOT/'tasks/20260908-161328'
BASE = '7e8095fd822dc6264806165aac3567235732ba1e'
ARCHIVE = TASK/'proof/before-clean-line-opening'
LIVE = TASK/'comic-opening-poc/generate.py'


def git(*args):
    """Inspect the recorded comparison revision without changing the index."""
    return subprocess.check_output(['git',*args],cwd=ROOT)


def load(source, name):
    """Evaluate a generator at its original location, without running its CLI."""
    module = types.ModuleType(name)
    module.__file__ = str(LIVE)
    sys.modules[name] = module
    exec(compile(source,str(LIVE),'exec'),module.__dict__)
    return module


def main():
    """Verify continuity, frozen/public assets, private discovery, palette, and scope."""
    records = json.loads((ARCHIVE/'sources.json').read_text())
    assert len(records)==7
    for record in records.values():
        assert hashlib.sha256((ROOT/record['copy']).read_bytes()).hexdigest()==record['sha256']
    old = load((ARCHIVE/'generate.py.txt').read_text(),'opening_before').pages()
    new = load(LIVE.read_text(),'opening_after').pages()
    assert [page[0] for page in old]==[page[0] for page in new]
    assert [[line for panel in page[2] for line in panel.lines] for page in old]==[[line for panel in page[2] for line in panel.lines] for page in new]
    assert new[0][2][0].lines[0][0]=='Location and time'
    assert all(speaker!='Location and time' for speaker,_ in new[0][2][1].lines)
    assert [len(page[2]) for page in new]==[2,3,3,2]
    nonspoken = {'Location and time','Display','Delivery record'}
    words = sum(len(line.split()) for _,_,panels in new for p in panels for speaker,line in p.lines if speaker not in nonspoken)
    assert words==130,words

    protected = git('ls-tree','-r','--name-only',BASE,'web/src/assets/lore').decode().splitlines()
    protected += [f'scripts/nova_illustration/{name}.py' for name in ('ships','styles','scenery','lettering')]
    protected += [f'tasks/20260908-161328/clean-line-study/{name}.svg' for name in ('01-close-up','02-window','03-exterior')]
    for filename in protected:
        assert (ROOT/filename).read_bytes()==git('show',f'{BASE}:{filename}'),filename

    for p in sorted((TASK/'comic-opening-poc').glob('*.svg')):
        root = ET.fromstring(p.read_bytes())
        ids = [node.get('id') for node in root.iter() if node.get('id')]
        assert len(ids)==len(set(ids)),p
        for node in root.iter():
            assert node.tag.rsplit('}',1)[-1] not in ('script','foreignObject','image'),p
            for value in node.attrib.values():
                assert all(key in ids for key in re.findall(r'url\(#([^)]*)\)',value)),(p,value)
        heads = [n for n in root.iter() if n.get('data-face')]
        assert all(n.get('data-gaze')=='forward' for n in heads)

    palette_sources = [LIVE,TASK/'opening_props.py',TASK/'clean-line-study/generate.py']
    for p in palette_sources:
        for node in ast.walk(ast.parse(p.read_text())):
            if isinstance(node,ast.Constant) and isinstance(node.value,str):
                assert not re.search(r'#[0-9a-fA-F]{6}\b',node.value),(p,node.lineno)

    catalog = json.loads(subprocess.check_output(['node','-e',"console.log(JSON.stringify(require('./web/comic-build').discoverComics().map(c=>c.path)))"],cwd=ROOT))
    assert catalog==['demo'],catalog
    site = ROOT/'web/dist'
    assert sorted(p.name for p in (site/'story').iterdir())==['demo','index.html']
    for p in site.rglob('*'):
        if p.suffix in ('.html','.js'):
            body = p.read_text()
            for private in ('tasks/20260908-161328','OPENING / CLEAN-LINE DRAFT','before-clean-line-opening','Baikal, before the trouble.'):
                assert private not in body,(p,private)
    for p in (ROOT/'web/src/assets/lore').glob('*.svg'):
        assert (site/'assets/lore'/p.name).read_bytes()==p.read_bytes(),p

    task_files = [
        'TASK.md','REVIEW.md','STYLE-REVIEW.md','COMMIT-REVIEW.md','RESTYLE-REVIEW.md','opening_props.py',
        'clean-line-study/README.md','clean-line-study/generate.py','clean-line-study/index.html',
        'comic-opening-poc/README.md','comic-opening-poc/generate.py','comic-opening-poc/index.html',
        'comic-opening-poc/page-01.svg','comic-opening-poc/page-02.svg','comic-opening-poc/page-03.svg','comic-opening-poc/page-04.svg',
        'proof/inspect.mjs','proof/check-opening-restyle.py',
    ]
    allowed = {'scripts/gen-lore-portraits.py','web/src/lore/README.md','web/src/lore/seasons/season-1.md'}
    allowed.update(f'scripts/nova_illustration/{name}' for name in ('README.md','colors.py','faces.py','portraits.py','test_illustration.py'))
    allowed.update(f'tasks/20260908-161328/{name}' for name in task_files)
    for folder in ('before-clean-line-opening','restyled','first-panel-card'):
        allowed.update(str(p.relative_to(ROOT)) for p in (TASK/'proof'/folder).rglob('*') if p.is_file())
    changed = set(git('diff',BASE,'--no-renames','--name-only').decode().splitlines())
    changed.update(git('ls-files','--others','--exclude-standard').decode().splitlines())
    concurrent = {'.scufris.toml'}
    for filename in concurrent:
        assert (ROOT/filename).read_bytes()==git('show',f'HEAD:{filename}'),filename
    changed -= concurrent
    other_tasks = sorted(p for p in changed if p.startswith('tasks/') and not p.startswith('tasks/20260908-161328/'))
    changed.difference_update(other_tasks)
    assert not changed-allowed,sorted(changed-allowed)
    assert not git('diff','--cached','--name-only')
    subprocess.run(['git','diff','--check'],cwd=ROOT,check=True)
    report = {
        'baseline':BASE,'checked_head':git('rev-parse','HEAD').decode().strip(),
        'frozen_original_files':len(records),'pages':4,'panels':10,'spoken_words':words,
        'all_dialogue_display_record_and_card_text_unchanged':True,
        'baikal_card_in_first_panel':True,
        'protected_files_unchanged':protected,'comic_discovery':catalog,
        'public_exports_copied_exactly':True,'private_markers_absent_from_build':True,
        'palette_and_svg_checks':True,'changed_paths':sorted(changed),'unexpected_paths':[],
        'concurrent_committed_paths_preserved':sorted(concurrent),
        'other_task_paths_outside_this_check':other_tasks,
        'changelog_unchanged':(ROOT/'CHANGELOG.md').read_bytes()==git('show',f'{BASE}:CHANGELOG.md'),
    }
    assert report['changelog_unchanged']
    (TASK/'proof/first-panel-card/source-and-scope.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Four pages, ten panels, 130 spoken words; dialogue, cards, frozen/public assets, palette, and publication boundaries passed.')
    print(f'{len(changed)} scoped paths; no unexpected changes. Index empty.')


if __name__=='__main__':
    main()
