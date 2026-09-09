"""Check independent deployment evidence without changing earlier proof."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / 'deployment'
preserved = json.loads((OUT / 'preserved.json').read_text())
for name, digest in preserved['hashes'].items():
    assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, name
for name in ['browser', 'prefix']:
    report = json.loads((OUT / name / 'checks.json').read_text())
    assert report['ownedResources'] and report['releasedFixture'] and not report['errors']
    assert len(report['reports']) == 14 and all(not p['issues'] for p in report['reports'])
    assert len(report['pixels']) == 7 and all(p['pixels'] == 0 for p in report['pixels'])
for name in ['web/dist-story', 'web/.cache/story/deployment-public-prefix']:
    directory = ROOT / name
    assert {p.name for p in directory.iterdir()} == {'story'}
    assert sorted(str(p.relative_to(directory)) for p in directory.rglob('*.html')) == ['story/index.html']
    assert not (directory / 'story/assets/season-1').exists()
    for file in directory.rglob('*'):
        if file.suffix in ['.html', '.js', '.map', '.json', '.svg']:
            text = file.read_text()
            for private in ['A useful job', 'baikal-work.svg', 'residential-console.svg', 'FUTURE_PRIVATE_CANARY']:
                assert private not in text, (file, private)
site = ROOT / 'web/.cache/story/deployment-site'
assert (site / 'index.html').is_file() and not (site / 'story').exists()
for file in OUT.glob('*/browser.pid'):
    assert not Path('/proc', file.read_text().strip()).exists(), file
sources = ['web/src/comics/README.md', 'docs/development.md', 'docs/keeping-docs-in-sync.md', 'RELEASE.md', 'tasks/20260908-161328/TASK.md', 'tasks/20260908-161328/DEPLOYMENT-REVIEW.md']
links = 0
for name in sources:
    file = ROOT / name
    text = re.sub(r'```[\s\S]*?```', '', file.read_text())
    for target in re.findall(r'\[[^\]]*\]\(([^)\s]+)\)', text):
        url = urlsplit(target)
        if url.scheme or not url.path or url.path.startswith('/'):
            continue
        assert (file.parent / unquote(url.path)).resolve().exists(), (name, target)
        links += 1
entry = re.search(r'^- Comic tags[^\n]+(?:\n  [^\n]+)*', (ROOT / 'CHANGELOG.md').read_text(), re.M)[0]
assert len(' '.join(entry[2:].split())) <= 200
assert not subprocess.check_output(['git', 'diff', '--cached', '--name-only'], cwd=ROOT).strip()
assert len(re.findall(r'^- \[ \]', (ROOT / 'tasks/20260908-161328/TASK.md').read_text(), re.M)) == 7
result = {'protectedFiles': len(preserved['hashes']), 'markdownSources': len(sources), 'localLinks': links, 'changelogCharacters': len(' '.join(entry[2:].split())), 'pixelDifferences': [0] * 7, 'realPublication': 'draft', 'serverStarted': False, 'liveDeployment': False, 'indexEmpty': True, 'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()}
(OUT / 'source-and-isolation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
