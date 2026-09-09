#!/usr/bin/env python3
"""Check the reader migration against its saved pre-batch working tree."""
import ast
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent/'reader'
TASK = 'tasks/20260908-161328/'
BEFORE = json.loads((OUT/'before.json').read_text())['files']


def digest(file):
    return hashlib.sha256(file.read_bytes()).hexdigest()


def tree(directory):
    return {str(p.relative_to(directory)):digest(p) for p in directory.rglob('*') if p.is_file()}


retained = []
for name, expected in BEFORE.items():
    protected = (name.startswith(TASK+'proof/') or
                 name.startswith('web/src/assets/lore/') or
                 name.startswith('web/src/lore/') or
                 name.startswith('scripts/nova_illustration/') and name.endswith('.py') or
                 name.startswith(TASK) and name.endswith(('.svg','.html')))
    if protected:
        assert digest(ROOT/name)==expected,name
        retained.append(name)

snapshots = ['comic-opening-poc/generate.py','aquila-pages/generate.py','aquila-pages/props.py',
             'opening_props.py','episode-1/SCRIPT.md','episode-1/generate.mjs']
for source in snapshots:
    assert digest(ROOT/(TASK+source+'.txt'))==BEFORE[TASK+source],source

canonical = ROOT/'art/comics/season-1/episode-1'
for source, target in [('comic-opening-poc/generate.py','opening.py'),('aquila-pages/generate.py','aquila.py')]:
    def pages(file):
        return next(node for node in ast.parse(file.read_text()).body if isinstance(node,ast.FunctionDef) and node.name=='pages')
    assert ast.dump(pages(ROOT/(TASK+source+'.txt')))==ast.dump(pages(canonical/target)),target
for source in ['props.py','opening_props.py']:
    old = 'aquila-pages/props.py' if source=='props.py' else source
    assert digest(canonical/source)==BEFORE[TASK+old],source
old_script = (ROOT/(TASK+'episode-1/SCRIPT.md.txt')).read_text()
new_script = (canonical/'SCRIPT.md').read_text()
assert old_script.split('## Page 5',1)[1]==new_script.split('## Page 5',1)[1]

with tempfile.TemporaryDirectory(prefix='nova-reader-generation-') as temporary:
    output = Path(temporary)
    command = ['python3','art/comics/build.py','--output',str(output)]
    subprocess.run(command,cwd=ROOT,check=True)
    first = tree(output)
    mtimes = {name:(output/name).stat().st_mtime_ns for name in first}
    stale = output/'assets/season-1/episode-1/stale.svg'
    stale.write_text('Obsolete generated output')
    obsolete = output/'comics/obsolete/pages'
    obsolete.mkdir(parents=True)
    (obsolete/'old.json').write_text('{}')
    subprocess.run(command,cwd=ROOT,check=True)
    assert tree(output)==first
    assert not stale.exists()
    assert not obsolete.parent.exists()
    assert {name:(output/name).stat().st_mtime_ns for name in first}==mtimes
    assert len(first)==16
    generated = json.loads((output/'comics/season-1/comic.json').read_text())
    pages = generated['episodes'][0]['pages']
    assert [p['id'] for p in pages]==[f'page-{n}' for n in range(1,8)]
    assert all(p['transcript'].strip() for p in pages)
    assert tree(ROOT/'web/.cache/story-preview/generated')==first

blocked = subprocess.run(['python3','art/comics/build.py','--output','web/src'],cwd=ROOT,capture_output=True,text=True)
assert blocked.returncode and 'cache/story-preview/generated only' in blocked.stderr

builds = {}
for name in ['web/dist','web/.cache/story-preview/public-prefix']:
    directory = ROOT/name
    assert directory.exists(),name
    routes = sorted(str(p.relative_to(directory)) for p in (directory/'story').rglob('*.html'))
    assert routes==['story/demo/episode-1/index.html','story/demo/index.html','story/index.html'],routes
    assert tree(directory/'assets/story')==tree(ROOT/'web/src/assets/story')
    checked = 0
    for file in directory.rglob('*'):
        if file.is_file() and file.suffix in ('.js','.json','.html','.svg','.map'):
            content = file.read_text()
            assert "Coffee's still hot" not in content,file
            assert 'episode-1/page-05.svg' not in content,file
            checked += 1
    builds[name] = {'routes':routes,'filesScanned':checked}

task = (ROOT/(TASK+'TASK.md')).read_text()
assert '- STATUS: OPEN' in task
assert sum(line.startswith('- [ ]') for line in task.splitlines())==7
assert subprocess.check_output(['git','diff','--cached','--name-only'],cwd=ROOT)==b''
changes = [name for name,expected in BEFORE.items() if not (ROOT/name).exists() or digest(ROOT/name)!=expected]
report = {'protectedFilesRetained':len(retained),'exactSourceSnapshots':snapshots,
          'authoredPageFunctionsUnchanged':['opening.py','aquila.py'],'pages5Through18ScriptUnchanged':True,
          'generatedFiles':16,'illustratedPages':7,'stableRegenerationAndStaleCleanup':True,
          'publicBuilds':builds,'indexEmpty':True,'preExistingPathsChanged':sorted(changes),
          'scopeNote':'The changed-path list includes concurrent media, news, and widget work; it is not a staging allowlist.'}
(OUT/'source-and-isolation.json').write_text(json.dumps(report,indent=2)+'\n')
print(f'Reader source/isolation checks passed; {len(retained)} protected files retained, seven illustrated pages, demo-only public builds.')
