#!/usr/bin/env python3
"""Publish one owned part of gh-pages without using the source checkout's index."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
from urllib.request import Request, urlopen


def validate_source(source, kind):
    """Reject incomplete artifacts, Git metadata, and links before changing files."""
    if source.is_symlink() or not source.is_dir():
        raise ValueError('Deployment source must be a directory, not a link')
    for file in source.rglob('*'):
        if file.is_symlink() or '.git' in [part.lower() for part in file.relative_to(source).parts]:
            raise ValueError(f'Unsafe deployment input: {file}')
        if not file.is_file() and not file.is_dir():
            raise ValueError(f'Unsupported deployment input: {file}')
    if kind == 'story':
        if {p.name for p in source.iterdir()} != {'story'}:
            raise ValueError('Comic artifacts must contain only story/')
        index = source / 'story/index.html'
    elif kind == 'site':
        index = source / 'index.html'
    else:
        raise ValueError(f'Unknown deployment kind: {kind}')
    if not index.is_file():
        raise ValueError(f'Missing deployment index: {index}')


def remove(path):
    """Remove an owned entry without following links."""
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists():
        shutil.rmtree(path)


def merge_site(source, destination, kind):
    """Replace Story or the surrounding site, retaining the other owner's files."""
    validate_source(source, kind)
    source, destination = source.resolve(), destination.resolve()
    if source == destination or source in destination.parents or destination in source.parents:
        raise ValueError('Deployment input and output must be separate trees')
    if kind == 'site' and (destination / 'story').exists():
        marker = destination / 'story/release.json'
        if (destination / 'story').is_symlink() or not marker.is_file() or marker.is_symlink():
            raise ValueError('Publish a comic bootstrap tag before replacing a site with legacy Story files')
        release = json.loads(marker.read_text())
        if not isinstance(release, dict) or set(release) != {'tag', 'revision'} or not isinstance(release['tag'], str) or not isinstance(release['revision'], str) or not re.fullmatch(r'comic-[a-z0-9][a-z0-9.-]*', release['tag']) or not re.fullmatch(r'[0-9a-f]{40}', release['revision']):
            raise ValueError('Invalid published comic release record')
    destination.mkdir(parents=True, exist_ok=True)
    if kind == 'story':
        remove(destination / 'story')
        shutil.copytree(source / 'story', destination / 'story')
    else:
        for entry in destination.iterdir():
            if entry.name not in {'story', 'CNAME', '.nojekyll'}:
                remove(entry)
        for entry in source.iterdir():
            if entry.name in {'story', 'CNAME', '.nojekyll'}:
                continue
            target = destination / entry.name
            if entry.is_dir():
                shutil.copytree(entry, target)
            else:
                shutil.copyfile(entry, target)
    if (destination / '.nojekyll').is_symlink():
        raise ValueError('The hosting marker must not be a link')
    (destination / '.nojekyll').touch()


def publish(repo, source, kind, revision, tag):
    """Commit a scoped merge onto the latest gh-pages tip; never force-push."""
    validate_source(source, kind)
    if not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise ValueError('A complete source commit is required')
    if kind == 'story' and not re.fullmatch(r'comic-[a-z0-9][a-z0-9.-]*', tag):
        raise ValueError('A comic-* tag is required')
    if kind == 'site' and tag:
        raise ValueError('Site deployment does not accept a comic tag')

    def git(*args, env=None):
        return subprocess.check_output(['git', *args], cwd=env['GIT_WORK_TREE'] if env else repo, env=env, text=True).strip()

    if git('rev-parse', 'HEAD') != revision:
        raise ValueError('The publisher checkout must match the built source commit')
    remote = subprocess.run(['git', 'ls-remote', '--exit-code', '--heads', 'origin', 'refs/heads/gh-pages'], cwd=repo, capture_output=True, text=True)
    if remote.returncode not in {0, 2}:
        raise RuntimeError(remote.stderr)
    parent = None
    if remote.returncode == 0:
        git('fetch', '--no-tags', 'origin', 'refs/heads/gh-pages')
        parent = git('rev-parse', 'FETCH_HEAD')

    with tempfile.TemporaryDirectory(prefix='nova-pages-') as temporary:
        work = Path(temporary) / 'tree'
        work.mkdir()
        env = dict(os.environ, GIT_DIR=git('rev-parse', '--absolute-git-dir'), GIT_WORK_TREE=str(work), GIT_INDEX_FILE=str(Path(temporary) / 'index'))
        if parent:
            git('read-tree', parent, env=env)
            git('checkout-index', '--all', env=env)
        else:
            git('read-tree', '--empty', env=env)
        merge_site(source, work, kind)
        if kind == 'story':
            (work / 'story/release.json').write_text(json.dumps({'tag': tag, 'revision': revision}, indent=2) + '\n')
        git('add', '-A', '--', '.', env=env)
        tree = git('write-tree', env=env)
        if parent and tree == git('rev-parse', parent + '^{tree}'):
            print('Published files are already current')
            return parent
        args = ['-c', 'user.name=github-actions[bot]', '-c', 'user.email=41898282+github-actions[bot]@users.noreply.github.com', 'commit-tree', tree]
        if parent:
            args += ['-p', parent]
        args += ['-m', f'Publish {tag or "site"} from {revision}']
        commit = git(*args, env=env)
        git('push', 'origin', f'{commit}:refs/heads/gh-pages', env=env)
        print(f'Published {kind}: {commit}')
        return commit


def pages_api(repository, token, endpoint='', method='GET'):
    """Call the Pages API without putting credentials in commands or logs."""
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository) or not token:
        raise ValueError('GitHub repository and token are required')
    request = Request(f'https://api.github.com/repos/{repository}/pages{endpoint}', method=method, headers={
        'Authorization': f'Bearer {token}', 'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
    })
    with urlopen(request, timeout=30) as response:
        return json.load(response)


def check_pages(repository, token):
    """Require branch-based Pages hosting from gh-pages at the root."""
    settings = pages_api(repository, token)
    if settings.get('build_type') != 'legacy' or settings.get('source') != {'branch': 'gh-pages', 'path': '/'}:
        raise ValueError('Configure Pages to deploy from the gh-pages branch root')


def build_pages(repository, token, revision):
    """Explicitly build token-pushed commits and wait while holding the publish lock."""
    pages_api(repository, token, '/builds', 'POST')
    for _ in range(120):
        build = pages_api(repository, token, '/builds/latest')
        if build.get('commit') == revision:
            if build.get('status') == 'built':
                return
            if build.get('status') == 'errored':
                raise RuntimeError(f'Pages build failed: {build.get("error")}')
        time.sleep(5)
    raise TimeoutError(f'Pages did not finish building {revision}')


def main():
    """Publish an already-built artifact from the matching source checkout."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--kind', choices=['story', 'site'], required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--tag', default='')
    parser.add_argument('--github-pages', action='store_true', help='Check hosting settings and request/wait for the Pages build')
    args = parser.parse_args()
    repository, token = os.environ.get('GITHUB_REPOSITORY', ''), os.environ.get('GITHUB_TOKEN', '')
    if args.github_pages:
        check_pages(repository, token)
    commit = publish(Path.cwd(), args.source.absolute(), args.kind, args.revision, args.tag)
    if args.github_pages:
        build_pages(repository, token, commit)


if __name__ == '__main__':
    main()
