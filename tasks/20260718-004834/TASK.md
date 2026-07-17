# scatter_density preset lever: wire authored scatter through scaled_count, or stop advertising it

- STATUS: OPEN
- PRIORITY: 36
- TAGS: v0.7.0,performance,settings,scenario


## Goal

The frame-time baseline (tasks/20260716-123551) found the `scatter_density`
graphics-preset lever is dead for the shipped scenes: it only thins *procedural*
scatter via `GraphicsBudget::scaled_count`, but `asteroid_field` /
`broadside` / `shakedown_run` place asteroids with authored
`SpawnScenarioObject` actions that never call `scaled_count`, so the multiplier
never fires. A preset lever that does nothing is worse than no lever. Pick one of
two resolutions and make the lever honest.

## Steps

- Decide the direction:
  - (a) Wire authored scatter through `scaled_count` so the density fraction
    thins hand-placed asteroid fields too (probabilistic skip / count scaling in
    the `SpawnScenarioObject` path), OR
  - (b) Drop `scatter_density` as an advertised preset lever and document that
    it only applies to procedural scatter.
- If (a): implement, then verify the Low preset visibly thins the shipped fields
  and re-measure with the baseline harness (native + web) to confirm a real win.
- If (b): remove/relabel the lever in `GraphicsBudget` and the settings UI so the
  preset only claims what it delivers.
- Update the GraphicsBudget-fraction tuning task with the outcome.

## Notes

- Baseline report, "Graphics-preset fractions - HOLD": tasks/20260716-123551/frametime-baseline-report.md
- `crates/nova_gameplay/src/settings.rs` (`GraphicsBudget`, `scaled_count`).
