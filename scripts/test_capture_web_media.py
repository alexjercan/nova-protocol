"""Exercise capture-web-media.sh against fake cargo, git and xvfb-run stubs."""
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/capture-web-media.sh'

# The news-0160 LOOPS rows, in script order: example -> loop.
NEWS_0160_EXAMPLES = [
    'news_0160_release_lead',
    'news_0160_world_setup',
    'news_0160_refit_catalogue',
    'news_0160_chart_view',
    'news_0160_station_orbit',
    'news_0160_escort_ambush',
]
NEWS_0160_LOOPS = [
    'news-0160-release-lead',
    'news-0160-world-setup',
    'news-0160-refit-catalogue',
    'news-0160-chart-view',
    'news-0160-station-orbit',
    'news-0160-escort-ambush',
    'news-0160-escort-done',
]

XVFB_RUN_STUB = """#!/usr/bin/env bash
set -euo pipefail
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        -a) shift ;;
        -s) shift 2 ;;
        *) break ;;
    esac
done
exec "$@"
"""

CARGO_STUB = """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >>"$FAKE_CARGO_LOG"
case "$1" in
    build)
        exit 0
        ;;
    run)
        example=""
        prev=""
        for arg in "$@"; do
            if [[ "$prev" == "--example" ]]; then
                example="$arg"
            fi
            prev="$arg"
        done
        printf '%s\n' "${WAYLAND_DISPLAY-<unset>}" >>"$FAKE_WAYLAND_LOG"
        loop="${example//news_0160_/news-0160-}"
        loop="${loop//_/-}"
        loops=("$loop")
        if [[ "$example" == "news_0160_escort_ambush" ]]; then
            loops+=("news-0160-escort-done")
        fi
        for loop in "${loops[@]}"; do
            if [[ "$loop" == "${FAKE_SKIP:-}" ]]; then
                continue
            fi
            if [[ "$loop" == "${FAKE_SMALL:-}" ]]; then
                fixture="$FAKE_FIXTURE_SMALL"
            elif [[ "$loop" == "${FAKE_SHORT:-}" ]]; then
                fixture="$FAKE_FIXTURE_SHORT"
            else
                fixture="$FAKE_FIXTURE_GOOD"
            fi
            cp "$fixture" "$NOVA_CAPTURE_DIR/${loop}.webm"
            if [[ "$loop" == "${FAKE_HEAVY:-}" ]]; then
                head -c 3500000 /dev/zero >>"$NOVA_CAPTURE_DIR/${loop}.webm"
            fi
        done
        exit 0
        ;;
    *)
        echo "unexpected cargo invocation: $*" >&2
        exit 1
        ;;
esac
"""

# A clean checkout at a fixed commit. FAKE_GIT_DIRTY=from-start dirties the
# worktree before the build; FAKE_GIT_DIRTY=after-build and FAKE_GIT_HEAD_MOVES=1
# change it once the fake cargo log records the build.
GIT_STUB = """#!/usr/bin/env bash
set -euo pipefail
built=0
grep -q '^build' "$FAKE_CARGO_LOG" && built=1
case "$*" in
    'status --porcelain --untracked-files=all')
        if [[ "${FAKE_GIT_DIRTY:-}" == from-start || ( "${FAKE_GIT_DIRTY:-}" == after-build && "$built" == 1 ) ]]; then
            echo '?? examples/screenshots/news_0160_chart_view.rs'
        fi
        ;;
    'rev-parse HEAD')
        if [[ "${FAKE_GIT_HEAD_MOVES:-}" == 1 && "$built" == 1 ]]; then
            echo 2222222222222222222222222222222222222222
        else
            echo 1111111111111111111111111111111111111111
        fi
        ;;
    'rev-parse --short HEAD') echo 1111111 ;;
    'rev-parse --abbrev-ref HEAD') echo master ;;
    *)
        echo "unexpected git invocation: $*" >&2
        exit 1
        ;;
esac
"""


def write_stub(path, content):
    path.write_text(content)
    path.chmod(path.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)


def read_lines(path):
    return path.read_text().splitlines()


class CaptureWebMediaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixtures_dir = tempfile.TemporaryDirectory()
        fixtures = Path(cls.fixtures_dir.name)
        codec = 'libvpx-vp9'
        probe = subprocess.run(
            ['ffmpeg', '-hide_banner', '-encoders'], capture_output=True, text=True, check=True
        )
        if 'libvpx-vp9' not in probe.stdout:
            codec = 'libvpx'
        cls.good_fixture = fixtures / 'good.webm'
        cls.small_fixture = fixtures / 'small.webm'
        cls.short_fixture = fixtures / 'short.webm'
        # cut_still seeks 12.0s into a packaged news-0160-release-lead.webm, so the
        # good fixture must run past that, not the ~1s a loop normally is.
        # The short fixture passes every loop check but ends before that seek.
        for path, size, duration in [
            (cls.good_fixture, '1280x720', 13),
            (cls.small_fixture, '640x360', 1),
            (cls.short_fixture, '1280x720', 1),
        ]:
            subprocess.run(
                [
                    'ffmpeg', '-v', 'error', '-y', '-f', 'lavfi',
                    '-i', f'color=c=blue:s={size}:d={duration}:r=10',
                    '-c:v', codec, '-crf', '30', '-b:v', '0', str(path),
                ],
                check=True,
            )

    @classmethod
    def tearDownClass(cls):
        cls.fixtures_dir.cleanup()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.out = self.root / 'out'
        self.out.mkdir()
        bin_dir = self.root / 'bin'
        bin_dir.mkdir()
        write_stub(bin_dir / 'xvfb-run', XVFB_RUN_STUB)
        write_stub(bin_dir / 'cargo', CARGO_STUB)
        write_stub(bin_dir / 'git', GIT_STUB)
        self.cargo_log = self.root / 'cargo.log'
        self.cargo_log.write_text('')
        self.wayland_log = self.root / 'wayland.log'
        self.wayland_log.write_text('')
        self.target_dir = self.root / 'target'
        self.env = dict(os.environ)
        self.env['PATH'] = f"{bin_dir}:{self.env['PATH']}"
        self.env['CARGO_TARGET_DIR'] = str(self.target_dir)
        self.env['FAKE_CARGO_LOG'] = str(self.cargo_log)
        self.env['FAKE_FIXTURE_GOOD'] = str(self.good_fixture)
        self.env['FAKE_FIXTURE_SMALL'] = str(self.small_fixture)
        self.env['FAKE_FIXTURE_SHORT'] = str(self.short_fixture)
        self.env['FAKE_WAYLAND_LOG'] = str(self.wayland_log)
        # A desktop session exports this; the producers must not see it.
        self.env['WAYLAND_DISPLAY'] = 'wayland-test'
        self.env['NOVA_NEWS_0160_ONLY'] = '1'
        for var in ['NOVA_REUSE_STAGE', 'NOVA_UNFREEZE', 'FAKE_GIT_DIRTY', 'FAKE_GIT_HEAD_MOVES']:
            self.env.pop(var, None)
        self.unrelated = self.out / 'news-0150-release-lead.webm'
        self.unrelated.write_bytes(b'frozen v0.15 footage, do not touch')
        self.manifest = self.out / 'manifest.txt'
        self.manifest.write_text(
            '# captured at commit deadbeef on master\n'
            '# a frozen row shipped with an earlier post and predates that commit\n'
            '# file\texample\tduration_s\tbytes\tstate\n'
            'loop-section-torpedo-bay.webm\tsystem_torpedo_launch\t3.2\t900000\tfrozen\n'
            'landing-wfc-2v2.webm\twfc_arena\t4.0\t1200000\tfrozen\n'
        )
        self.seeded_manifest = self.manifest.read_bytes()

    def run_script(self, **extra_env):
        env = dict(self.env)
        env.update(extra_env)
        return subprocess.run(
            ['bash', str(SCRIPT), str(self.out)],
            cwd=ROOT, env=env, capture_output=True, text=True,
        )

    def manifest_group(self):
        lines = read_lines(self.manifest)
        header_idx = [i for i, line in enumerate(lines) if line.startswith('# news-0160 group:')]
        return lines, header_idx

    def test_clean_pass_packages_every_news_0160_loop_and_cuts_the_v0_16_0_thumbnail(self):
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)

        cargo_lines = read_lines(self.cargo_log)
        build_lines = [line for line in cargo_lines if line.startswith('build')]
        run_lines = [line for line in cargo_lines if line.startswith('run')]
        self.assertEqual(len(build_lines), 1)
        for example in NEWS_0160_EXAMPLES:
            self.assertIn(f'--example {example}', build_lines[0])
        self.assertEqual(len(run_lines), len(NEWS_0160_EXAMPLES))
        self.assertEqual(read_lines(self.wayland_log), ['<unset>'] * len(NEWS_0160_EXAMPLES))

        for loop in NEWS_0160_LOOPS:
            self.assertTrue((self.out / f'{loop}.webm').is_file(), loop)

        lines, header_idx = self.manifest_group()
        self.assertEqual(len(header_idx), 1)
        header = lines[header_idx[0]]
        self.assertIn('1111111 on master', header)
        seeded_lines = self.seeded_manifest.decode().splitlines()
        self.assertEqual(lines[: len(seeded_lines)], seeded_lines)
        rows = lines[header_idx[0] + 1 :]
        self.assertEqual(len(rows), len(NEWS_0160_LOOPS))
        row_by_loop = {}
        for row in rows:
            file_name, example, duration, size, state = row.split('\t')
            row_by_loop[file_name] = (example, duration, size, state)
            self.assertRegex(duration, r'^\d+\.\d$')
            self.assertTrue(size.isdigit())
            self.assertEqual(state, 'fresh')
        self.assertEqual(row_by_loop['news-0160-escort-done.webm'][0], 'news_0160_escort_ambush')

        self.assertEqual(self.unrelated.read_bytes(), b'frozen v0.15 footage, do not touch')

        shots = self.target_dir / 'shots'
        self.assertTrue((shots / 'thumb-news-0.16.0.png').is_file())
        self.assertFalse((shots / 'thumb-news-0.15.0.png').exists())

    def test_shipped_v0_16_0_loop_refuses_without_unfreeze(self):
        (self.out / 'news-0160-chart-view.webm').write_bytes(b'already shipped')
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('news-0160-chart-view', result.stderr)
        self.assertEqual(self.cargo_log.read_text(), '')
        self.assertEqual(self.manifest.read_bytes(), self.seeded_manifest)

    def test_unfreeze_replaces_the_group_instead_of_duplicating_it(self):
        first = self.run_script()
        self.assertEqual(first.returncode, 0, first.stderr)
        lines_before, header_idx_before = self.manifest_group()
        self.assertEqual(len(header_idx_before), 1)

        second = self.run_script(NOVA_UNFREEZE='news-0160')
        self.assertEqual(second.returncode, 0, second.stderr)
        lines_after, header_idx_after = self.manifest_group()
        self.assertEqual(len(header_idx_after), 1)
        seeded_lines = self.seeded_manifest.decode().splitlines()
        self.assertEqual(lines_after[: len(seeded_lines)], seeded_lines)
        rows_after = lines_after[header_idx_after[0] + 1 :]
        self.assertEqual(len(rows_after), len(NEWS_0160_LOOPS))

    def test_manifest_group_without_unfreeze_and_missing_webms_refuses(self):
        first = self.run_script()
        self.assertEqual(first.returncode, 0, first.stderr)
        for loop in NEWS_0160_LOOPS:
            (self.out / f'{loop}.webm').unlink()

        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)

    def test_malformed_manifest_group_under_unfreeze_refuses_and_leaves_manifest_untouched(self):
        malformed = self.seeded_manifest.decode()
        malformed += '# news-0160 group: stale group a\nnews-0160-chart-view.webm\tnews_0160_chart_view\t3.0\t100\tfresh\n'
        malformed += '# news-0160 group: stale group b\nnews-0160-release-lead.webm\tnews_0160_release_lead\t3.0\t100\tfresh\n'
        self.manifest.write_bytes(malformed.encode())
        before = self.manifest.read_bytes()

        result = self.run_script(NOVA_UNFREEZE='news-0160')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.manifest.read_bytes(), before)

    def test_empty_or_missing_manifest_refuses_before_building(self):
        self.manifest.write_text('')
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.cargo_log.read_text(), '')

        self.manifest.unlink()
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.cargo_log.read_text(), '')

    def test_reuse_stage_refuses_a_news_0160_only_pass(self):
        result = self.run_script(NOVA_REUSE_STAGE='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.cargo_log.read_text(), '')

    def test_bad_capture_output_refuses_and_leaves_out_and_manifest_untouched(self):
        cases = [
            ('missing escort-done', {'FAKE_SKIP': 'news-0160-escort-done'}, 'news-0160-escort-done'),
            ('wrong dimensions', {'FAKE_SMALL': 'news-0160-world-setup'}, 'news-0160-world-setup'),
            ('overbudget', {'FAKE_HEAVY': 'news-0160-refit-catalogue'}, 'news-0160-refit-catalogue'),
        ]
        for name, extra_env, needle in cases:
            with self.subTest(name):
                result = self.run_script(**extra_env)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(needle, result.stderr)
                for loop in NEWS_0160_LOOPS:
                    self.assertFalse((self.out / f'{loop}.webm').exists())
                self.assertEqual(self.manifest.read_bytes(), self.seeded_manifest)

    def assert_nothing_shipped(self):
        for loop in NEWS_0160_LOOPS:
            self.assertFalse((self.out / f'{loop}.webm').exists())
        self.assertEqual(self.manifest.read_bytes(), self.seeded_manifest)
        self.assertFalse((self.out / 'manifest.txt.next').exists())

    def test_release_lead_too_short_for_the_thumbnail_refuses_and_leaves_out_and_manifest_untouched(self):
        before = {path.name: path.read_bytes() for path in self.out.iterdir()}
        stale = self.target_dir / 'shots' / 'thumb-news-0.16.0.png'
        stale.parent.mkdir(parents=True)
        stale.write_bytes(b'stale thumbnail from an earlier pass')

        result = self.run_script(FAKE_SHORT='news-0160-release-lead')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('thumb-news-0.16.0.png', result.stderr)
        self.assertEqual({path.name: path.read_bytes() for path in self.out.iterdir()}, before)
        self.assert_nothing_shipped()
        self.assertFalse(stale.exists())

    def test_dirty_worktree_refuses_before_building_and_before_shipping(self):
        result = self.run_script(FAKE_GIT_DIRTY='from-start')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('news_0160_chart_view.rs', result.stderr)
        self.assertEqual(self.cargo_log.read_text(), '')
        self.assert_nothing_shipped()

        result = self.run_script(FAKE_GIT_DIRTY='after-build')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('news_0160_chart_view.rs', result.stderr)
        self.assertEqual(len(read_lines(self.wayland_log)), len(NEWS_0160_EXAMPLES))
        self.assert_nothing_shipped()

    def test_head_moved_during_the_pass_refuses_before_shipping(self):
        result = self.run_script(FAKE_GIT_HEAD_MOVES='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('HEAD moved', result.stderr)
        self.assertEqual(len(read_lines(self.wayland_log)), len(NEWS_0160_EXAMPLES))
        self.assert_nothing_shipped()

    def test_default_pass_runs_producers_without_wayland(self):
        # The stub writes no hero-wfc-duel.webm, so the pass stops after its
        # first producer; that run is the one under test.
        self.run_script(NOVA_NEWS_0160_ONLY='0')
        self.assertEqual(read_lines(self.wayland_log), ['<unset>'])


if __name__ == '__main__':
    unittest.main()
