"""Check this navigation revision without rewriting earlier proof."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit

ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent/'navigation'
before=json.loads((OUT/'before.json').read_text())
for name,digest in before['protected'].items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest()==digest,name
for name in ['browser','prefix']:
    data=json.loads((OUT/name/'checks.json').read_text())
    assert data['noServer'] and data['singleLibrary'] and data['seasonRedirect']
    assert data['modalChecks'] and data['noLayoutUi'] and not data['errors']
    assert len(data['reports'])==14 and all(not p['issues'] for p in data['reports'])
    assert len(data['pixels'])==7 and all(p['pixels']==0 for p in data['pixels'])
for name in ['public','public-prefix']:
    directory=ROOT/'web/.cache/story/navigation'/name
    assert sorted(str(f.relative_to(directory)) for f in (directory/'story').rglob('*.html'))==['story/index.html']
    assert not (directory/'assets/story').exists()
    for file in directory.rglob('*'):
        if file.suffix in ['.html','.js','.map','.json','.svg']:
            text=file.read_text()
            for private in ["Coffee's still hot",'baikal-work.svg','residential-console.svg','A useful job','story/season-1/episode-1','story/demo']:
                assert private not in text,(file,private)
files=['docs/development.md','web/src/comics/README.md','tasks/20260908-161328/TASK.md','tasks/20260908-161328/NAVIGATION-REVIEW.md']
links=0
for name in files:
    file=ROOT/name
    text=re.sub(r'```[\s\S]*?```','',file.read_text())
    for target in re.findall(r'\[[^\]]*\]\(([^)\s]+)\)',text):
        url=urlsplit(target)
        if url.scheme or not url.path or url.path.startswith('/'):continue
        assert (file.parent/unquote(url.path)).resolve().exists(),(name,target)
        links+=1
entry=re.search(r'^- Story lists[^\n]+(?:\n  [^\n]+)*',(ROOT/'CHANGELOG.md').read_text(),re.M)[0]
assert len(' '.join(entry[2:].split()))<=200
assert not subprocess.check_output(['git','diff','--cached','--name-only'],cwd=ROOT,text=True).strip()
assert len(re.findall(r'^- \[ \]',(ROOT/'tasks/20260908-161328/TASK.md').read_text(),re.M))==7
result={'protectedFiles':len(before['protected']),'markdownSources':len(files),'localLinkTargets':links,'singleLibrary':True,'publicStoryRoutes':['story/index.html'],'pixelDifferences':[0]*7,'serverStarted':False,'indexEmpty':True,'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()}
(OUT/'source-and-isolation.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
