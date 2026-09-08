#!/usr/bin/env python3
"""Verify this revision against its intake workspace and prefixed website build."""

import ast
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
WORK = Path(sys.argv[1])
PROOF = Path(__file__).resolve().parent / 'clean-line'
TASK = 'tasks/20260908-161328/'
before = json.loads((WORK / 'before.json').read_text())


def digest(path):
    """Hash bytes rather than treating a successful build as proof of unchanged assets."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def functions(path):
    """Read function bodies without executing historical scene code."""
    return {node.name: node for node in ast.parse(path.read_text()).body if isinstance(node, ast.FunctionDef)}


allowed = {
    'scripts/gen-lore-portraits.py', 'scripts/gen-lore-designs.py',
    'scripts/nova_illustration/faces.py', 'scripts/nova_illustration/portraits.py',
    'scripts/nova_illustration/ships.py', 'scripts/nova_illustration/colors.py',
    'scripts/nova_illustration/test_illustration.py', 'scripts/nova_illustration/README.md',
    'web/src/lore/README.md', TASK+'TASK.md', TASK+'STYLE-REVIEW.md',
}
exports = [f'web/src/assets/lore/{name}.svg' for name in (
    'kaveri-design-concept', 'ebro-design-concept', 'elena-ward-portrait-concept')]
allowed.update(exports)
allowed_prefixes = (TASK+'clean-line-study/', TASK+'proof/clean-line/', TASK+'proof/clean-line-before-frontal/')
allowed.add(TASK+'proof/check-frontal-revision.py')
paths = set(subprocess.check_output(['git','ls-files','-co','--exclude-standard','-z'], cwd=ROOT).decode().split('\0')) - {''}
changed = sorted(p for p in set(before) | paths if (ROOT/p).is_file() and before.get(p) != digest(ROOT/p))
deleted = sorted(p for p in before if not (ROOT/p).is_file())
unexpected = [p for p in changed+deleted if p not in allowed and not p.startswith(allowed_prefixes)]
assert not unexpected, unexpected

scene = TASK+'clean-line-study/generate.py'
old, new = functions(WORK/scene), functions(ROOT/scene)
for name in ('definitions','mug_hand','render_svg','render_review'):
    assert ast.dump(old[name]) == ast.dump(new[name]), name


def layout(node):
    """Ignore the revised face function name and descriptive prose, not drawing arguments."""
    node = copy.deepcopy(node)
    for child in ast.walk(node):
        if isinstance(child, ast.Name) and child.id == 'elena_profile':
            child.id = 'elena_gesture'
        if isinstance(child, ast.Call) and isinstance(child.func, ast.Name) and child.func.id == 'Study':
            child.args[2:4] = [ast.Constant('purpose'), ast.Constant('description')]
    return ast.dump(node)


assert layout(old['studies']) == layout(new['studies']), 'Scene layout or dialogue changed'
poses = 'scripts/nova_illustration/portraits.py'
old, new = functions(WORK/poses), functions(ROOT/poses)
for old_name,new_name in [('elena_close','elena_close'),('elena_profile','elena_gesture'),('jonah_listener','jonah_listener')]:
    body = new[new_name].body[1:-2]
    assert [ast.dump(n) for n in body] == [ast.dump(n) for n in old[old_name].body[1:1+len(body)]], new_name

exporter = ROOT/'scripts/gen-lore-designs.py'
previous = exporter.read_text().replace('A forward-facing portrait study of Elena Ward.', 'An angle-specific portrait study of Elena Ward.')
assert hashlib.sha256(previous.encode()).hexdigest() == before['scripts/gen-lore-designs.py'], 'Export layout changed'
for name in ('styles.py','scenery.py','lettering.py'):
    path = 'scripts/nova_illustration/'+name
    assert digest(ROOT/path) == before[path], path

site = WORK/'site/nova-protocol'
for path in exports:
    assert (site/path.removeprefix('web/src/')).read_bytes() == (ROOT/path).read_bytes(), path
for path in exports+[TASK+f'clean-line-study/{name}.svg' for name in ('01-close-up','02-window','03-exterior')]:
    document = ET.fromstring((ROOT/path).read_bytes())
    ids = [n.get('id') for n in document.iter() if n.get('id')]
    assert len(ids) == len(set(ids)), path
    for node in document.iter():
        assert node.tag.split('}')[-1] not in ('script','foreignObject','image'), path
        for value in node.attrib.values():
            for key in re.findall(r'url\(#([^)]*)\)', value):
                assert key in ids, (path,key)
assert sorted(p.name for p in (site/'story').iterdir()) == ['demo','index.html']
for path in site.rglob('*'):
    if path.suffix in ('.html','.js'):
        content = path.read_text()
        assert not any(word in content for word in ('PRIVATE STYLE STUDY','clean-line-study','calendar date TBD','20260908-161328')), path

for path in changed:
    source = ROOT/path
    if source.suffix in ('.py','.md'):
        text = source.read_text()
        assert text.endswith('\n') and all(line.rstrip() == line for line in text.splitlines()), path
        if source.suffix == '.py' and source.name not in ('colors.py','gen-lore-portraits.py','check-frontal-revision.py'):
            assert not re.search(r'#[0-9a-fA-F]{6}\b', text), path

report = {
    'head': subprocess.check_output(['git','rev-parse','HEAD'], cwd=ROOT, text=True).strip(),
    'changed_paths': changed, 'unexpected_changes': [],
    'scene_layout_and_dialogue_unchanged': True, 'body_pose_geometry_unchanged': True,
    'sheet_layout_unchanged': True, 'lore_shader_and_scenery_unchanged': True,
    'original_four_page_poc_and_five_portraits_unchanged': True,
    'game_sources_and_website_media_unchanged': True, 'changelog_unchanged_this_batch': True,
    'private_drafts_excluded': True, 'public_comics': ['demo'],
    'public_exports_copied_exactly': exports, 'unit_tests': 13,
    'full_ci_run': False, 'rust_checks_or_new_capture_run': False, 'committed': False,
}
(PROOF/'source-and-publication.json').write_text(json.dumps(report,indent=2)+'\n')
(PROOF/'batch-scope.json').write_text(json.dumps(report,indent=2)+'\n')
print('Framing, dialogue, body poses, shader, baseline assets, and publication boundaries passed.')
print(f'{len(changed)} scoped changes; no unexpected changes.')
