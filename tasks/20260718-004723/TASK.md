# Web render-scale / resolution lever for the graphics preset (aim the over-budget web target)

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.7.0, performance, web, settings

## Goal

The v0.7.0 frame-time baseline (tasks/20260716-123551) established that web is
the only over-budget target: 26-29 fps at rest and 24 fps in combat on the same
GPU that renders ~48-52 fps natively, and it is fill/overhead-bound with almost
no headroom. The report's strongest concrete direction is a render-scale /
resolution lever on the Low graphics preset, aimed specifically at web: on a
fill-bound path, dropping internal render resolution buys more than the existing
particle/scatter toggles.

Add a render-scale lever to `GraphicsBudget` / `GraphicsQuality` and wire it into
the render path, then measure the win on web with the existing harness so the
change clears the same measure-first gate the baseline used.

## Steps

- Add a `render_scale` (or resolution-scale) fraction to `GraphicsBudget`
  (`crates/nova_gameplay/src/settings.rs`), defaulted per quality tier.
- Wire it into the render path (internal render-target resolution / camera
  viewport scaling), upscaling to the window for presentation.
- Verify it takes effect on both native and the web/WebGPU build.
- Re-run `scripts/perf-web.sh` for the three scenes (+ `COMBAT=1` on broadside)
  at High vs Low and record the before/after frame-time delta in this task
  folder. Justify the lever by the measured web win, not the plausible story.
- If the win is real, set the Low/Medium tier fractions from the numbers
  (coordinate with the GraphicsBudget-fraction tuning task).

## Notes

- Baseline + rigs + reproduce commands: tasks/20260716-123551/frametime-baseline-report.md
- Harness lives in `crates/nova_perf`; web capture is `scripts/perf-web.sh`.
- Related: GraphicsBudget-fraction tuning and scatter_density follow-ups from
  the same baseline.
