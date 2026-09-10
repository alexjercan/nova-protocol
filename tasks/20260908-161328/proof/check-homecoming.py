"""Verify this private illustration batch without rewriting older evidence."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
from urllib.parse import urlsplit, unquote
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / 'homecoming'
EPISODE = ROOT / 'web/src/comics/season-1/episode-1'
before = json.loads((OUT / 'before-script.json').read_text())
after = json.loads(subprocess.check_output(['node', 'web/read-comic-script.js', str(EPISODE / 'episode.ts')], cwd=ROOT))
assert before['heading'] == after['heading'] and before['footer'] == after['footer']
assert len(after['pages']) == 18
assert sum(len(p['panels']) for p in after['pages']) == 50
words = sum(len(d['text'].split()) for p in after['pages'] for panel in p['panels'] for d in panel['dialogue'])
assert words == 730, words
for n, (old, new) in enumerate(zip(before['pages'], after['pages'], strict=True), 1):
    for key in ['id', 'title', 'purpose']:
        assert old[key] == new[key], (n, key)
    assert len(old['panels']) == len(new['panels'])
    for a, b in zip(old['panels'], new['panels'], strict=True):
        for key in ['id', 'action', 'dialogue']:
            assert a[key] == b[key], (n, a['id'], key)
    if n not in [16, 17, 18]:
        assert old == new, n
assert [p['kind'] for p in after['pages']] == ['illustrated'] * 18
metadata = json.loads((EPISODE / 'episode.json').read_text())
assert metadata['publication'] == 'draft' and metadata['pageCount'] == 18
preserved = json.loads((OUT / 'before.json').read_text())['protected']
for name, digest in preserved.items():
    assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, name
new_scenes = [panel['art'] for p in after['pages'][15:18] for panel in p['panels']]
assert len(set(new_scenes)) == 8
cache = ROOT / 'web/.cache/story/development/generated/season-1/episode-1'
assets = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in cache.glob('*.svg')}
assert len(assets) == 50
expected_faces = [4, 2, 5, 2, 0, 2, 2, 1]
for scene, face_count in zip(new_scenes, expected_faces, strict=True):
    svg = ET.fromstring((cache / (scene + '.svg')).read_text())
    assert all(word in ['EBRO'] for word in ''.join(svg.itertext()).split()), scene
    faces = [e for e in svg.iter() if e.get('data-face')]
    assert len(faces) == face_count, (scene, len(faces))
    assert all(e.get('data-gaze') == 'forward' and not e.get('data-expression') for e in faces)
    assert not any(e.get('data-face') in ['daniel', 'mara'] for e in faces)
    if scene in ['asking-for-record', 'welcome-to-baikal']:
        patients = [e for e in svg.iter() if e.get('data-prop') == 'padded-stretcher']
        assert len(patients) == 1 and patients[0].get('data-body-restraints') == 'secured'
    if scene == 'asking-for-record':
        clamps = [e for e in svg.iter() if e.get('data-prop') == 'receiving-restraint']
        assert len(clamps) == 2
        assert all(e.get('data-mechanism') == 'rigid-clamp' and e.get('data-latch') == 'closed' for e in clamps)
    if scene == 'welcome-to-baikal':
        assert {e.get('data-holder') for e in svg.iter() if e.get('data-holder')} == {'samir'}
    if scene == 'covers-still-on':
        names = {e.get('data-component') for e in svg.iter()}
        assert {'replacement-housing', 'protective-cover', 'assembly-restraint'} <= names
    if scene in ['line-back-in-service', 'penalty-at-window', 'mug-back-home']:
        assert [e.get('data-ship') for e in svg.iter() if e.get('data-ship')] == ['ebro']
    if scene == 'mug-back-home':
        assert any(e.get('data-prop') == 'returned-borrowed-mug' and e.get('data-contact') == 'worktop' for e in svg.iter())
prior_assets = json.loads((OUT.parent / 'transfer/source-and-isolation.json').read_text())['sceneHashes']
assert all(assets[name] == digest for name, digest in prior_assets.items())
assert len(prior_assets) == 42
source = (EPISODE / 'art/homecoming.py').read_text()
assert not re.search(r'#[0-9a-fA-F]{6}\b', source)
for name in ['browser', 'prefix']:
    report = json.loads((OUT / name / 'checks.json').read_text())
    assert not report['errors'] and len(report['reports']) == 36
    assert all(not p['issues'] for p in report['reports'])
    transfers = [p for p in report['reports'] if p['id'] == 'page-14']
    assert len(transfers) == 2
    assert all(len(p['contacts']) == 5 and all(c['error'] < .01 for c in p['contacts']) for p in transfers)
    assert all(all(d > 0 for d in p['faceCropClearance']) for p in transfers)
    assert len(report['pixels']) == 15 and all(p['pixels'] == 0 for p in report['pixels'])
    assert sum(len(p['panels']) for p in report['reports'] if p['width'] == 1440) == 50
    assert len(list((OUT / name).glob('export-page-*.svg'))) == 18
for name in ['public', 'public-prefix']:
    directory = ROOT / 'web/.cache/story/homecoming' / name
    assert sorted(str(p.relative_to(directory)) for p in (directory / 'story').rglob('*.html')) == ['story/index.html']
    assert not (directory / 'story/assets/season-1').exists()
    for file in directory.rglob('*'):
        if file.suffix in ['.html', '.js', '.map', '.json', '.svg']:
            text = file.read_text()
            for private in ['A useful job', 'mug-back-home.svg', 'line-back-in-service.svg', "Still late. The penalty stands.", 'supported-recline', 'story/season-1/episode-1']:
                assert private not in text, (file, private)
for file in OUT.glob('*/browser.pid'):
    assert not Path('/proc', file.read_text().strip()).exists(), file
sources = ['web/src/comics/README.md', 'web/src/comics/season-1/episode-1/NOTES.md', 'tasks/20260908-161328/TASK.md', 'tasks/20260908-161328/HOMECOMING-REVIEW.md', 'web/src/lore/seasons/season-1.md']
links = 0
for name in sources:
    file = ROOT / name
    text = re.sub(r'```[\s\S]*?```', '', file.read_text())
    for target in re.findall(r'\[[^\]]*\]\(([^)\s]+)\)', text):
        url = urlsplit(target)
        if not url.scheme and url.path and not url.path.startswith('/'):
            assert (file.parent / unquote(url.path)).resolve().exists(), (name, target)
            links += 1
assert not subprocess.check_output(['git', 'diff', '--cached', '--name-only'], cwd=ROOT).strip()
assert len(re.findall(r'^- \[ \]', (ROOT / 'tasks/20260908-161328/TASK.md').read_text(), re.M)) == 7
result = {'protectedFiles': len(preserved), 'scriptPages': 18, 'panels': 50, 'spokenWords': words, 'illustratedPages': 18, 'newScenes': 8, 'rawScenes': 50, 'unchangedEarlierPixels': [0] * 15, 'markdownSources': len(sources), 'localLinks': links, 'serverStarted': False, 'draftOnly': True, 'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'sceneHashes': assets}
(OUT / 'source-and-isolation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({k:v for k,v in result.items() if k != 'sceneHashes'}))
