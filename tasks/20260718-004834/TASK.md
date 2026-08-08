# scatter_density preset lever: wire authored scatter through scaled_count, or stop advertising it

- STATUS: CLOSED
- PRIORITY: 36
- TAGS: v0.7.0, performance, settings, scenario

## Goal

The frame-time baseline (tasks/20260716-123551) found the `scatter_density`
graphics-preset lever is dead for the shipped scenes: it only thins *procedural*
scatter via `GraphicsBudget::scaled_count`, but `asteroid_field` /
`broadside` / `shakedown_run` place asteroids with authored
`SpawnScenarioObject` actions that never call `scaled_count`, so the multiplier
never fires. A preset lever that does nothing is worse than no lever. Pick one of
two resolutions and make the lever honest.

## Decision (2026-07-18): REMOVE the lever entirely

Chosen direction is a stronger form of option (b): do not keep `scatter_density`
even for procedural scatter. Asteroids/rocks/debris are gameplay content, not a
graphics-budget knob, so no preset tier should thin them. `particles` remains the
sole `GraphicsBudget` lever (that one is a validated combat cost and stays).

The companion fraction-tuning task (20260718-004809) is closed WON'T DO as a
consequence: with `scatter_density` gone, `for_quality` carries no fractions left
to tune.

## Steps

- Remove the `scatter_density` field from `GraphicsBudget` (`crates/nova_gameplay/src/settings.rs`).
- Remove `GraphicsBudget::scaled_count` and its per-tier values in `for_quality`;
  `for_quality` keeps only the `particles` bool per tier.
- Update the `ScatterObjects` action (`crates/nova_scenario/src/actions.rs`) so it
  spawns the authored `count` directly instead of `graphics_budget().scaled_count(count)`.
  Drop the now-dead `GraphicsBudget` plumbing there if nothing else uses it
  (check `world.rs` graphics_budget carry-through - keep whatever `particles` needs).
- Remove/relabel any settings-UI copy that advertises scatter/object density as a
  quality lever, so the preset only claims what it delivers (particles).
- Update the tests in `settings.rs` and `actions.rs` that assert on
  `scatter_density` / `scaled_count` / thinned-field behaviour.
- Keep `particles` behaviour (Low = no particle spawns) exactly as-is.

## Notes

- Baseline report, "Graphics-preset fractions - HOLD": tasks/20260716-123551/frametime-baseline-report.md
- `crates/nova_gameplay/src/settings.rs` (`GraphicsBudget`, `scaled_count`).
- Correction to the premise above: the "never fires for shipped scenes" claim is
  only true for `shakedown_run`. `asteroid_field` and `broadside` DO thin their
  fields today via a real `ScatterObjects((count: 20/24))` block. Removing the
  lever restores full asteroid/rock counts on Low in those two scenes - which is
  the intended gameplay behaviour.
