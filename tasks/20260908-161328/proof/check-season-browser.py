"""Check the season-first revision without rewriting earlier evidence."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / 'season-browser'
before = json.loads((OUT / 'before.json').read_text())
for name, digest in before['protected'].items():
    assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, name
assert (ROOT / 'CHANGELOG.md').read_text() == before['sources']['CHANGELOG.md']
for name in ['browser', 'prefix']:
    data = json.loads((OUT / name / 'checks.json').read_text())
    assert data['noServer'] and data['seasonLibrary'] and data['episodeLists']
    assert data['browserHistory'] and data['modalChecks'] and data['noLayoutUi']
    assert not data['errors']
    assert len(data['reports']) == 14 and all(not p['issues'] for p in data['reports'])
    assert len(data['pixels']) == 7 and all(p['pixels'] == 0 for p in data['pixels'])
for name in ['public', 'public-prefix']:
    directory = ROOT / 'web/.cache/story/season-browser' / name
    assert sorted(str(f.relative_to(directory)) for f in (directory / 'story').rglob('*.html')) == ['story/index.html']
    assert not (directory / 'assets/story').exists()
    for file in directory.rglob('*'):
        if file.suffix in ['.html', '.js', '.map', '.json', '.svg']:
            text = file.read_text()
            for private in ["Coffee's still hot", 'baikal-work.svg', 'residential-console.svg', 'A useful job', 'story/season-1/episode-1', 'story/demo']:
                assert private not in text, (file, private)
for file in OUT.glob('*/browser.pid'):
    assert not Path('/proc', file.read_text().strip()).exists(), file
markup = (ROOT / 'web/comic-build.js').read_text()
assert re.search(r'<noscript>.*?</noscript>', markup, re.S)[0] == re.search(r'<noscript>.*?</noscript>', before['sources']['web/comic-build.js'], re.S)[0]
assert 'comicSeasonRedirect' not in markup
files = ['docs/development.md', 'docs/keeping-docs-in-sync.md', 'web/src/comics/README.md', 'tasks/20260908-161328/TASK.md', 'tasks/20260908-161328/SEASON-BROWSER-REVIEW.md']
links = 0
for name in files:
    file = ROOT / name
    text = re.sub(r'```[\s\S]*?```', '', file.read_text())
    for target in re.findall(r'\[[^\]]*\]\(([^)\s]+)\)', text):
        url = urlsplit(target)
        if url.scheme or not url.path or url.path.startswith('/'):
            continue
        assert (file.parent / unquote(url.path)).resolve().exists(), (name, target)
        links += 1
entry = re.search(r'^- Story lists[^\n]+(?:\n  [^\n]+)*', (ROOT / 'CHANGELOG.md').read_text(), re.M)[0]
assert len(' '.join(entry[2:].split())) <= 200
assert not subprocess.check_output(['git', 'diff', '--cached', '--name-only'], cwd=ROOT, text=True).strip()
assert len(re.findall(r'^- \[ \]', (ROOT / 'tasks/20260908-161328/TASK.md').read_text(), re.M)) == 7
result = {
    'protectedFiles': len(before['protected']), 'markdownSources': len(files),
    'localLinkTargets': links, 'seasonLibrary': True, 'episodeLists': True,
    'publicStoryRoutes': ['story/index.html'], 'pixelDifferences': [0] * 7,
    'serverStarted': False, 'inspectionBrowsersStopped': True, 'indexEmpty': True,
    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
}
(OUT / 'source-and-isolation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
