#!/usr/bin/env python3
"""Check the private episode script, retained opening, and publication boundaries."""

import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path
from types import ModuleType

ROOT = Path(__file__).resolve().parents[3]
TASK = 'tasks/20260908-161328'
BASE = '62043f3b531f3ac7ec4b9d684150466b7efbb181'
sys.dont_write_bytecode = True


def git(*args):
    """Read revision evidence without changing the index."""
    return subprocess.check_output(['git',*args],cwd=ROOT)


def baseline_module(relative, package=''):
    """Load a retained definition without running its exporter."""
    module = ModuleType('episode_baseline_'+Path(relative).stem.replace('-','_'))
    module.__file__ = str(ROOT/relative)
    module.__package__ = package
    sys.modules[module.__name__] = module
    exec(compile(git('show',f'{BASE}:{relative}'),relative,'exec'),module.__dict__)
    return module


def main():
    """Write the bounded source and script report for the approved feedback."""
    protected = [
        'scripts/gen-lore-designs.py',
        *[f'scripts/nova_illustration/{name}.py' for name in ('__init__','expressions','lettering','scenery','ships','styles','svg')],
        'web/src/assets/lore','web/src/assets/story',
        *[f'{TASK}/{name}' for name in ('clean-line-study','expression-study','opening_props.py')],
        *[f'{TASK}/proof/{name}' for name in ('baseline','revised','before-clean-line-opening','restyled','first-panel-card','clean-line','clean-line-initial','clean-line-before-frontal','expressions')],
    ]
    assert not git('diff','--name-only',BASE,'--',*protected), 'Retained artwork or prior proof changed'
    source = ROOT/TASK/'comic-opening-poc/generate.py'
    spec = importlib.util.spec_from_file_location('opening',source)
    opening = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(opening)
    retained_panels = [panel for _,_,panels in opening.pages() for panel in panels]
    record_roles = {'Location and time','Display','Delivery record'}
    retained_words = sum(len(line.split()) for panel in retained_panels for speaker,line in panel.lines if speaker not in record_roles)
    assert retained_words == 134 and len(retained_panels) == 10
    before = baseline_module(f'{TASK}/comic-opening-poc/generate.py')
    original_pages, current_pages = before.pages(), opening.pages()
    spoken = lambda pages: [(speaker,line) for _,_,panels in pages for panel in panels for speaker,line in panel.lines if speaker not in record_roles]
    current_spoken = spoken(current_pages)
    assert current_spoken.count(('Samir','Residential supply is normal.')) == 1
    assert [line for line in current_spoken if line[0]!='Samir'] == spoken(original_pages)
    for number in (1,3,4):
        title,_,panels = original_pages[number-1]
        old_svg = before.render_page(number,title,panels)
        assert old_svg.encode() == git('show',f'{BASE}:{TASK}/comic-opening-poc/page-0{number}.svg')
        new_svg = (ROOT/TASK/f'comic-opening-poc/page-0{number}.svg').read_text()
        if number == 1:
            new_svg = new_svg.replace('2078','Opening day / calendar date TBD')
        elif number == 4:
            new_svg = new_svg.replace('Later that day','Later that day / calendar date TBD')
        assert new_svg == old_svg, number
    assert original_pages[1][2][2].render() == current_pages[1][2][2].render()
    from nova_illustration import colors, faces, portraits
    prior_colors = baseline_module('scripts/nova_illustration/colors.py','nova_illustration')
    for name,value in vars(prior_colors).items():
        if name.isupper():
            assert value == getattr(colors,name), name
    prior_faces = baseline_module('scripts/nova_illustration/faces.py','nova_illustration')
    for name in prior_faces.FACES:
        assert prior_faces.frontal_head(name) == faces.frontal_head(name), name
    prior_poses = baseline_module('scripts/nova_illustration/portraits.py','nova_illustration')
    for name in ('elena_close','elena_gesture','jonah_listener'):
        assert getattr(prior_poses,name)() == getattr(portraits,name)(), name
    for name in ('leila','rina','tomas'):
        assert prior_poses.work_portrait(name) == portraits.work_portrait(name), name
    archive = ROOT/TASK/'proof/before-episode-feedback'
    for source,record in json.loads((archive/'sources.json').read_text()).items():
        assert hashlib.sha256((archive/record['file']).read_bytes()).hexdigest() == record['sha256'], source
    for source,sha in json.loads((archive/'retained-hashes.json').read_text()).items():
        assert hashlib.sha256((ROOT/source).read_bytes()).hexdigest() == sha, source
    script = (ROOT/TASK/'episode-1/SCRIPT.md').read_text()
    parts = re.split(r'^## Page (\d+):[^\n]*\n',script,flags=re.M)
    pages = []
    cast = {'Tomas','Rina','Samir','Leila','Jonah','Nadia','Owen','Ivo','Elena','Daniel'}
    for i in range(1,len(parts),2):
        number,body = int(parts[i]),parts[i+1]
        panels = re.findall(r'^### Panel (\d+[a-z])$',body,re.M)
        assert panels == [f'{number}{chr(97+j)}' for j in range(len(panels))]
        assert len(panels) in (2,3)
        speech = []
        for block in body.split('\n\n'):
            match = re.match(r'\*\*([^*]+):\*\*\s+(.*)',block,re.S)
            if match and match[1] != 'Purpose':
                assert match[1].split(' /')[0] in cast, match[1]
                speech.append(match[2])
        pages.append({'page':number,'panels':len(panels),'spoken_words':sum(len(line.split()) for line in speech)})
    assert [page['page'] for page in pages] == list(range(5,19))
    assert '<!-- opening-pages -->' in script
    assert 'practise' not in script and 'Samir shows Leila a close view' not in script
    assert 'Leila finishes checking the connection faces in person.' in script
    assert 'Both coast without thrust;' in script
    assert 'They do not carry his weight under their arms.' in script
    assert "I'm not approving the cost." in script
    task = (ROOT/TASK/'TASK.md').read_text()
    assert '- STATUS: OPEN\n- PRIORITY: 50\n- TAGS: v0.13.0, story, comic' in task
    assert task.count('- [ ]') == 7
    public = ROOT/'web/dist'
    assert sorted(p.name for p in (public/'story').iterdir()) == ['demo','index.html']
    assert (public/'story/demo/episode-1/index.html').is_file()
    public_lore = (public/'lore/index.html').read_text()
    assert 'Nova Protocol opens in 2078' in public_lore
    assert "Earth&#39;s calendar" in public_lore or "Earth's calendar" in public_lore
    for file in public.rglob('*'):
        if file.suffix not in ('.html','.js'):
            continue
        data = file.read_bytes()
        for marker in (b'PRIVATE SCRIPT REVIEW',b'Page 18: The bill',b'episode-1/SCRIPT.md',b'comic-opening-poc'):
            assert marker not in data, file
    assets = {}
    for asset in (ROOT/'web/src/assets/lore').glob('*.svg'):
        assert asset.read_bytes() == (public/'assets/lore'/asset.name).read_bytes()
        assets[str(asset.relative_to(ROOT))] = hashlib.sha256(asset.read_bytes()).hexdigest()
    allowed = {
        'CHANGELOG.md','docs/development.md','web/comic-build.js','web/webpack.config.js',
        'web/src/comics/comic-catalog.ts','web/src/comics/comic-page.ts','web/src/comics/comic-renderer.ts',
        'web/src/comics/demo/comic.json','web/src/comics/demo/pages/poster.ts','web/src/story-reader.ts',
        'web/src/story.ts','web/src/style.css','web/tests/comics.test.js','web/tests/comic-renderer.test.ts',
        'web/src/lore/seasons/season-1.md','web/src/lore/index.md',f'{TASK}/TASK.md',f'{TASK}/EPISODE-REVIEW.md',
        f'{TASK}/EPISODE-FEEDBACK.md',f'{TASK}/proof/check-episode.py',f'{TASK}/proof/inspect-episodes.mjs',
        f'{TASK}/proof/inspect-opening-feedback.mjs','scripts/gen-lore-portraits.py',
        *[f'scripts/nova_illustration/{name}' for name in ('colors.py','faces.py','portraits.py','test_illustration.py','README.md')],
    }
    paths = set(git('diff','--name-only','HEAD').decode().splitlines())
    paths.update(git('ls-files','--others','--exclude-standard').decode().splitlines())
    excluded = []
    for name in paths:
        prefixes = ('episode-1/','comic-opening-poc/','proof/episode-1/','proof/episode-feedback/','proof/before-episode-feedback/')
        if name in allowed or any(name.startswith(f'{TASK}/{prefix}') for prefix in prefixes):
            continue
        assert not name.startswith(('web/','docs/',TASK+'/','scripts/nova_illustration/')), name
        excluded.append(name)
    assert not git('diff','--cached','--name-only')
    report = {
        'baseline':BASE,'checked_head':git('rev-parse','HEAD').decode().strip(),
        'page_count':18,'panel_count':len(retained_panels)+sum(p['panels'] for p in pages),
        'spoken_words':retained_words+sum(p['spoken_words'] for p in pages),
        'original_spoken_words':130,'opening_spoken_words':retained_words,'new_script_pages':pages,
        'checks':['all 130 original spoken words retained; only Samir adds four words','pages 1 and 4 differ only in approved date stamps; page 3 and panel 2c exact','five original heads, existing poses and palette values unchanged','accepted studies, public lore art and old proof retained','first-draft script sources and proof hashes retained','fourteen script pages have ordered panel IDs and known speakers','public archive is demo-only; private script markers absent','built public lore art equals source','task remains OPEN with seven writing checks; index empty'],
        'public_lore_asset_sha256':assets,'protected_paths':protected,
        'unrelated_paths_not_validated':sorted(excluded),
    }
    target = ROOT/TASK/'proof/episode-feedback/source-and-script.json'
    target.write_text(json.dumps(report,indent=2)+'\n')
    print(f"Script and scope pass: {report['page_count']} pages, {report['panel_count']} panels, {report['spoken_words']} spoken words; public story remains demo-only")


if __name__=='__main__':
    main()
