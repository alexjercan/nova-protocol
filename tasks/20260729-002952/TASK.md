# Investigate probe FPS regression from July 29 runs

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.9.0,bug,perf,probe

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

- [ ] Record the current evidence in NOTES.md from the JSON/CSV/trace data, not
      from the HTML reports: current/before git SHAs, affected examples,
      `frametime.csv` deltas, `checks.json` verdicts, and top trace deltas for
      render/extract/font loading.
- [ ] Isolate the regression range with targeted probe runs or a small bisect
      across likely commits: at minimum `87600482` (Iosevka .ttc), `14aea590`
      (NOVA OS render-to-texture CRT), `dec4ee9f` (nova_os extraction + loader),
      and current HEAD. Keep the run artifacts reusable; prefer the commit-keyed
      layout from task 20260729-003352 if it is available.
- [ ] Coordinate with task 20260729-000956: treat its preload/static-asset work
      as a likely partial fix for the repeated font-load finding, but verify
      whether render extraction remains elevated after that task lands.
- [ ] Run before/after probe checks and update NOTES.md
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
