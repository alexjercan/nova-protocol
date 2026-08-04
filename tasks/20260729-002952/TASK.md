# Investigate probe FPS regression from July 29 runs

- PRIORITY: 80
- TAGS: v0.9.0, bug, perf, probe
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

The July 29 probe report shows a small but consistent FPS regression versus the
July 20 baseline. The current run artifacts compare `probe-runs/before/*` from
`87003703` against `probe-runs/*` from `6fb581ca`; `scenario` warns at +35.7%
frame time, `perf_baseline` warns at +10.6%, and `playable` is close to the
10% gate at +9.1%. The trace data points first at the NOVA OS render path:
`RenderExtractApp` and `RenderApp` grew, and the current traces include repeated
`NovaOsTtcFontLoader` work for `fonts/SGr-IosevkaTerm-Regular.ttc`.

This task should investigate the regression without assuming the static-asset
fix is the whole answer. It should use commit-keyed probe artifacts once task
20260729-003352 lands, but it does not own the nova_probe workflow change.

## Steps

- [x] Record the current evidence in NOTES.md from the JSON/CSV/trace data, not
      from the HTML reports: current/before git SHAs, affected examples,
      `frametime.csv` deltas, `checks.json` verdicts, and top trace deltas for
      render/extract/font loading.
- [x] Isolate the regression range with targeted probe runs or a small bisect
      across likely commits: at minimum `87600482` (Iosevka .ttc), `14aea590`
      (NOVA OS render-to-texture CRT), `dec4ee9f` (nova_os extraction + loader),
      and current HEAD. Keep the run artifacts reusable; prefer the commit-keyed
      layout from task 20260729-003352 if it is available.
      NOT NEEDED as a bisect: three commit-keyed run sets (`08420e2a`,
      `82bc2dc9`, `a6d06220`) already bracket the window, and `git log -S ttc`
      attributes the `.ttc` loader's removal to `c3ee1988` unambiguously.
- [x] Coordinate with task 20260729-000956: treat its preload/static-asset work
      as a likely partial fix for the repeated font-load finding, but verify
      whether render extraction remains elevated after that task lands.
      It was the WHOLE fix; render extraction is not elevated in the timed pass.
- [x] Run before/after probe checks and update NOTES.md
      with the final interpretation: whether the FPS drop was removed, reduced,
      or still open for a follow-up optimization.

## Definition of Done

1. notes: `tasks/<id>/NOTES.md` contains the measured regression summary and the
   commit attribution evidence from JSON/CSV/trace files.
2. cmd: targeted probe command(s) for the affected example(s) are recorded in
   NOTES.md with their resulting `checks.json` / `frametime.csv` deltas.
3. manual: final report states whether task 20260729-000956 resolved the font
   loading part, and whether any remaining render-side regression needs a new
   optimization task.
4. manual: final report states whether task 20260729-003352 was available for
   the investigation and, if not, how the run folders were kept comparable.

## Notes

- Related WIP: 20260729-000956 (`Preload static assets via bevy_asset_loader +
  phosphor boot loading screen`) should address the lazy font/static-asset side
  of the finding. This task owns measuring whether that is sufficient.
- Split task: 20260729-003352 owns the nova_probe commit-keyed
  `probe-runs/<short-commit>/<example>/...` layout, automatic baseline discovery,
  and multi-item `nova_probe render` fix.
- Current evidence from 2026-07-29 inspection: `scenario` mean rose from
  18.7076 ms to 25.3912 ms, `perf_baseline` from 17.1788 ms to 18.9941 ms,
  and `playable` from 16.6648 ms to 18.1831 ms. Current run SHA was
  `6fb581ca`; HEAD at inspection had later task/asset commits not represented
  in the probe report.
- Likely code surfaces: `crates/nova_gameplay/src/hud/nova_os.rs` RTT setup,
  `NovaOsTtcFontLoader`, `nova_os_font`, and `reconcile_nova_os_target`.
- Do not hand-edit historical probe artifacts as part of the investigation;
  new structure should coexist with old folders until there is a deliberate
  cleanup task.

## Outcome (2026-07-29) - FIXED

Closed as FIXED. Full evidence in NOTES.md; the short version:

- The regression is GONE at `a6d06220`. Measured `mean_ms` vs the pre-regression
  `08420e2a` baseline: `scenario` 18.44 vs 18.43, `playable` 17.11 vs 17.12,
  `perf_baseline` 19.30 vs 21.04, `lifeline` 23.10 vs 28.28. Every scored
  example's `fps_within_baseline` check reads PASS with a negative delta.
- Cause and fix: the lazy `NovaOsTtcFontLoader` `.ttc` path. `c3ee1988` (task
  20260729-000956) replaced it with a preloaded `.ttf` through Bevy's stock
  `FontLoader`; the custom loader no longer exists in the tree or in the traces.
- No residual render-side regression, so NO follow-up optimization task (DoD 3).
- Task 20260729-003352's commit-keyed layout WAS available and is what made the
  comparison possible without a bisect (DoD 4).
- No bisect was run, per the owner's direction - the existing run sets and the
  `git log -S ttc` attribution were sufficient.
- Follow-up filed: backlog task `20260729-205957` makes an end-of-sprint probe
  sweep a standing per-sprint check, so the next such regression is caught by
  the sprint that ships it.
