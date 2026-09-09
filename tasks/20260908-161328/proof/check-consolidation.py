#!/usr/bin/env python3
"""Check the source move and the draft/release boundary at this revision."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TASK = ROOT/'tasks/20260908-161328'
OUT = TASK/'proof/consolidation'
before = json.loads((OUT/'before.json').read_text())

def digest(file):
    return hashlib.sha256(file.read_bytes()).hexdigest()

protected = []
for name, expected in before['files'].items():
    file = ROOT/name
    keep = (name.startswith('tasks/20260908-161328/proof/') or
            name.startswith('web/src/assets/lore/') or
            name.startswith('scripts/nova_illustration/') and file.suffix=='.py' or
            name.startswith('tasks/20260908-161328/') and file.suffix in ('.svg','.html','.txt'))
    if keep:
        assert digest(file)==expected,name
        protected.append(name)

old = 'art/comics/season-1/episode-1/'
new = ROOT/'web/src/comics/season-1/episode-1'
for name, current, previous in [('opening.py','parents[5]','parents[4]'),('aquila.py','OUTPUT.parents[4]','OUTPUT.parents[3]')]:
    data = (new/name).read_text().replace(current,previous).encode()
    assert hashlib.sha256(data).hexdigest()==before['files'][old+name],name
for name in ['props.py','opening_props.py']:
    assert digest(new/name)==before['files'][old+name]
script = (new/'SCRIPT.md').read_text().replace('../../../lore/seasons/season-1.md','../../../../web/src/lore/seasons/season-1.md').replace('npm --prefix web run serve','npm --prefix web run story:dev')
assert hashlib.sha256(script.encode()).hexdigest()==before['files'][old+'SCRIPT.md']

cache = ROOT/'web/.cache/story/development/generated'
for name, expected in before['scenes'].items():
    if name=='season-1/cover.svg':name='season-1/episode-1/cover.svg'
    assert digest(cache/name)==expected,name
assert len(list(cache.rglob('*.svg')))==8
assert json.loads((new/'episode.json').read_text())['publication']=='draft'
assert json.loads((new/'episode.json').read_text())['pageCount']==18
assert len(json.loads((cache/'season-1/episode-1/pages.json').read_text()))==7

for removed in ['art/comics','web/story-preview.js','web/src/comics/demo','web/src/assets/story/demo']:
    assert not (ROOT/removed).exists(),removed
package = json.loads((ROOT/'web/package.json').read_text())
assert 'story:dev' not in package['scripts'] and 'story:build' not in package['scripts']
assert 'storyPreview' not in (ROOT/'web/webpack.config.js').read_text()
assert 'require.context' not in (ROOT/'web/src/comics/comic-catalog.ts').read_text()

builds = {}
for directory in [ROOT/'web/dist',ROOT/'web/.cache/story/public-prefix']:
    routes = sorted(str(p.relative_to(directory)) for p in (directory/'story').rglob('*.html'))
    assert routes==['story/index.html'],routes
    assert not (directory/'assets/story').exists()
    checked = 0
    for file in directory.rglob('*'):
        if file.is_file() and file.suffix in ('.html','.js','.json','.svg','.map'):
            text = file.read_text()
            for marker in ["Coffee's still hot",'episode-1/page-05.svg','story/demo/','comics/season-1/episode-1/opening.py']:
                assert marker not in text,(file,marker)
            checked += 1
    builds[str(directory.relative_to(ROOT))]={'routes':routes,'filesScanned':checked,'storyAssets':0}

assert '- STATUS: OPEN' in (TASK/'TASK.md').read_text()
assert sum(line.startswith('- [ ]') for line in (TASK/'TASK.md').read_text().splitlines())==7
assert subprocess.check_output(['git','diff','--cached','--name-only'],cwd=ROOT)==b''
report = {'baseline':before['head'],'protectedFilesRetained':len(protected),'sceneAndCoverHashesRetained':8,
          'episodeSourceChanges':'Only root paths and the script documentation link/serve command changed.',
          'illustratedPages':7,'completePageCount':18,'publication':'draft','publicBuilds':builds,'indexEmpty':True}
(OUT/'source-and-isolation.json').write_text(json.dumps(report,indent=2)+'\n')
print(f'Consolidation checks passed: {len(protected)} protected files, eight exact SVGs, seven draft pages, empty released-only builds.')
