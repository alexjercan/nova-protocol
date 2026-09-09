"""Exercise scoped Pages publishing with local repositories and mocked HTTP only."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('deploy_pages', Path(__file__).with_name('deploy-pages.py'))
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


def put(root, name, text):
    file = root / name
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(text)


def files(root):
    return {str(f.relative_to(root)): f.read_bytes() for f in root.rglob('*') if f.is_file()}


class PublishingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.site = self.root / 'site'
        self.story = self.root / 'comic'
        self.destination = self.root / 'published'
        put(self.site, 'index.html', 'New home')
        put(self.site, 'play/game.wasm', 'New game')
        put(self.site, 'story/index.html', 'Must not overwrite independent story')
        put(self.story, 'story/index.html', 'New story')
        put(self.story, 'story/reader.js', 'Reader')
        put(self.destination, 'index.html', 'Old home')
        put(self.destination, 'play/game.wasm', 'Old game')
        put(self.destination, 'story/index.html', 'Old story')
        put(self.destination, 'story/stale.svg', 'Retired story art')
        put(self.destination, 'story/release.json', json.dumps({'tag': 'comic-reader-1', 'revision': 'a' * 40}))
        put(self.destination, 'old-site.css', 'Retired site style')
        put(self.destination, 'CNAME', 'example.invalid')
        put(self.destination, '.nojekyll', '')

    def test_story_update_and_rollback_leave_every_other_file_exact(self):
        original = files(self.destination)
        deploy.merge_site(self.story, self.destination, 'story')
        result = files(self.destination)
        self.assertEqual({k: v for k, v in original.items() if not k.startswith('story/')}, {k: v for k, v in result.items() if not k.startswith('story/')})
        self.assertNotIn('story/stale.svg', result)
        self.assertEqual(result['story/index.html'], b'New story')
        put(self.story, 'story/index.html', 'Earlier release')
        deploy.merge_site(self.story, self.destination, 'story')
        self.assertEqual((self.destination / 'story/index.html').read_text(), 'Earlier release')
        self.assertEqual((self.destination / 'play/game.wasm').read_text(), 'Old game')

    def test_site_update_preserves_story_and_hosting_configuration(self):
        original = files(self.destination)
        deploy.merge_site(self.site, self.destination, 'site')
        result = files(self.destination)
        for name in ['story/index.html', 'story/stale.svg', 'CNAME', '.nojekyll']:
            self.assertEqual(result[name], original[name])
        self.assertEqual(result['play/game.wasm'], b'New game')
        self.assertNotIn('old-site.css', result)

    def test_legacy_story_must_be_bootstrapped_before_site_files_are_replaced(self):
        (self.destination / 'story/release.json').unlink()
        original = files(self.destination)
        with self.assertRaisesRegex(ValueError, 'bootstrap'):
            deploy.merge_site(self.site, self.destination, 'site')
        self.assertEqual(files(self.destination), original)

    def test_new_site_does_not_publish_a_comic_from_its_source_revision(self):
        empty = self.root / 'empty'
        deploy.merge_site(self.site, empty, 'site')
        self.assertFalse((empty / 'story').exists())
        self.assertTrue((empty / '.nojekyll').is_file())

    def test_invalid_inputs_fail_before_changing_the_published_tree(self):
        original = files(self.destination)
        put(self.story, 'index.html', 'Outside the owned directory')
        with self.assertRaises(ValueError):
            deploy.merge_site(self.story, self.destination, 'story')
        (self.story / 'index.html').unlink()
        (self.story / 'story/link').symlink_to(self.site, target_is_directory=True)
        with self.assertRaises(ValueError):
            deploy.merge_site(self.story, self.destination, 'story')
        self.assertEqual(files(self.destination), original)
        with self.assertRaises(ValueError):
            deploy.merge_site(self.site, self.site / 'nested', 'site')
        with self.assertRaises(ValueError):
            deploy.validate_source(self.root, 'unknown')

    def test_publish_bootstraps_reuses_latest_tip_and_leaves_source_index_untouched(self):
        remote, repo = self.root / 'remote.git', self.root / 'repo'
        def git(*args, cwd=repo):
            return subprocess.check_output(['git', *args], cwd=cwd, text=True, stderr=subprocess.DEVNULL).strip()
        git('init', '--bare', str(remote), cwd=self.root)
        repo.mkdir()
        git('init', '-b', 'master')
        git('config', 'user.name', 'Fixture')
        git('config', 'user.email', 'fixture@example.invalid')
        git('remote', 'add', 'origin', str(remote))
        put(repo, 'source.txt', 'Source stays unchanged')
        git('add', 'source.txt')
        git('commit', '-m', 'Fixture source')
        revision = git('rev-parse', 'HEAD')
        original_index = (repo / '.git/index').read_bytes()
        first = deploy.publish(repo, self.site, 'site', revision, '')
        second = deploy.publish(repo, self.story, 'story', revision, 'comic-s01e01')
        self.assertEqual(git('rev-parse', second + '^'), first)
        self.assertEqual(git('show', second + ':play/game.wasm'), 'New game')
        self.assertEqual(json.loads(git('show', second + ':story/release.json')), {'tag': 'comic-s01e01', 'revision': revision})
        self.assertEqual(deploy.publish(repo, self.story, 'story', revision, 'comic-s01e01'), second)
        put(self.site, 'index.html', 'Another site release')
        third = deploy.publish(repo, self.site, 'site', revision, '')
        self.assertEqual(git('show', third + ':story/index.html'), 'New story')
        self.assertEqual(git('rev-parse', third + '^'), second)
        self.assertEqual(git('rev-parse', 'HEAD'), revision)
        self.assertEqual((repo / '.git/index').read_bytes(), original_index)
        with self.assertRaises(ValueError):
            deploy.publish(repo, self.story, 'story', '0' * 40, 'comic-s01e01')
        with self.assertRaises(ValueError):
            deploy.publish(repo, self.story, 'story', revision, 'v1.0.0')
        output = subprocess.check_output
        def race(command, **kwargs):
            if command[:2] == ['git', 'push']:
                rival = git('commit-tree', git('rev-parse', third + '^{tree}'), '-p', third, '-m', 'Concurrent publisher')
                git('--git-dir=' + str(remote), 'update-ref', 'refs/heads/gh-pages', rival)
            return output(command, **kwargs)
        put(self.story, 'story/index.html', 'Losing concurrent attempt')
        with patch.object(subprocess, 'check_output', side_effect=race):
            with self.assertRaises(subprocess.CalledProcessError):
                deploy.publish(repo, self.story, 'story', revision, 'comic-s01e02')
        self.assertEqual((repo / '.git/index').read_bytes(), original_index)
        self.assertEqual(git('--git-dir=' + str(remote), 'show', 'gh-pages:story/index.html'), 'New story')


class WorkflowTests(unittest.TestCase):
    def test_publishers_share_one_lock_and_request_a_pages_build(self):
        root = Path(__file__).resolve().parents[1]
        for name in ['deploy-page.yaml', 'deploy-comic.yaml']:
            source = (root / '.github/workflows' / name).read_text()
            self.assertEqual(source.count('group: github-pages-publish'), 1)
            self.assertIn('cancel-in-progress: false', source)
            self.assertIn('pages: write', source)
            self.assertIn('--github-pages', source)
        comic = (root / '.github/workflows/deploy-comic.yaml').read_text()
        self.assertIn('tags: ["comic-*"]', comic)
        self.assertIn('ref: refs/tags/${{ steps.tag.outputs.name }}', comic)
        self.assertIn('npm run build:story', comic)
        self.assertNotIn('npm run build:site', comic)
        self.assertNotIn('npm run test:deploy', comic)
        self.assertNotIn('trunk build', comic)
        self.assertIn('npm run build:site', (root / '.github/workflows/deploy-page.yaml').read_text())


class PagesApiTests(unittest.TestCase):
    def test_hosting_must_use_the_expected_branch_root(self):
        with patch.object(deploy, 'pages_api', return_value={'build_type': 'legacy', 'source': {'branch': 'gh-pages', 'path': '/'}}):
            deploy.check_pages('owner/repo', 'token')
        for settings in [{'build_type': 'workflow'}, {'build_type': 'legacy', 'source': {'branch': 'master', 'path': '/'}}]:
            with patch.object(deploy, 'pages_api', return_value=settings):
                with self.assertRaises(ValueError):
                    deploy.check_pages('owner/repo', 'token')

    def test_api_build_waits_for_its_exact_commit(self):
        with patch.object(deploy, 'pages_api', side_effect=[{}, {'commit': 'older', 'status': 'built'}, {'commit': 'wanted', 'status': 'built'}]) as api, patch.object(deploy.time, 'sleep'):
            deploy.build_pages('owner/repo', 'token', 'wanted')
            self.assertEqual(api.call_args_list[0].args, ('owner/repo', 'token', '/builds', 'POST'))
        with patch.object(deploy, 'pages_api', return_value={'commit': 'wanted', 'status': 'errored'}), patch.object(deploy.time, 'sleep'):
            with self.assertRaises(RuntimeError):
                deploy.build_pages('owner/repo', 'token', 'wanted')
        with patch.object(deploy, 'pages_api', return_value={'commit': 'older', 'status': 'built'}), patch.object(deploy.time, 'sleep'):
            with self.assertRaises(TimeoutError):
                deploy.build_pages('owner/repo', 'token', 'wanted')


if __name__ == '__main__':
    unittest.main()
