# Review: Starter New Game scenario (Shakedown Run)

- TASK: 20260711-180506
- BRANCH: shakedown-run (commit 9f3a6c2 vs master)

## Round 1

- VERDICT: REQUEST_CHANGES (fresh-context agent review)

- [x] R1.1 (MAJOR) crates/nova_scenario/src/world.rs:20 - the OnUpdate
  pulse keeps the event queue warm every frame, so state_to_world's
  unconditional GameObjectives clear+extend flags the resource changed
  every frame and the objectives panel (gated on resource_changed)
  despawns/respawns its text lines per frame for the whole session.
  Fix: write-on-diff.
  - Response: fixed - state_to_world compares (id, message) pairs and
    writes only on a real change; regression test
    world::tests::unchanged_objectives_do_not_flag_the_resource counts
    change-detections across repeated syncs (fails against the old blind
    write: 5+ rebuilds vs <= 2).
- [x] R1.2 (MAJOR) crates/nova_assets/src/scenario/shakedown.rs - the
  4.0-4.55 geometric-factor band was folklore from one observed comment;
  the codebase's own sanity bound admits up to 7x, and a factor > ~5.27
  parks the ORBIT ring outside the 160u gate: beat 4 soft-locks with all
  tests green. Fix: pin the real bound with a seed sweep and share the
  const.
  - Response: fixed - and the sweep proved it worse than folklore: 256
    seeds measured [3.70, 5.64], so the softlock was REAL on many seeds.
    nova_scenario now exports ASTEROID_GEOMETRIC_FACTOR_MIN/MAX
    (3.5/6.0, margin around the measured range) pinned by
    geometric_factor_bounds_hold_across_seeds (256-seed sweep of the
    production mesh path); the shakedown gate grew to 200u (widest ring
    181.5u), beacon 3 moved out to ~260u, and the beat-4 geometry test
    now cites the exported consts instead of local numbers.
- [x] R1.3 (MINOR) two unreachable-but-unpinned geometric invariants
  (crate inside the orbit gate when beat 4 starts; crate sensor
  overlapping beacon 2's trigger when the pickups arm).
  - Response: fixed - both pinned as assertions in
    beat4_geometry_holds_across_the_derived_radius_range.
- [x] R1.4 (NIT) the two walk tests duplicated ~45 lines of App/handler
  setup.
  - Response: fixed - shared scripted_app()/boot()/pulse()/enter()/
    destroy() helpers.

Reviewer verified clean in round 1: OnUpdate "dead config" claim (no
prior fire site); tally ordering safe (synchronous per-action mutation,
exact-match count filters); complete+re-add in-order within one action
list; death-restart teardown; walk-test rigor and honesty of TASK.md
(including the non-ticked visual playtest); no non-ASCII; turret math.
Tests observed: all passing, check + fmt clean.

## Round 2

- VERDICT: APPROVE

All four findings verified fixed in 63dc629. R1.1's regression test was
mutation-tested (reverting to the blind clear+extend fails it); R1.2's
margins independently re-derived (beacon 3 at 259.9u: inside half-SOI
280u, outside gate+30; widest ring 181.5u < 200u gate), with the honest
caveat that 256 samples bound the seed space statistically, mitigated by
the 6% margin over the observed max and the const doc forbidding widening
without sweep numbers. Checks clean, no non-ASCII.
