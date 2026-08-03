# Fix the hud_range example smoke: the scripted run never reaches its last beat

- PRIORITY: 90
- TAGS: v0.10.0, bug, examples, testing
- KIND: TASK
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -

## Story

`tests/examples_smoke.rs::ui_reach_playing_without_panic` fails on CI. The
`hud_range` example's backstop fires:

```
thread 'main' panicked at examples/ui/hud_range.rs:340:9:
hud range: the scripted run never finished (ring=true lock=true goto=true drop=false)
```

Pre-existing and NOT caused by the `nova_autopilot` migration: the identical
failure is in run `30768496842` (2026-08-02 21:42), before `8cf34ebf` landed.
It also reproduces on run `30805870861` (2026-08-03).

## Diagnosis (starting point, not confirmed)

Two clocks disagree:

- The script timeline `t` is relative to entering `Playing`, deliberately, so
  a slow load shifts the beats instead of truncating them.
- The backstop at line 338 uses `elapsed`, the autopilot-window clock, and
  fires at `elapsed > 7.5`.

The last beat needs `t > 4.8`. CI logs show the kill beat (`t > 4.4`) firing
and `scenario_elapsed` reaching only ~4.76 when the run ends, so on a loaded
runner the load cost eats the difference between the two clocks and the
window closes before the final beat runs. The assertions themselves never got
a chance to fail - the beat never ran.

## Steps

- [ ] Reproduce under Xvfb with an artificially slowed load, so the failure is
      falsified rather than argued from logs.
- [ ] Decide the fix: extend the autopilot window, base the backstop on the
      same `Playing`-relative clock the beats use, or both. Whatever lands must
      keep the backstop's original purpose - a run that never reaches `Playing`
      still has to fail loudly, not pass vacuously.
- [ ] Check the sibling examples for the same two-clock split; the pattern is
      copied around the fleet.

## Done Means

- The example smoke suite passes.
  (cmd: `nix develop --command cargo test --test examples_smoke`)
- A run that never reaches `Playing` still fails loudly - proven by a
  deliberate falsification, recorded in the retro.

## Notes

- Found while closing epic `20260802-120019`; it kept that epic's fourth DoD
  command red. The epic closed anyway because the failure predates its work.
