"""Revision-scoped source and output proof. No prior proof writer is rerun."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent/'dsl'
EP=ROOT/'web/src/comics/season-1/episode-1'
before=json.loads((OUT/'before.json').read_text())
script=json.loads(subprocess.check_output(['node',str(ROOT/'web/read-comic-script.js'),str(EP/'episode.ts')],text=True))
assert len(script['pages'])==18
assert sum(len(p['panels']) for p in script['pages'])==50
assert sum(len(d['text'].split()) for p in script['pages'] for panel in p['panels'] for d in panel['dialogue'])==730
for old,new in zip(before['pages'],script['pages']):
    for a,b in zip(old['panels'],new['panels']):
        assert a['action']==b['action'],b['id']
        assert [(v['speaker'],' '.join(v['text'].split())) for v in a['balloons']]==[(d['speaker'],d['text']) for d in b['dialogue']],b['id']
        assert new['layout'][b['id']]=={'at':a['at'],'size':a['size']}
oldscript=(OUT/'SCRIPT.md.txt').read_text()
for match in re.finditer(r'^## Page (\d+): ([^\n]+)\n([\s\S]+?)(?=^## |\Z)',oldscript,re.M):
    number=int(match[1]);new=script['pages'][number-1]
    assert new['title']==match[2]
    assert new['purpose']==re.search(r'\*\*Purpose:\*\* (.+)',match[3])[1]
    panels=list(re.finditer(r'^### Panel ([0-9]+[a-z])\n([\s\S]+?)(?=^### |\Z)',match[3],re.M))
    assert len(panels)==len(new['panels'])
    for old,panel in zip(panels,new['panels']):
        assert old[1]==panel['id']
        action=' '.join(re.split(r'^\*\*[^\n]+:\*\* ',old[2],maxsplit=1,flags=re.M)[0].split()).replace('**','')
        assert panel['action']==action,panel['id']
        spoken=[]
        for paragraph in re.split(r'\n\s*\n',old[2]):
            line=re.match(r'^\*\*([^*]+):\*\* (.+)$',' '.join(paragraph.split()))
            if line:spoken.append((line[1],line[2]))
        assert [(d['speaker'],d['text']) for d in panel['dialogue']]==spoken,panel['id']
items=json.loads((OUT/'migration-items.json').read_text())
for page in script['pages'][:7]:
    for panel in page['panels']:
        assert panel['labels']==items['labels'][panel['id']]
        assert panel['cards']==items['cards'].get(panel['id'],[])
        assert panel['art']==items['sceneIds'][panel['id']]
for file in ['generate.py','opening.py','aquila.py','props.py','opening_props.py','SCRIPT.md']:
    assert not (EP/file).exists(),file
for file in (EP/'art').glob('*.py'):
    source=file.read_text()
    assert not re.search(r'\b(Panel|say|speak|text|render_page|dialogue_lines|script_panels)\(',source),file
allowed={
 'scripts/nova_illustration/README.md',
 'tasks/20260908-161328/TASK.md',
 'tasks/20260908-161328/SOURCE-REVIEW.md',
 'tasks/20260908-161328/READER-REVIEW.md',
 'tasks/20260908-161328/episode-1/SCRIPT.md',
 'tasks/20260908-161328/clean-line-study/generate.py',
 'tasks/20260908-161328/clean-line-study/README.md',
}
protected=0
for file,digest in before['files'].items():
    if file in allowed:continue
    assert hashlib.sha256((ROOT/file).read_bytes()).hexdigest()==digest,file
    protected+=1
for name in ['browser','prefix']:
    report=json.loads((OUT/name/'checks.json').read_text())
    assert report['noServer'] and report['modalChecks'] and report['noLayoutUi']
    assert len(report['reports'])==14 and len(report['pixels'])==7
    assert all(p['pixels']==0 for p in report['pixels'])
    assert all(not p['issues'] for p in report['reports'])
for name in ['public','public-prefix']:
    directory=ROOT/'web/.cache/story/dsl'/name
    assert [str(f.relative_to(directory)) for f in (directory/'story').rglob('*.html')]==['story/index.html']
    assert not (directory/'assets/story').exists()
    for file in directory.rglob('*'):
        if file.suffix in ['.html','.js','.json','.svg','.map']:
            text=file.read_text()
            for private in ["Coffee's still hot",'residential-console.svg','FUTURE PRIVATE CANARY','season-1/episode-1/pages','story/demo','baikal-work.svg']:
                assert private not in text,(file,private)
assert not subprocess.check_output(['git','diff','--cached','--name-only'],cwd=ROOT,text=True).strip()
assert len(re.findall(r'^- \[ \]',(ROOT/'tasks/20260908-161328/TASK.md').read_text(),re.M))==7
result={'protectedFiles':protected,'scriptPages':18,'panels':50,'spokenWords':730,'illustratedPages':7,'pixelDifferences':[0]*7,'serverStarted':False,'publication':'draft','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'indexEmpty':True}
(OUT/'source-and-isolation.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
