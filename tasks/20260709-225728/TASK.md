# AI fire discipline: turret lead, burst cadence, range gating

- STATUS: CLOSED
- PRIORITY: 74
- TAGS: v0.4.0,ai,spike,turret


Spike: tasks/20260709-225508/SPIKE.md (wave 1)

Goal: make AI gunnery honest instead of a continuous aligned-spray. Feed the
target root's LinearVelocity into TurretSectionTargetVelocity so AI turrets
actually lead via lead_intercept_point (the AI-side sibling of
20260709-173700); gate fire on an effective-range envelope, not just muzzle
alignment; add burst cadence (fire-window/cooldown timers) instead of holding
the trigger while aligned.

Depends on: 20260709-225726 (skeleton), pairs with 20260709-225727 (AITarget).

## Steps

- [x] Velocity feed: the AI `update_turret_target_input` also writes
      `TurretSectionTargetVelocity` from the target root's `LinearVelocity`
      (ZERO when absent or no target), the AI-side sibling of the player
      feed from 20260709-173700 - `lead_intercept_point` then computes a
      real lead for AI turrets.
- [x] Aim-point fire gate: `on_projectile_input` checks muzzle alignment
      against the turret's `TurretSectionAimPoint` (the LEADED point the
      turret actually steers to), falling back to the target anchor when
      unset - otherwise a correctly-leading turret never "aligns" with the
      raw anchor and holds fire forever against crossing targets.
- [x] Range gate: no fire beyond the turret's effective range, derived per
      turret from its config (`muzzle_speed * projectile_lifetime`, scaled
      by a tunable AI_FIRE_RANGE_FACTOR margin) - bullets that die in
      flight are noise, not discipline.
- [x] Burst cadence: `AIFireCadence` component (required by the AI marker,
      free-running fire-window/hold cycle with tunable
      AI_BURST_FIRE_SECS / AI_BURST_HOLD_SECS), ticked by a new
      `update_fire_cadence` system in the AI chain; `on_projectile_input`
      holds fire outside the window.
- [x] Tests: velocity feed (moving target -> its velocity, no target ->
      ZERO); aim-point gate discriminator (muzzle on the lead point but off
      the raw anchor still fires); range gate (aligned but beyond effective
      range holds); cadence (input alternates over simulated time).
- [x] Verify: cargo fmt, cargo check --workspace, ai:: module tests (skip
      full local suite per user instruction; report skips honestly).

## Notes

- Relevant files: crates/nova_gameplay/src/input/ai.rs;
  sections/turret_section.rs (TurretSectionTargetVelocity,
  TurretSectionAimPoint, TurretSectionConfigHelper - all pub).
- The lead SOLVE is shooter-frame-correct already (20260709-211701); this
  task only feeds it and gates on its output.
- Friendly-fire hold (don't shoot through an allied ship) considered and
  deferred: needs a raycast/occlusion check; today only one AI archetype
  exists and allies rarely cross the line of fire. Revisit with the
  standoff-flight task (225729) if formation fights make it visible.

## Resolution (20260710)

Shipped all four gates: the target-velocity feed into
TurretSectionTargetVelocity (AI turrets now lead), the aim-point alignment
gate (fires on the LEADED point the turret steers to, with anchor
fallback - the discriminator test puts the anchor 22 degrees off the cone
and the lead dead-ahead), the per-turret effective-range gate
(muzzle_speed * lifetime * 0.9), and AIFireCadence (required by the
marker, free-running 1.5 s fire / 0.8 s hold cycle - free volley
staggering between ships since phases drift apart). 5 new tests; full
crate suite 213/213 green this once, fmt + check clean. Skipped per user
instruction: clippy.

Bug caught during work: the first aim-point discriminator geometry
(30,0,-100) was still INSIDE the 0.95 alignment cone (16.7 deg vs 18.2),
so the test could not fail on the old code; recomputed the cosine and
moved the anchor to 22 degrees off. Lesson: for cone tests, compute the
cosine by hand before trusting the scenario discriminates.

Deferred per Notes: friendly-fire hold (needs occlusion; revisit with
standoff flight 225729 if formation fights make it visible).
