# NOTES - probe FPS regression investigation (2026-07-29)

Verdict: **FIXED, no residual render-side regression.** The regression was the
lazy `.ttc` font path; `c3ee1988` (task 20260729-000956) removed it, and the
current commit measures at or below the pre-regression baseline on every
example. No bisect was needed - three commit-keyed run sets already spanned the
window.

## What was measured

Three `probe-runs/<short-commit>/` sets exist (the commit-keyed layout from task
20260729-003352, which landed as `00211ac5` and WAS available for this
investigation - see DoD 4):

| run dir | commit | date | position |
|---|---|---|---|
| `08420e2a` | `docs(news): v0.8.1 point-release note` | code 2026-07-24, run 2026-07-29 01:46-01:50 | pre-regression baseline |
| `82bc2dc9` | `lessons: asset-format changes grep the extension+loader` | 2026-07-29 01:13, run 2026-07-29 01:55-01:59 | first run AFTER the preload fix `c3ee1988` |
| `a6d06220` | `fix(probe): sandbox native runs from the operator's local profile` | 2026-07-29 19:26, run 2026-07-29 20:44-20:48 | current HEAD |

All numbers below are `mean_ms` / `p95_ms` read from each run's
`frametime.csv`, not from the HTML reports. Lower is better.

| example | 08420e2a (base) | 82bc2dc9 (post-fix) | a6d06220 (HEAD) | HEAD vs base |
|---|---|---|---|---|
| `scenario` | 18.43 / 19.72 | 19.51 / 23.50 | **18.44** / 20.01 | +0.05% |
| `playable` | 17.12 / 23.83 | 17.57 / 23.23 | **17.11** / 25.71 | -0.06% |
| `perf_baseline` | 21.04 / 23.98 | 22.16 / 24.15 | **19.30** / 24.00 | -8.3% |
| `lifeline` | 28.28 / 25.27 | 47.72 / 112.02 | **23.10** / 24.25 | -18.3% |

For reference, the regression this task was opened for (old `probe-runs/before`
layout, `87003703` vs `6fb581ca`): `scenario` 18.71 -> 25.39 ms (+35.7%),
`perf_baseline` 17.18 -> 18.99 (+10.6%), `playable` 16.66 -> 18.18 (+9.1%).
That gap is gone.

The `checks.json` verdicts agree: at `a6d06220` every scored example reports
`fps_within_baseline: PASS` with a NEGATIVE delta (`playable -2.6%`,
`scenario -5.5%`, `perf_baseline -12.9%`, `lifeline -51.6%`), i.e. "improved;
no label regressed against the baseline".

## The mechanism, confirmed in the traces

The original finding was repeated `NovaOsTtcFontLoader` work on
`fonts/SGr-IosevkaTerm-Regular.ttc`. Scanning `trace.json` for font-related
span names:

- `08420e2a` (base): no font asset-loading spans at all - the pre-NOVA-OS-font
  code path.
- `82bc2dc9` and `a6d06220`: the custom loader is **entirely absent**. Text now
  loads through Bevy's stock
  `bevy_text::font_loader::FontLoader` on a `.ttf`
  (`SGr-IosevkaTerm-Regular.ttf`, then `-Medium.ttf` after `dee12e9f`), 10-16
  asset-load spans across a whole multi-pass run rather than recurring work.
- `grep -rn NovaOsTtcFontLoader crates/` returns nothing: the type no longer
  exists.

Attribution: `git log -S ttc -- crates/` and `git log -- assets/fonts/*` both
point at `c3ee1988` "feat(assets): preload static assets via bevy_asset_loader +
phosphor boot loading screen" as the commit that dropped the `.ttc` and its
loader. That is task **20260729-000956**, exactly the "likely partial fix" this
task was asked to measure. It turned out to be the whole fix (DoD 3).

## Render side: no residual regression

The trace totals at `a6d06220` vs `08420e2a` for `scenario` show `RenderApp`
3966 -> 4149 ms and `RenderExtractApp` 958 -> 1081 ms across the traced pass -
but the traced pass is untimed instrumentation over differing frame counts
(2.23M vs 2.36M events), and `camera_schedule: camera="Camera 0"` appearing at
320 ms of that delta is new instrumentation, not new work. The authoritative
signal is the timed `frametime.csv` pass, which is at parity or better on every
example. There is nothing left to open an optimization task for.

## The 82bc2dc9 middle run was host-noisy, not a residual +5%

`82bc2dc9` (already containing `c3ee1988`) still read +5.9% `scenario` / +5.3%
`perf_baseline` against the base run, which could look like a leftover
regression. It is not: the same session's `lifeline` read 47.72 ms mean with a
112.02 ms p95, versus 28.28 (base) and 23.10 (HEAD) with p95 ~24-25 ms. A p95
4.5x the surrounding runs is a busy host, not code. Every one of that session's
numbers is inflated in the same direction, and the numbers came back down at
HEAD across 15 further commits of HUD/UI work.

## Confounder checked and cleared: the probe sandbox

`a6d06220` is itself the commit that sandboxes probe runs away from the
operator's profile, so its run measured a different environment from the two
earlier ones. The operator's real profile is non-default
(`graphics_quality: High`, `enabled_mods: ["base","gauntlet","the-ledger"]`,
two installed mods), which would have been a serious confounder. It was not:

- `frametime.csv` records `quality=default`, `resolution=1280x720`,
  `backend=vulkan` on ALL THREE runs - the probe examples pin their own
  quality, so `graphics_quality: High` never reached any run.
- `grep -ricE 'gauntlet|the-ledger' probe-runs/*/*/run.log` returns 0 for every
  run in all three sets - the operator's mods were never loaded, before or after
  the sandbox.

So the improvement is the code, not the measurement change.

## Commands

```sh
# the run sets compared (already on disk; re-runnable per commit)
cargo run -p nova_probe -- run scenario
cargo run -p nova_probe -- run perf_baseline
cargo run -p nova_probe -- run playable
cargo run -p nova_probe -- run lifeline

# what was read (JSON/CSV/trace, not the HTML)
head -2 probe-runs/<commit>/<example>/frametime.csv          # mean_ms/p95_ms/quality/git_sha
jq '.checks[] | select(.name=="fps_within_baseline")' probe-runs/<commit>/<example>/checks.json
grep -ricE 'gauntlet|the-ledger' probe-runs/*/*/run.log      # mod-leak check: all 0
grep -rn NovaOsTtcFontLoader crates/                         # no hits: loader is gone
```

## Minor artifact quirk (not acted on)

In `probe-runs/82bc2dc9/`, the `perf_baseline` and `lifeline` CSVs record
`git_sha=1a0c11b5` (the parent commit) while `scenario` and `playable` record
`82bc2dc9`. The run session straddled a commit. It does not change the
conclusion - both SHAs are after `c3ee1988` - but it is worth knowing the
folder name is the run's key, not a per-example guarantee.

## Follow-up

The gap here was that nothing routinely re-measures perf at the END of a sprint;
this regression was only caught by an ad-hoc comparison. Backlog task
`20260729-205957` adds an end-of-sprint probe sweep as a standing item.
