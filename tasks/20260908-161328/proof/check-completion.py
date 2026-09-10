"""Check the exact episode-completion revision before its requested commit."""
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / 'completion'
EPISODE = ROOT / 'web/src/comics/season-1/episode-1'

def sha(file):
    """Hash source and artifact bytes without changing them."""
    return hashlib.sha256(file.read_bytes()).hexdigest()

baseline = json.loads((OUT / 'before.json').read_text())
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
assert head == baseline['head']
assert not subprocess.check_output(['git', 'diff', '--cached', '--name-only'], cwd=ROOT).strip()
for key in ['protected', 'unrelated']:
    for name, digest in baseline[key].items():
        assert sha(ROOT / name) == digest, name
before = json.loads((OUT / 'before-script.json').read_text())
after = json.loads(subprocess.check_output(['node', 'web/read-comic-script.js', str(EPISODE / 'episode.ts')], cwd=ROOT))
assert after == {**before, 'heading': 'SEASON 1 / EPISODE 1', 'footer': 'A USEFUL JOB'}
assert len(after['pages']) == 18 and all(p['kind'] == 'illustrated' for p in after['pages'])
panels = [panel for page in after['pages'] for panel in page['panels']]
assert len(panels) == 50
words = sum(len(d['text'].split()) for panel in panels for d in panel['dialogue'])
assert words == 730
metadata = json.loads((EPISODE / 'episode.json').read_text())
assert metadata == {**json.loads((OUT / 'before-metadata.json').read_text()), 'publication': 'released'}
prior = json.loads((OUT.parent / 'homecoming/source-and-isolation.json').read_text())['sceneHashes']
assert len(prior) == 50
for cache in ['web/.cache/story/development/generated', 'web/.cache/story/release/generated', 'web/.cache/story/completion/regenerated']:
    actual = {p.name: sha(p) for p in (ROOT / cache / 'season-1/episode-1').glob('*.svg')}
    assert actual == prior, cache
for name in ['browser', 'story-root', 'story-prefix', 'site-root']:
    report = json.loads((OUT / name / 'checks.json').read_text())
    assert report['noServer'] and report['releasedEpisode'] and report['normalizedFraming']
    assert not report['errors'] and len(report['reports']) == 36
    assert all(not p['issues'] for p in report['reports'])
    assert len(report['pixels']) == 18 and all(p['pixels'] == 0 for p in report['pixels'])
    assert len(list((OUT / name).glob('export-page-*.svg'))) == 18
    assert len(list((OUT / name).glob('panel-*.svg'))) == 50
    for file in (OUT / name).glob('*.svg'):
        text = file.read_text()
        assert 'PRIVATE COMIC DRAFT' not in text and 'CLEAN-LINE DRAFT' not in text
    transfers = [p for p in report['reports'] if p['id'] == 'page-14']
    assert len(transfers) == 2
    assert all(len(p['contacts']) == 5 and all(c['error'] < .01 for c in p['contacts']) for p in transfers)
    assert all(all(d > 0 for d in p['faceCropClearance']) for p in transfers)
    assert not Path('/proc', (OUT / name / 'browser.pid').read_text().strip()).exists()
for directory, prefix, story_only in [
    ('web/.cache/story/completion/local', '/', False),
    ('web/.cache/story/completion/public', '/', True),
    ('web/dist-story', '/nova-protocol/', True),
    ('web/dist', '/', False),
]:
    output = ROOT / directory
    if story_only:
        assert sorted(p.name for p in output.iterdir()) == ['story']
    assert sorted(str(p.relative_to(output / 'story')) for p in (output / 'story').rglob('*.html')) == [
        'index.html', 'season-1/episode-1/index.html', 'season-1/index.html']
    art = output / 'story/assets/season-1/episode-1'
    assert {p.name: sha(p) for p in art.glob('*.svg')} == prior
    for file in (output / 'story').rglob('*'):
        if file.suffix in ['.html', '.js', '.map', '.json']:
            text = file.read_text()
            for private in ['OPENING / CLEAN-LINE DRAFT', 'PRIVATE COMIC DRAFT', 'DESIGNS AND DIALOGUE PROVISIONAL', 'tasks/20260908-161328', 'FUTURE_PRIVATE_CANARY']:
                assert private not in text, (file, private)
    source = (output / 'story/season-1/episode-1/index.html').read_text()
    assert 'class="story-draft-label"' not in source and 'data-art-toggle' not in source
    manifest = json.loads(re.search(r'<script id="comic-definition"[^>]*>(.*?)</script>', source, re.S)[1])
    assert manifest['basePath'] == prefix and len(manifest['pages']) == 18
    for page in manifest['pages']:
        assert page['definition']['heading'] == after['heading']
        assert page['definition']['footer'] == after['footer']
    fallback = html.unescape(re.search(r'<noscript>(.*?)</noscript>', source, re.S)[1])
    assert fallback.count('class="story-page-text"') == 18
    for panel in panels:
        assert panel['action'] in fallback
        for dialogue in panel['dialogue']:
            assert dialogue['text'] in fallback
sources = ['web/src/comics/README.md', 'web/src/comics/season-1/episode-1/NOTES.md',
           'tasks/20260908-161328/TASK.md', 'tasks/20260908-161328/COMPLETION-REVIEW.md']
links = 0
for name in sources:
    file = ROOT / name
    text = re.sub(r'```[\s\S]*?```', '', file.read_text())
    for target in re.findall(r'\[[^\]]*\]\(([^)\s]+)\)', text):
        url = urlsplit(target)
        if not url.scheme and url.path and not url.path.startswith('/'):
            assert (file.parent / unquote(url.path)).resolve().exists(), (name, target)
            links += 1
task = (ROOT / 'tasks/20260908-161328/TASK.md').read_text()
assert '- STATUS: OPEN\n' in task and '- PRIORITY: 62\n' in task
assert len(re.findall(r'^- \[ \]', task, re.M)) == 7
assert task.count('- [x]') >= 22
block = (ROOT / 'CHANGELOG.md').read_text().split('## [Unreleased]\n', 1)[1].split('\n## [', 1)[0]
entry = ' '.join(block.split('- ', 1)[1].split())
assert len(entry) <= 200
subprocess.run(['git', 'diff', '--check'], cwd=ROOT, check=True)
result = {'head': head, 'protectedFiles': len(baseline['protected']), 'unrelatedFiles': len(baseline['unrelated']),
          'illustratedPages': 18, 'panels': 50, 'spokenWords': words, 'releasedMetadata': True,
          'readerBuilds': 4, 'cleanLayoutsPerBuild': 36, 'unchangedNormalizedPagePixels': [0] * 18,
          'unchangedRawScenes': 50, 'markdownSources': len(sources), 'localLinks': links,
          'changelogCharacters': len(entry), 'seasonTaskOpen': True, 'serverStarted': False, 'liveDeployment': False}
(OUT / 'source-checks.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
