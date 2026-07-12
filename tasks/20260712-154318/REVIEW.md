# Review: Lock reticle on beacons sizes to the trigger sensor

- TASK: 20260712-154318
- BRANCH: beacon-reticle-size

## Round 1

- VERDICT: APPROVE

One-file semantic fix reviewed in-session with the re-derive rule:

- Re-derived the consumer set: target_world_aabb has exactly one caller
  (indicator_size's ApparentSize arm), whose consumers are the torpedo
  lock reticle and the candidate brackets - both anchor ships/asteroids
  whose visible bodies are solid colliders; no anchored entity NEEDS a
  sensor extent (grep-verified: sensors in the workspace are triggers,
  pickup volumes, bullet/muzzle probes - all invisible). The exclusion
  is a semantics correction, not a tuning knob.
- The locked-beacon case is pinned by test with a strip-the-Sensor
  delivery guard (the query shape, not entity absence, excludes it);
  the mixed-subtree case pins that solid parts still union.
- The WorldRadius mode from 20260712-152340's sibling fix is untouched
  and its huge-sensor-AABB test still passes - the two fixes compose.
- Checks: fmt clean, cargo check --workspace --all-targets clean,
  screen_indicator suite 24 passed.

No findings.
